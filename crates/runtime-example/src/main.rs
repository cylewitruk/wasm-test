use std::rc::Rc;
use wasm_rustref::runtime::{Stack, ClarityWasmContext};
use wasmtime::{Store, Config, Engine};

#[macro_use]
extern crate wasm_rustref;

// Include each host function individually.
register_host_functions!(
    add,
    div,
    gt,
    gte,
    lt,
    lte,
    mul,
    sub
);

fn main() {
    let config = Config::default();
    let engine = Engine::new(&config)
        .expect("Failed to initialize engine");
    let stack = Rc::new(Stack::default());
    let data = ClarityWasmContext::new(Rc::clone(&stack));
    let store = Store::new(&engine, data);
}