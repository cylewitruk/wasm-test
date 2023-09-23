use clarity::vm::Value;
use criterion::{criterion_group, criterion_main, Criterion, black_box};
use wasm_test::ClarityWasmContext;
use wasmtime::{Config, Engine, Store};

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut store = get_new_store();
    let data = store.data_mut();

    c.bench_function("context/push/i128", move |b| {
        b.iter_batched(|| {}, |_| {
            let value = Value::Int(1);
            let ptr = data.values.push(value);
            data.values.drop(ptr);
        },
        criterion::BatchSize::SmallInput);
    });

    c.bench_function("context/take/i128", move |b| {
        let mut store = get_new_store();
        let value = Value::Int(1);
        let ptr = store.data_mut().values.push(value);

        b.iter(|| {
            store.data_mut().values.take(ptr);
        })
    });

    c.bench_function("context/drop/i128", move |b| {
        let mut store = get_new_store();
        let value = Value::Int(1);
        let ptr = store.data_mut().values.push(value);

        b.iter(|| {
            store.data_mut().values.drop(ptr);
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
