use std::rc::Rc;

use clarity::vm::{
    types::{ListData, ListTypeData, SequenceData, TypeSignature},
    Value,
};
use criterion::{criterion_group, criterion_main, Criterion};
use wasm_test::runtime::{ClarityWasmContext, Stack};
use wasmtime::{Config, Engine, Store};

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("context/push/i128", move |b| {
        let stack = Rc::new(Stack::default());
        let mut store = get_new_store(Rc::clone(&stack));
        
        let data = store.data_mut();

        b.iter_batched(
            || {},
            |_| {
                let value = Value::Int(1);
                let ptr = data.values.push(value);
                data.values.drop(ptr);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    c.bench_function("context/take/i128", move |b| {
        let stack = Rc::new(Stack::default());
        let mut store = get_new_store(Rc::clone(&stack));
        
        let value = Value::Int(1);
        let ptr = store.data_mut().values.push(value);

        b.iter(|| {
            store.data_mut().values.take(ptr);
        })
    });

    c.bench_function("context/drop/i128", move |b| {
        let stack = Rc::new(Stack::default());
        let mut store = get_new_store(Rc::clone(&stack));
        let value = Value::Int(1);
        let ptr = store.data_mut().values.push(value);

        b.iter(|| {
            store.data_mut().values.drop(ptr);
        })
    });

    c.bench_function("context/push/list", move |b| {
        let stack = Rc::new(Stack::default());
        let mut store = get_new_store(Rc::clone(&stack));

        let value = Value::Sequence(SequenceData::List(ListData {
            data: vec![
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
                Value::Int(4),
                Value::Int(5),
            ],
            type_signature: ListTypeData::new_list(TypeSignature::IntType, 5)
                .expect("Could not construct list"),
        }));

        b.iter_batched(
            || {},
            |_| {
                let ptr = store.data_mut().values.push(value.clone());
                store.data_mut().values.drop(ptr);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    c.bench_function("context/take/list", move |b| {
        let stack = Rc::new(Stack::default());
        let mut store = get_new_store(Rc::clone(&stack));

        let value = Value::Sequence(SequenceData::List(ListData {
            data: vec![
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
                Value::Int(4),
                Value::Int(5),
            ],
            type_signature: ListTypeData::new_list(TypeSignature::IntType, 5)
                .expect("Could not construct list"),
        }));

        let ptr = store.data_mut().values.push(value);

        b.iter(|| {
            store.data_mut().values.take(ptr);
        })
    });

    c.bench_function("context/drop/list", move |b| {
        let stack = Rc::new(Stack::default());
        let mut store = get_new_store(Rc::clone(&stack));

        let value = Value::Sequence(SequenceData::List(ListData {
            data: vec![
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
                Value::Int(4),
                Value::Int(5),
            ],
            type_signature: ListTypeData::new_list(TypeSignature::IntType, 5)
                .expect("Could not construct list"),
        }));

        let ptr = store.data_mut().values.push(value);

        b.iter(|| {
            store.data_mut().values.drop(ptr);
        })
    });
}

/// Helper function. Initializes a clean new `Store` using defaults, but
/// with WASM reference types enabled.
fn get_new_store(stack: Rc<Stack>) -> Store<ClarityWasmContext> {
    let mut config = Config::default();
    config.wasm_reference_types(true);
    let engine = Engine::new(&config).expect("Failed to initialize Wasmtime Engine.");
    let context = ClarityWasmContext::new(Rc::clone(&stack));
    Store::new(&engine, context)
}
