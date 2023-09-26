use std::time::Duration;
use clarity::vm::Value;
use criterion::{criterion_group, criterion_main, Criterion};
use wasm_test::runtime::stack::*;

criterion_group!{
    name = stack_benches;
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets = local_push
}

criterion_main!(
    stack_benches,
);

pub fn local_push(c: &mut Criterion) {
    c.bench_function("stack/push/i128", move |b| {

        b.iter_batched(
            || {},
            |_| {
            },
            criterion::BatchSize::SmallInput,
        );
    });
}