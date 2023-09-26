use std::time::Duration;
use clarity::vm::Value;
use criterion::{criterion_group, criterion_main, Criterion};
use wasm_test::runtime::stack::*;

criterion_group!{
    name = stack_benches;
    config = Criterion::default().measurement_time(Duration::from_secs(5));
    targets = local_push
}

criterion_main!(
    stack_benches,
);

pub fn local_push(c: &mut Criterion) {
    c.bench_function("stack/push/i128", move |b| {
        let stack = Stack::new();

        b.iter_batched(
            || {
                stack.clear_locals();
            },
            |_| {
                stack.local_push(Value::Int(5));
            },
            criterion::BatchSize::SmallInput,
        );
    });
}