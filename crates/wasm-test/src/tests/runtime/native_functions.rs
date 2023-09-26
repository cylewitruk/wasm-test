use crate::ClarityWasmContext;
use clarity::vm::Value;
use test_case::test_case;
use wasmtime::{AsContextMut, Caller, Config, Engine, ExternRef, Func, Store, Val};

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
        .call(&mut store, params, &mut results)
        .expect("Failed to call function");

    results[0]
        .unwrap_externref()
        .unwrap()
        .data()
        .downcast_ref::<Value>()
        .unwrap()
        .to_owned()
}

#[test_case(Value::Int(1), Value::Int(2) => Value::Int(3))]
#[test_case(Value::UInt(2), Value::UInt(3) => Value::UInt(5))]
fn test_add_rustref(a: Value, b: Value) -> Value {
    let mut store = get_new_store();

    let a_ptr = store.data_mut().values.push(a);
    let b_ptr = store.data_mut().values.push(b);

    let add_fn = crate::runtime::native_functions::define_add_rustref(&mut store);
    let params = &[Val::I32(a_ptr), Val::I32(b_ptr)];

    let mut results = [Val::null()];

    add_fn
        .call(&mut store, params, &mut results)
        .expect("Failed to call function");

    let result_ptr = results[0].unwrap_i32();
    let result = store.data_mut().values.take(result_ptr);
    println!("Result: {:?}", result);
    result.unwrap()
}

#[test_case(Value::Int(1), Value::Int(2) => Value::Int(2))]
#[test_case(Value::UInt(2), Value::UInt(3) => Value::UInt(6))]
#[test_case(Value::UInt(5), Value::UInt(5) => Value::UInt(25))]
fn test_mul_rustref(a: Value, b: Value) -> Value {
    let mut store = get_new_store();

    let a_ptr = store.data_mut().values.push(a);
    let b_ptr = store.data_mut().values.push(b);

    let add_fn = crate::runtime::native_functions::define_mul_rustref(&mut store);
    let params = &[Val::I32(a_ptr), Val::I32(b_ptr)];

    let mut results = [Val::null()];

    add_fn
        .call(&mut store, params, &mut results)
        .expect("Failed to call function");

    let result_ptr = results[0].unwrap_i32();
    let result = store.data_mut().values.take(result_ptr);
    println!("Result: {:?}", result);
    result.unwrap()
}

#[test]
fn test_fold_rustref() {
    let mut store = get_new_store();

    let fold_fn = crate::runtime::native_functions::define_fold_rustref(&mut store);
    let add_fn = crate::runtime::native_functions::define_add_rustref(&mut store);

    // Define our input parameters.
    let list = Value::list_from((1..=100).map(Value::Int).collect())
        .expect("failed to construct list argument");
    let init = Value::Int(1);

    let list_ptr = store.data_mut().values.push(list);
    let init_ptr = store.data_mut().values.push(init);

    let params = &[
        Val::FuncRef(Some(add_fn)),
        Val::I32(list_ptr),
        Val::I32(init_ptr),
    ];

    let mut results = [Val::null()];

    fold_fn
        .call(&mut store, params, &mut results)
        .expect("Failed to call function");

    let result_ptr = results[0].unwrap_i32();
    println!(
        "Result ptr: {}, data size: {}",
        results[0].unwrap_i32(),
        store.data().values.count()
    );
    let result = store.data_mut().values.take(result_ptr);
    println!("Result: {:?}", result);
    assert_eq!(183285493761, result.unwrap().expect_i128());
}

/// Helper function. Initializes a clean new `Store` using defaults, but
/// with WASM reference types enabled.
fn get_new_store() -> Store<ClarityWasmContext> {
    let mut config = Config::default();
    config.wasm_reference_types(true);
    let engine = Engine::new(&config).expect("Failed to initialize Wasmtime Engine.");
    let context = ClarityWasmContext::default();
    Store::new(&engine, context)
}
