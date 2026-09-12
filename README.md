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
