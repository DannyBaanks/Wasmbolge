# Wasmbolge

Experimental, dependency-free Rust core for a future WebAssembly Malbolge
backend. The exported ABI is `wasmbolge_run`: caller-owned linear-memory
buffers in, raw output plus compact JSON report out. `wasmbolge.mjs` provides
a small browser/Node host wrapper.

Current scope: Classic execution core, native Rust tests, and a verified
`wasm32-unknown-unknown` release build. Browser/Node execution remains
verified in Node 22.23.1; browser execution remains `NOT_DEMONSTRATED`.

Build and Node smoke test:

```powershell
cargo test
cargo build --target wasm32-unknown-unknown --release
node --input-type=module -e "import fs from 'node:fs'; import {loadWasmbolge} from './wasmbolge.mjs'; const w=await loadWasmbolge(fs.readFileSync('target/wasm32-unknown-unknown/release/wasmbolge.wasm')); const r=w.run(fs.readFileSync('fixture.mal'),new Uint8Array(),100); console.log(JSON.stringify({...r,output:Array.from(r.output)}));"
```

Observed result: `output_len=13`, `steps=48`, `a=19758`, `c=85`, `d=63`,
`status=HALTED`, `halt_reason=VInstruction`.

## Unshackled 3^19

`src/unshackled19.rs` provides the local positional codec and a separate 3^19
machine. It exports `wasmbolge_run_unshackled19` with the same caller-owned
buffer ABI as `wasmbolge_run`; `wasmbolge_run` remains Classic. The machine
uses lazy 65536-cell chain pages, 3^19 wrapping, the fast20 CRZ/rotation
semantics, direct 135-character encryption, and EOF=`END-1`.

The Unshackled export is exercised by native tests and the wasm smoke test;
the JavaScript convenience wrapper currently exposes the Classic method only.
