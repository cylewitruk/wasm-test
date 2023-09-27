use std::time::Duration;
use clarity::vm::Value;
use criterion::{criterion_group, criterion_main, Criterion};
use wasm_test::runtime::stack::*;

criterion_group!{
    name = stack_benches;
    config = Criterion::default().measurement_time(Duration::from_secs(5));
    targets = 
        local_push, 
        push_1_local_from_frame,
        push_2_locals_from_frame,
        push_2_locals_from_frame,
        push_5_locals_from_frame
}

criterion_main!(
    stack_benches,
);

pub fn local_push(c: &mut Criterion) {
    c.bench_function("stack/push/i128", move |b| {
        let stack = Stack::new();
        let frame = stack.as_frame();

        b.iter_batched(
            || {
                frame.clear();
            },
            |_| {
                frame.push(Value::Int(5));
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

pub fn push_1_local_from_frame(c: &mut Criterion) {
    c.bench_function("stack/push_single_local_from_frame", move |b| {
        let stack = Stack::new();
        let frame = stack.as_frame();
        let mut results = Vec::<i32>::new();

        b.iter_batched(|| {
                frame.clear();
            },
            |_| {
                stack.exec(&mut results, |frame: StackFrame| {
                    frame.push(Value::Int(5));
                    vec![]
                });
            }, 
            criterion::BatchSize::SmallInput
        );
    });
}

pub fn push_2_locals_from_frame(c: &mut Criterion) {
    c.bench_function("stack/push/two_locals_separately", move |b| {
        let stack = Stack::new();
        let mut results = Vec::<i32>::new();

        b.iter_batched(|| {
                stack.clear_locals();
            },
            |_| {
                stack.exec(&mut results,|frame: StackFrame| {
                    frame.push(Value::Int(1));
                    frame.push(Value::Int(2));
                    vec![]
                });
            }, 
            criterion::BatchSize::SmallInput
        );
    });
}

pub fn push_5_locals_from_frame(c: &mut Criterion) {
    c.bench_function("stack/push_five_locals_from_frame", move |b| {
        let stack = Stack::new();
        let mut results = Vec::<i32>::new();

        b.iter_batched(|| {
                stack.clear_locals();
            },
            |_| {
                stack.exec(&mut results,|frame: StackFrame| {
                    frame.push(Value::Int(1));
                    frame.push(Value::Int(2));
                    frame.push(Value::Int(3));
                    frame.push(Value::Int(4));
                    frame.push(Value::Int(5));
                    vec![]
                });
            }, 
            criterion::BatchSize::SmallInput
        );
    });
}