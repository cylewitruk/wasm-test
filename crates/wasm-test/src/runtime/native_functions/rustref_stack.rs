use clarity::vm::Value;
use wasmtime::{AsContextMut, Func, Caller};

use crate::runtime::{ClarityWasmContext, AsStack, AsCallerExec, StackFrame};

#[inline]
pub fn define_add_rustref_stack<'a>(mut store: impl AsContextMut<Data = ClarityWasmContext>) -> Func {
    Func::wrap(
        &mut store,
        |caller: Caller<'_, ClarityWasmContext>, a_ptr: i32, b_ptr: i32| -> i32 {
            println!("[add_rustref_stack] here!");
            caller.as_stack().exec(|frame: StackFrame<'_>| {
                let a = unsafe { frame.get_unchecked(a_ptr) };
                let b = unsafe { frame.get_unchecked(b_ptr) };

                eprintln!("a_ptr={}", a_ptr);
                eprintln!("a={:?}", a.unwrap());
                eprintln!("b_ptr={}", b_ptr);
                eprintln!("b={:?}", b.unwrap());

                let result = match (a, b) {
                    (Some(Value::Int(a)), Some(Value::Int(b))) => Value::Int(a + b),
                    (Some(Value::UInt(a)), Some(Value::UInt(b))) => Value::UInt(a.checked_add(*b).unwrap()),
                    _ => todo!("Add not implemented for given types"),
                };

                vec![result]
            });
            5
        },
    )
}