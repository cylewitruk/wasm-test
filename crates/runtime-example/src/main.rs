use std::rc::Rc;
use wasm_rustref::runtime::{ClarityWasmContext, Stack};
use wasmtime::{Config, Engine, Store};

#[macro_use]
extern crate wasm_rustref;

//host_functions!(host_functions => add, sub);

pub(crate) mod host_functions {
    pub(crate) mod add;
    pub(crate) mod sub;
    pub fn wasmtime_imports(
        mut store: impl wasmtime::AsContextMut<Data = wasm_rustref::runtime::ClarityWasmContext>,
    ) -> Vec<wasmtime::Extern> {
        let ret: Vec<wasmtime::Extern> = Default::default();
        let ext_func: wasmtime::Func = add::Add::wasmtime_func(&mut store);
        let ext_func: wasmtime::Func = sub::Sub::wasmtime_func(&mut store);
        ret
    }
}

fn main() {
    let config = Config::default();
    let engine = Engine::new(&config).expect("Failed to initialize engine");
    let stack = Rc::new(Stack::default());
    let data = ClarityWasmContext::new(Rc::clone(&stack));
    let store = Store::new(&engine, data);
    //host_functions::add::Add::wasmtime_func(_);
}
