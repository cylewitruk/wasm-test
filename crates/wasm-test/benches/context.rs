use clarity::vm::Value;
use criterion::{criterion_group, criterion_main, Criterion, black_box};
use wasm_test::ClarityWasmContext;
use wasmtime::{Config, Engine, Store};

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut store = get_new_store();
    let data = store.data_mut();

    c.bench_function("push value", move |b| {
        b.iter_batched(|| {}, |_| {
            let value = Value::Int(1);
            let ptr = data.push_value(value);
            data.drop_ptr(ptr);
        },
        criterion::BatchSize::SmallInput);
    });

    c.bench_function("get value", move |b| {
        let mut store = get_new_store();
        let value = Value::Int(1);
        let ptr = store.data_mut().push_value(value);

        b.iter(|| {
            store.data().get_value(ptr);
        })
    });

    c.bench_function("drop value", move |b| {
        let mut store = get_new_store();
        let value = Value::Int(1);
        let ptr = store.data_mut().push_value(value);

        b.iter(|| {
            store.data_mut().drop_ptr(ptr);
        })
    });
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
