## Requirements
- Install the wasm32 rust target: ```rustup target add wasm32-unknown-unknown```
- Install the `nightly` rust toolchain: ```rustup toolchain install nightly```
- Install `rust-src` nightly component: ```rustup +nightly component add rust-src```
- Install `llvm-strip` command line tool (`llvm` package)
- Install `wasm-opt` command line tool (`binaryen` package)

## Building
Run `cargo install` to make sure you've got everything. After that, it's just to `cargo make build`. This will result in a `wasm_stdlib.wasm` in `target/wasm32-unknown-unknown/release/` which you can then wasm2wat (or using the VSCode extension `WebAssembly Toolkit for VSCode` you can easily view the generated WAT).