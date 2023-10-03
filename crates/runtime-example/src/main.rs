use std::rc::Rc;
use wasm_rustref::runtime::{ClarityWasmContext, Stack};
use wasmtime::{Config, Engine, Store};

#[macro_use]
extern crate wasm_rustref;

mod host_functions {
    mod add;
}

fn main() {
    let config = Config::default();
    let engine = Engine::new(&config).expect("Failed to initialize engine");
    let stack = Rc::new(Stack::default());
    let data = ClarityWasmContext::new(Rc::clone(&stack));
    let store = Store::new(&engine, data);
}
