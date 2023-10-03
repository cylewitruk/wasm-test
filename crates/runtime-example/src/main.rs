use std::rc::Rc;
use wasm_rustref::runtime::{ClarityWasmContext, Stack};
use wasmtime::{Config, Engine, Store};

#[macro_use]
extern crate wasm_rustref;

host_functions!(host_functions => add, sub, div, mul);

fn main() {
    let config = Config::default();
    let engine = Engine::new(&config).expect("Failed to initialize engine");
    let stack = Rc::new(Stack::default());
    let data = ClarityWasmContext::new(Rc::clone(&stack));
    let mut store = Store::new(&engine, data);
    host_functions::wasmtime_imports(&mut store);
    println!("store: {:?}", store);
}
