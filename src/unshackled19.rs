//! Positional Unshackled 3^19 codec and execution machine.
//!
//! This is deliberately a separate machine from Classic Malbolge.  The
//! runtime follows the local bolge19 reference: a 3^19 address space, the
//! positional decoder, lazy 65536-cell chain pages, and direct indexing into
//! the 135-character encryption table.

use std::collections::HashMap;

pub const END: u64 = 1_162_261_467;
pub const THIRD: u64 = 387_420_489;
const PAGE: usize = 65_536;
const XLAT: &[u8] = b"SOMEBODY MAKE ME FEEL ALIVE[hj9>,5z]&gqtyfr$(we4{WP)H-Zn,[%\\3dL+Q;>U!pJS72FhOA1CB6v^=I_0/8|jsb9m<.TVac`uY*MK'X~xDl}REokN:#?G\"i@AND SHATTER ME";
const CRZ2: [u32; 144] = [
    4, 3, 3, 1, 0, 0, 1, 0, 0, 9, 9, 9, 9, 9, 9, 9,
    4, 3, 5, 1, 0, 2, 1, 0, 2, 9, 9, 9, 9, 9, 9, 9,
    5, 5, 4, 2, 2, 1, 2, 2, 1, 9, 9, 9, 9, 9, 9, 9,
    4, 3, 3, 1, 0, 0, 7, 6, 6, 9, 9, 9, 9, 9, 9, 9,
    4, 3, 5, 1, 0, 2, 7, 6, 8, 9, 9, 9, 9, 9, 9, 9,
    5, 5, 4, 2, 2, 1, 8, 8, 7, 9, 9, 9, 9, 9, 9, 9,
    7, 6, 6, 7, 6, 6, 4, 3, 3, 9, 9, 9, 9, 9, 9, 9,
    7, 6, 8, 7, 6, 8, 4, 3, 5, 9, 9, 9, 9, 9, 9, 9,
    8, 8, 7, 8, 8, 7, 5, 5, 4, 9, 9, 9, 9, 9, 9, 9,
];

fn crazy(a0: u32, d0: u32) -> u32 {
    let mut a = a0;
    let mut d = d0;
    let mut result = 0u32;
    let mut place = 1u32;
    for _ in 0..9 {
        let digit = CRZ2[(a % 9 + 16 * (d % 9)) as usize];
        result = result.wrapping_add(place.wrapping_mul(digit));
        a /= 9;
        d /= 9;
        place = place.wrapping_mul(9);
    }
    let table = [[1u32, 0, 0], [1, 0, 2], [2, 2, 1]];
    result.wrapping_add(place.wrapping_mul(table[(d % 3) as usize][(a % 3) as usize]))
}

fn rotate(value: u32) -> u32 {
    let b = value % THIRD as u32;
    value - b + b / 3 + (b % 3) * (THIRD as u32 / 3)
}

pub fn offset(pos: u64) -> u64 {
    assert!(pos < END);
    let a1 = 94 - ((END - 1) / 6 - 29_524) % 94;
    let a2 = 94 - ((END - 1) / 3 - 59_048) % 94;
    match pos / THIRD {
        0 => 0,
        1 => (a1 + 94 - THIRD % 94) % 94 + 94,
        _ => a2 - (2 * THIRD) % 94 + 94,
    }
}

pub fn decode(ch: u8, pos: u64) -> u8 { ((ch as u64 + pos + offset(pos)) % 94) as u8 }

pub fn encode(op: u8, pos: u64) -> u8 {
    assert!(op < 94 && pos < END);
    let mut r = (op as u64 + 94 - (pos + offset(pos)) % 94) % 94;
    if r < 33 { r += 94; }
    r as u8
}

#[derive(Debug, PartialEq, Eq)]
enum Halt {
    VInstruction,
    StepsExhausted,
}

struct Machine {
    image: Vec<u32>,
    zone_start: usize,
    zone: Vec<u32>,
    pattern: [u32; 6],
    overlay: HashMap<usize, u32>,
    a: u32,
    c: usize,
    d: usize,
    steps: u64,
    input: Vec<u8>,
    input_pos: usize,
    output: Vec<u8>,
}

impl Machine {
    fn new(cells: &[u8], input: &[u8]) -> Result<Self, ()> {
        if cells.is_empty() || cells.len() as u64 > END {
            return Err(());
        }
        let image: Vec<u32> = cells.iter().map(|&cell| cell as u32).collect();
        let zone_start = image.len().div_ceil(PAGE) * PAGE;
        let mut p2 = image.get(image.len().saturating_sub(2)).copied().unwrap_or(0);
        let mut p1 = image.last().copied().unwrap_or(0);
        let mut zone = Vec::with_capacity(zone_start - image.len());
        for _ in image.len()..zone_start {
            let next = crazy(p1, p2);
            zone.push(next);
            p2 = p1;
            p1 = next;
        }
        let mut pattern = [0u32; 6];
        for slot in &mut pattern {
            let next = crazy(p1, p2);
            *slot = next;
            p2 = p1;
            p1 = next;
        }
        Ok(Self {
            image,
            zone_start,
            zone,
            pattern,
            overlay: HashMap::new(),
            a: 0,
            c: 0,
            d: 0,
            steps: 0,
            input: input.to_vec(),
            input_pos: 0,
            output: Vec::new(),
        })
    }

