use std::time::Duration;
use clarity::vm::Value;
use criterion::{criterion_group, criterion_main, Criterion};
use wasm_test::runtime::stack::*;

criterion_group!{
    name = stack_benches;
    config = Criterion::default().measurement_time(Duration::from_secs(5));
    targets = 
        local_push, 
        exec_single_frame,
        push_single_local_from_frame,
        push_five_locals_from_frame
}

criterion_main!(
    stack_benches,
);

pub fn local_push(c: &mut Criterion) {
    c.bench_function("stack/push/i128", move |b| {
        let stack = Stack::new();
        let frame = Frame::new(1, &stack, None);

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

pub fn exec_single_frame(c: &mut Criterion) {
    c.bench_function("stack/exec-single-frame", move |b| {
        let stack = Stack::new();

        b.iter_batched(|| {

            },
            |_| {
                stack.exec(|_| { })
            }, 
            criterion::BatchSize::SmallInput
        );
    });
}

pub fn push_single_local_from_frame(c: &mut Criterion) {
    c.bench_function("stack/push_from_frame", move |b| {
        let stack = Stack::new();

        b.iter_batched(|| {
                stack.clear_locals();
            },
            |_| {
                stack.exec(|frame| {
                    frame.local_push(Value::Int(5));
                });
            }, 
            criterion::BatchSize::SmallInput
        );
    });
}

pub fn push_five_locals_from_frame(c: &mut Criterion) {
    c.bench_function("stack/push_from_frame", move |b| {
        let stack = Stack::new();

        b.iter_batched(|| {
                stack.clear_locals();
            },
            |_| {
                stack.exec(|frame| {
                    frame.local_push(Value::Int(1));
                    frame.local_push(Value::Int(2));
                    frame.local_push(Value::Int(3));
                    frame.local_push(Value::Int(4));
                    frame.local_push(Value::Int(5));
                });
            }, 
            criterion::BatchSize::SmallInput
        );
    });
}