## Requirements
- Install the wasm32 rust target: ```rustup target add wasm32-unknown-unknown```
- Install the `nightly` rust toolchain: ```rustup toolchain install nightly```
- Install `rust-src` nightly component: ```rustup +nightly component add rust-src```
- Install `llvm-strip` command line tool (`llvm` package)
- Install `wasm-opt` command line tool (`binaryen` package)

## Building
Run `cargo install` to make sure you've got everything. After that, it's just to `cargo make build`. This will result in a `wasm_stdlib.wasm` in `target/wasm32-unknown-unknown/release/` which you can then wasm2wat (or using the VSCode extension `WebAssembly Toolkit for VSCode` you can easily view the generated WAT).

The following steps will be executed:
- `cargo +nightly build --target=wasm32-unknown-unknown --release -Z build-std=core`
- `llvm-strip --keep-section=name ../../target/wasm32-unknown-unknown/release/wasm_stdlib.wasm`
- `wasm-opt -03 -o ../../target/wasm32-unknown-unknown/release/wasm_stdlib.wasm ../../target/wasm32-unknown-unknown/release/wasm_stdlib.wasm`

## Example
The following Rust code:
```rust
#[no_mangle]
#[export_name = "add-int128"]
pub extern "C" fn add_int128(a_lo: i64, a_hi: i64, b_lo: i64, b_hi: i64) -> (i64, i64) {
    let a = ((a_lo as u64) as u128) | ((a_hi as u64) as u128) << 64;

    let b = ((b_lo as u64) as u128) | ((b_hi as u64) as u128) << 64;

    let result = a + b;
    if result > i128::MAX as u128 {
        return (-1, -1);
    }

    (
        (result & 0xFFFFFFFFFFFFFFFF) as i64,
        ((result >> 64) & 0xFFFFFFFFFFFFFFFF) as i64,
    )
}
```

Compiles to the following WAT:
```wasm
(module
  (type (;0;) (func (param i32 i64 i64 i64 i64)))
  (func (;0;) (type 0) (param i32 i64 i64 i64 i64)
    (local i32)
    local.get 0
    i64.const -1
    local.get 2
    local.get 1
    local.get 1
    local.get 3
    i64.add
    local.tee 1
    i64.gt_u
    i64.extend_i32_u
    i64.add
    local.get 4
    i64.add
    local.tee 2
    local.get 2
    i64.const 0
    i64.lt_s
    local.tee 5
    select
    i64.store offset=8
    local.get 0
    i64.const -1
    local.get 1
    local.get 5
    select
    i64.store)
  (memory (;0;) 16)
  (global (;0;) i32 (i32.const 1048576))
  (global (;1;) i32 (i32.const 1048576))
  (export "memory" (memory 0))
  (export "add-int128" (func 0))
  (export "__data_end" (global 0))
  (export "__heap_base" (global 1)))

```