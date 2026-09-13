#![deny(unsafe_op_in_unsafe_fn)]

const MEM_SIZE: usize = 59049;
const LAST: usize = MEM_SIZE - 1;
const BLOCK: usize = 243;
const POW9: u32 = 19683;
const ENC: &[u8] = b"5z]&gqtyfr$(we4{WP)H-Zn,[%\\3dL+Q;>U!pJS72FhOA1CB6v^=I_0/8|jsb9m<.TVac`uY*MK'X~xDl}REokN:#?G\"i@";

pub mod unshackled19;

fn crazy5(a: u32, b: u32) -> u32 {
    let table = [[1u32, 0, 0], [1, 0, 2], [2, 2, 1]];
    let mut aa = a;
    let mut bb = b;
    let mut place = 1;
    let mut result = 0;
    for _ in 0..5 {
        result += table[(bb % 3) as usize][(aa % 3) as usize] * place;
        aa /= 3;
        bb /= 3;
        place *= 3;
    }
    result
}

fn crazy(a: u32, b: u32) -> u32 {
    crazy5(a % BLOCK as u32, b % BLOCK as u32)
        + BLOCK as u32 * crazy5(a / BLOCK as u32, b / BLOCK as u32)
}

fn rotate(n: u32) -> u32 { POW9 * (n % 3) + n / 3 }

#[derive(Debug, PartialEq, Eq)]
enum Halt { VInstruction, InvalidCell, Eof, StepsExhausted }

struct Machine {
    a: u32,
    c: usize,
    d: usize,
    steps: u64,
    fill_start: usize,
    chain_until: usize,
    overlay: Vec<u32>,
    present: Vec<bool>,
    chain: Vec<u32>,
    input: Vec<u8>,
    input_pos: usize,
    output: Vec<u8>,
}

impl Machine {
    fn new(cells: &[u8], input: &[u8]) -> Result<Self, u8> {
        if cells.is_empty() { return Err(0); }
        let fill_start = cells.len().max(2).min(MEM_SIZE);
        let mut m = Self {
            a: 0, c: 0, d: 0, steps: 0, fill_start, chain_until: fill_start,
            overlay: vec![0; MEM_SIZE], present: vec![false; MEM_SIZE],
            chain: vec![0; MEM_SIZE], input: input.to_vec(), input_pos: 0,
            output: Vec::new(),
        };
        for (i, &cell) in cells.iter().take(MEM_SIZE).enumerate() {
            m.overlay[i] = cell as u32;
            m.present[i] = true;
        }
        Ok(m)
    }

    fn get(&mut self, x: usize) -> u32 {
        if self.present[x] { return self.overlay[x]; }
        if x < self.fill_start { return 0; }
        if x >= self.chain_until { self.fill(x); }
        self.chain[x]
    }

    fn fill(&mut self, x: usize) {
        let block_end = ((x / BLOCK) + 1) * BLOCK;
        let tail_end = ((self.fill_start / BLOCK) + 1) * BLOCK;
        let (mut p1, mut p2, mut i) = if self.chain_until == self.fill_start {
            let p1 = if self.fill_start > 0 && self.present[self.fill_start - 1] { self.overlay[self.fill_start - 1] } else { 0 };
            let p2 = if self.fill_start > 1 && self.present[self.fill_start - 2] { self.overlay[self.fill_start - 2] } else { 0 };
            (p1, p2, self.fill_start)
        } else {
            (self.chain[self.chain_until - 1], self.chain[self.chain_until - 2], self.chain_until)
        };
        while i < block_end && i < MEM_SIZE {
            let next = if i == tail_end {
                crazy(self.get(i - 1), self.get(i - 2))
            } else if i == tail_end + 1 {
                crazy(p1, self.get(i - 2))
            } else { crazy(p1, p2) };
            self.chain[i] = next;
            p2 = p1;
            p1 = next;
            i += 1;
        }
        self.chain_until = block_end.min(MEM_SIZE);
    }

    fn set(&mut self, x: usize, value: u32) {
        self.present[x] = true;
        self.overlay[x] = value;
    }

