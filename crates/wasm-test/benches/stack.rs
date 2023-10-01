use clarity::vm::Value;
use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Duration;
use wasm_test::runtime::stack::*;

criterion_group! {
    name = stack_benches;
    config = Criterion::default().measurement_time(Duration::from_secs(5));
    targets =
        local_push_checked,
        local_push_unchecked,
        push_1_local_from_frame,
        push_2_locals_from_frame,
        push_5_locals_from_frame,
        push_5000_locals_from_frame,
        get_1_local_from_frame,
}

criterion_main!(stack_benches,);

pub fn local_push_checked(c: &mut Criterion) {
    c.bench_function("stack/push/checked i128", move |b| {
        let stack = Stack::new();
        let frame = stack.as_frame();

        b.iter_batched(
            || {
                unsafe { stack.clear_locals() };
            },
            |_| {
                frame.push(&Value::Int(5));
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

pub fn local_push_unchecked(c: &mut Criterion) {
    c.bench_function("stack/push/unchecked i128", move |b| {
        let stack = Stack::new();
        let frame = stack.as_frame();

        b.iter_batched(
            || {
                unsafe { stack.clear_locals() };
            },
            |_| {
                frame.push(&Value::Int(5));
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

pub fn push_1_local_from_frame(c: &mut Criterion) {
    c.bench_function("stack/push/one local from frame", move |b| {
        let stack = Stack::new();

        b.iter_batched(
            || {
                unsafe { stack.clear_locals() };
            },
            |_| {
                stack.exec(|frame: StackFrame| {
                    frame.push(&Value::Int(5));
                    vec![]
                });
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

pub fn push_2_locals_from_frame(c: &mut Criterion) {
    c.bench_function("stack/push/two locals from frame", move |b| {
        let stack = Stack::new();

        b.iter_batched(
            || {
                unsafe { stack.clear_locals() };
            },
            |_| {
                stack.exec(|frame: StackFrame| {
                    frame.push(&Value::Int(1));
                    frame.push(&Value::Int(2));
                    vec![]
                });
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

pub fn push_5_locals_from_frame(c: &mut Criterion) {
    c.bench_function("stack/push/five locals from frame", move |b| {
        let stack = Stack::new();

        b.iter_batched(
            || {
                unsafe { stack.clear_locals() };
            },
            |_| {
                stack.exec(|frame: StackFrame| {
                    frame.push(&Value::Int(1));
                    frame.push(&Value::Int(2));
                    frame.push(&Value::Int(3));
                    frame.push(&Value::Int(4));
                    frame.push(&Value::Int(5));
                    vec![]
                });
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

pub fn push_5000_locals_from_frame(c: &mut Criterion) {
    c.bench_function("stack/push/5 000 locals from frame", move |b| {
        let stack = Stack::new();

        b.iter_batched(
            || {
                unsafe { stack.clear_locals() };
            },
            |_| {
                stack.exec(|frame: StackFrame| {
                    for i in 0..5000 {
                        frame.push(&Value::Int(i));
                    }
                    vec![]
                });
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

pub fn get_1_local_from_frame(c: &mut Criterion) {
    c.bench_function("stack/get/one local from frame", move |b| {
        let stack = Stack::new();
        let frame = stack.as_frame();
        let ptr = frame.push(&Value::Int(5));

        b.iter_batched(
            || {},
            |_| {
                stack.exec(|frame: StackFrame| {
                    frame.get(ptr);
                    vec![]
                });
            },
            criterion::BatchSize::SmallInput,
        );
    });
}
