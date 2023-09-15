use crate::ClarityWasmContext;
use clarity::vm::Value;
use test_case::test_case;
use wasmtime::{Config, Engine, ExternRef, Store, Val};

#[test_case(Value::Int(1), Value::Int(2) => Value::Int(3))]
#[test_case(Value::UInt(2), Value::UInt(3) => Value::UInt(5))]
fn test_add_extref(a: Value, b: Value) -> Value {
    let mut store = get_new_store();
    let add_fn = crate::runtime::native_functions::define_add_extref(&mut store);
    let params = &[
        Val::ExternRef(Some(ExternRef::new(a))),
        Val::ExternRef(Some(ExternRef::new(b))),
    ];
    let mut results = [Val::ExternRef(Some(ExternRef::new(Value::none())))];
    add_fn
        .call(store, params, &mut results)
        .expect("Failed to call function");

    results[0]
        .unwrap_externref()
        .unwrap()
        .data()
        .downcast_ref::<Value>()
        .unwrap()
        .to_owned()
}

/// Helper function. Initializes a clean new `Store` using defaults, but
/// with WASM reference types enabled.
fn get_new_store() -> Store<ClarityWasmContext> {
    let mut config = Config::default();
    config.wasm_reference_types(true);
    let engine = Engine::new(&config).expect("Failed to initialize Wasmtime Engine.");
    let context = ClarityWasmContext {};
    Store::new(&engine, context)
}
