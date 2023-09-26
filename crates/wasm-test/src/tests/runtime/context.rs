use clarity::vm::Value;
use wasmtime::{Config, Engine, Store};

use crate::ClarityWasmContext;

#[test]
fn test_new_ptr() {
    let mut store = get_new_store();

    let values = &mut store.data_mut().values;

    assert_eq!(0, values.new_ptr());
    assert_eq!(1, values.new_ptr());
    assert_eq!(2, values.new_ptr());
    values.drop(2);
    assert_eq!(2, values.push(Value::Int(1)));
    assert_eq!(3, values.new_ptr());
    assert_eq!(4, values.push(Value::Int(2)));
    assert_eq!(5, values.new_ptr());
    values.drop(5);
    assert_eq!(5, values.new_ptr());
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
