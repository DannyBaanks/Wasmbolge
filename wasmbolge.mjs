// Minimal host wrapper for the raw WASM ABI.
export async function loadWasmbolge(urlOrBytes) {
  const source = typeof urlOrBytes === 'string'
    ? await WebAssembly.instantiateStreaming(fetch(urlOrBytes), {})
    : await WebAssembly.instantiate(urlOrBytes, {});
  return new Wasmbolge(source.instance.exports);
}

export class Wasmbolge {
  constructor(exports) {
    this.e = exports;
    this.memory = exports.memory;
    if (!this.memory || !exports.wasmbolge_alloc || !exports.wasmbolge_run) {
      throw new Error('wasmbolge ABI exports are incomplete');
    }
  }

  run(program, input = new Uint8Array(), maxSteps = 1000000) {
    const source = toBytes(program);
    const inputBytes = toBytes(input);
    const outputCap = 65536;
    const jsonCap = 512;
    const pp = this.e.wasmbolge_alloc(source.length);
    const ip = this.e.wasmbolge_alloc(inputBytes.length || 1);
    const op = this.e.wasmbolge_alloc(outputCap);
    const jp = this.e.wasmbolge_alloc(jsonCap);
    try {
      new Uint8Array(this.memory.buffer, pp, source.length).set(source);
      if (inputBytes.length) new Uint8Array(this.memory.buffer, ip, inputBytes.length).set(inputBytes);
      const packed = this.e.wasmbolge_run(pp, source.length, ip, inputBytes.length,
        BigInt(maxSteps), op, outputCap, jp, jsonCap);
      if (packed < 0) throw new Error('invalid Malbolge program');
      const value = BigInt(packed);
      const jsonLen = Number((value >> 32n) & 0xffffffffn);
      const outputLen = Number(value & 0xffffffffn);
      const output = new Uint8Array(this.memory.buffer, op, outputLen).slice();
      const json = new TextDecoder().decode(new Uint8Array(this.memory.buffer, jp, jsonLen));
      return { output, ...JSON.parse(json) };
    } finally {
      for (const [ptr, size] of [[pp, source.length], [ip, inputBytes.length || 1], [op, outputCap], [jp, jsonCap]]) {
        if (this.e.wasmbolge_dealloc) this.e.wasmbolge_dealloc(ptr, size);
      }
    }
  }
}

function toBytes(value) {
  if (value instanceof Uint8Array) return value;
  if (typeof value === 'string') return new TextEncoder().encode(value);
  return Uint8Array.from(value);
}