    fn run(&mut self, max_steps: u64) -> Halt {
        while self.steps < max_steps {
            self.steps += 1;
            let ins = self.get(self.c);
            if !(33..=126).contains(&ins) { return Halt::InvalidCell; }
            match ((ins as usize + self.c) % 94) as u32 {
                4 => self.c = self.get(self.d) as usize,
                5 => self.output.push((self.a % 256) as u8),
                23 => {
                    if self.input_pos >= self.input.len() { return Halt::Eof; }
                    self.a = self.input[self.input_pos] as u32;
                    self.input_pos += 1;
                }
                39 => { let r = rotate(self.get(self.d)); self.set(self.d, r); self.a = r; }
                40 => self.d = self.get(self.d) as usize,
                62 => { let r = crazy(self.a, self.get(self.d)); self.set(self.d, r); self.a = r; }
                81 => return Halt::VInstruction,
                _ => {}
            }
            let current = self.get(self.c);
            if (33..=126).contains(&current) { self.set(self.c, ENC[(current - 33) as usize] as u32); }
            self.c = if self.c == LAST { 0 } else { self.c + 1 };
            self.d = if self.d == LAST { 0 } else { self.d + 1 };
        }
        Halt::StepsExhausted
    }
}

fn load(raw: &[u8]) -> Result<Vec<u8>, u8> {
    let mut cells = Vec::new();
    for &b in raw {
        if matches!(b, b' ' | b'\t' | b'\r' | b'\n') { continue; }
        if !(33..=126).contains(&b) || cells.len() >= MEM_SIZE { return Err(b); }
        cells.push(b);
    }
    if cells.is_empty() { Err(0) } else { Ok(cells) }
}

#[no_mangle]
pub extern "C" fn wasmbolge_alloc(size: usize) -> *mut u8 {
    let mut buffer = Vec::<u8>::with_capacity(size);
    let pointer = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    pointer
}

#[no_mangle]
pub unsafe extern "C" fn wasmbolge_dealloc(pointer: *mut u8, size: usize) {
    if !pointer.is_null() { unsafe { drop(Vec::from_raw_parts(pointer, 0, size)); } }
}

/// Runs a program. Buffers are caller-owned linear-memory regions.
/// Returns `(json_len << 32) | output_len`; returns -1 for invalid input.
#[no_mangle]
pub unsafe extern "C" fn wasmbolge_run(
    program_ptr: *const u8, program_len: usize, input_ptr: *const u8, input_len: usize,
    max_steps: u64, output_ptr: *mut u8, output_cap: usize,
    json_ptr: *mut u8, json_cap: usize,
) -> i64 {
    let program = unsafe { std::slice::from_raw_parts(program_ptr, program_len) };
    let input = unsafe { std::slice::from_raw_parts(input_ptr, input_len) };
    let cells = match load(program) { Ok(c) => c, Err(_) => return -1 };
    let mut m = match Machine::new(&cells, input) { Ok(m) => m, Err(_) => return -1 };
    let halt = m.run(max_steps);
    let output_len = m.output.len().min(output_cap);
    unsafe { std::ptr::copy_nonoverlapping(m.output.as_ptr(), output_ptr, output_len); }
    let status = if halt == Halt::StepsExhausted { "TIMEOUT" } else { "HALTED" };
    let reason = match halt { Halt::VInstruction => "VInstruction", Halt::InvalidCell => "InvalidCell", Halt::Eof => "Eof", Halt::StepsExhausted => "StepsExhausted" };
    let json = format!(r#"{{"final":{{"a":{},"c":{},"d":{}}},"halt_reason":"{}","output_len":{},"status":"{}","steps":{}}}"#, m.a, m.c, m.d, reason, m.output.len(), status, m.steps);
    let bytes = json.as_bytes();
    let json_len = bytes.len().min(json_cap);
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), json_ptr, json_len); }
    ((json_len as i64) << 32) | output_len as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    const CLASSIC: &[u8] = b"(=<`#9]~6ZY327Uv4-QsqpMn&+Ij\"'E%e{Ab~w=_:]Kw%o44Uqp0/Q?xNvL:`H%c#DD2^WV>gY;dts76qKJImZkj";

    #[test]
    fn crazy_and_rotate_are_canonical() {
        assert_eq!(rotate(1), 19683);
        assert_eq!(crazy(19700, 0), 9828);
    }

    #[test]
    fn empty_program_is_rejected() { assert!(load(b" \r\n\t").is_err()); }

    #[test]
    fn classic_gate_matches() {
        let mut m = Machine::new(CLASSIC, b"").unwrap();
        assert_eq!(m.run(100), Halt::VInstruction);
        assert_eq!(m.steps, 48);
        assert_eq!((m.a, m.c, m.d), (19758, 85, 63));
        assert_eq!(m.output, b"Hello, world.");
    }
}