    fn get(&self, index: usize) -> u32 {
        if index < self.image.len() {
            return self.image[index];
        }
        if let Some(&value) = self.overlay.get(&index) {
            return value;
        }
        if index < self.zone_start {
            return self.zone[index - self.image.len()];
        }
        let page = index & !(PAGE - 1);
        let phase = ((page % THIRD as usize) + (index - page)) % 6;
        self.pattern[phase]
    }

    fn set(&mut self, index: usize, value: u32) {
        if index < self.image.len() {
            self.image[index] = value;
        } else {
            self.overlay.insert(index, value);
        }
    }

    fn run(&mut self, max_steps: u64) -> Halt {
        while self.steps < max_steps {
            let region_offset = offset(self.c as u64);
            let opcode = ((self.c as u64 + self.get(self.c) as u64 + region_offset) % 94) as u8;
            match opcode {
                4 => self.c = self.get(self.d) as usize % END as usize,
                5 => self.output.push((self.a & 255) as u8),
                23 => {
                    self.a = if self.input_pos < self.input.len() {
                        let value = self.input[self.input_pos];
                        self.input_pos += 1;
                        value as u32
                    } else {
                        (END - 1) as u32
                    };
                }
                39 => {
                    let value = rotate(self.get(self.d));
                    self.set(self.d, value);
                    self.a = value;
                }
                40 => self.d = self.get(self.d) as usize % END as usize,
                62 => {
                    let value = crazy(self.a, self.get(self.d));
                    self.set(self.d, value);
                    self.a = value;
                }
                81 => return Halt::VInstruction,
                _ => {}
            }
            let current = self.get(self.c);
            if (current as usize) < XLAT.len() {
                self.set(self.c, XLAT[current as usize] as u32);
            }
            self.c = (self.c + 1) % END as usize;
            self.d = (self.d + 1) % END as usize;
            self.steps += 1;
        }
        Halt::StepsExhausted
    }
}

fn load(raw: &[u8]) -> Result<Vec<u8>, ()> {
    let cells: Vec<u8> = raw.iter().copied().filter(|b| !b" \t\r\n".contains(b)).collect();
    if cells.is_empty() || cells.iter().any(|&b| !(33..=126).contains(&b)) {
        Err(())
    } else {
        Ok(cells)
    }
}

/// Separate ABI entry point for Unshackled. `wasmbolge_run` remains Classic.
#[no_mangle]
pub unsafe extern "C" fn wasmbolge_run_unshackled19(
    program_ptr: *const u8, program_len: usize, input_ptr: *const u8, input_len: usize,
    max_steps: u64, output_ptr: *mut u8, output_cap: usize,
    json_ptr: *mut u8, json_cap: usize,
) -> i64 {
    let program = unsafe { std::slice::from_raw_parts(program_ptr, program_len) };
    let input = unsafe { std::slice::from_raw_parts(input_ptr, input_len) };
    let cells = match load(program) { Ok(cells) => cells, Err(()) => return -1 };
    let mut machine = match Machine::new(&cells, input) { Ok(machine) => machine, Err(()) => return -1 };
    let halt = machine.run(max_steps);
    let output_len = machine.output.len().min(output_cap);
    if output_len > 0 {
        unsafe { std::ptr::copy_nonoverlapping(machine.output.as_ptr(), output_ptr, output_len); }
    }
    let reason = match halt { Halt::VInstruction => "VInstruction", Halt::StepsExhausted => "StepsExhausted" };
    let status = if halt == Halt::StepsExhausted { "TIMEOUT" } else { "HALTED" };
    let json = format!(r#"{{"final":{{"a":{},"c":{},"d":{}}},"halt_reason":"{}","output_len":{},"status":"{}","steps":{}}}"#, machine.a, machine.c, machine.d, reason, machine.output.len(), status, machine.steps);
    let json_len = json.len().min(json_cap);
    if json_len > 0 {
        unsafe { std::ptr::copy_nonoverlapping(json.as_ptr(), json_ptr, json_len); }
    }
    ((json_len as i64) << 32) | output_len as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn round_trip_boundaries() {
        for &p in &[0, 1, THIRD - 1, THIRD, THIRD + 1, END - 1] {
            for op in [4u8, 5, 23, 39, 40, 62, 68, 81] {
                assert_eq!(decode(encode(op, p), p), op);
            }
        }
    }
    #[test] fn manual_nop() { assert_eq!(encode(68, 17), b'3'); }

    #[test]
    fn unshackled_machine_halts_with_positional_v() {
        let mut machine = Machine::new(&[encode(81, 0)], &[]).unwrap();
        assert_eq!(machine.run(10), Halt::VInstruction);
        assert_eq!(machine.steps, 0);
        assert_eq!((machine.a, machine.c, machine.d), (0, 0, 0));
    }

    #[test]
    fn unshackled_machine_uses_direct_encryption_table() {
        let nop = encode(68, 0);
        let mut machine = Machine::new(&[nop, encode(81, 1)], &[]).unwrap();
        assert_eq!(machine.run(10), Halt::VInstruction);
        assert_eq!(machine.image[0], XLAT[nop as usize] as u32);
        assert_eq!(machine.steps, 1);
    }
}
