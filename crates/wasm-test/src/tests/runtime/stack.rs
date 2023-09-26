/* use std::cell::{Ref, RefMut};

use clarity::vm::Value;
use wasmtime::{AsContextMut, Caller, Config, Engine, Func, Store};

use crate::{
    runtime::{
        stack::{HostStack, LocalsContext, FrameContext, AsStackMut, AsFrameContextMut},
    },
    ClarityWasmContext,
};

/// Test function
#[test]
fn test() {
    let mut store = get_new_store();
    let stack = HostStack::default();

    let func = Func::wrap(&mut store, move |mut caller: Caller<'_, ClarityWasmContext>| {
        stack.frame(|mut frame: FrameContext<Value>| {
            frame.local_set(1, Value::Int(1));
            frame.local_take(5);
            caller.data_mut().values.drop(5);
        });
    });

    func.call(&mut store, &[], &mut [])
        .expect("Failed to call function");


}

/// Helper function. Initializes a clean new `Store` using defaults, but
/// with WASM reference types enabled.
fn get_new_store() -> Store<ClarityWasmContext> {
    let mut config = Config::default();
    config.wasm_reference_types(true);
    let engine = Engine::new(&config).expect("Failed to initialize Wasmtime Engine.");
    let context = ClarityWasmContext::new();
    Store::new(&engine, context)
}
*/
