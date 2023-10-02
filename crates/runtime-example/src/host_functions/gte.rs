use clarity::vm::Value;
use wasm_rustref::runtime::{ClarityWasmContext, StackFrame, AsStack};
use wasmtime::{Func, Caller, AsContextMut};

#[allow(dead_code)]
fn fn_greater_than_or_equal_to(mut store: impl AsContextMut<Data = ClarityWasmContext>) -> Func {
    Func::wrap(
        &mut store,
        #[inline]
        |caller: Caller<'_, ClarityWasmContext>, a_ptr: i32, b_ptr: i32| -> i32 {
            caller.as_stack().exec(|frame: StackFrame<'_>| {
                let a = unsafe { frame.get_unchecked(a_ptr) };
                let b = unsafe { frame.get_unchecked(b_ptr) };

                let result = match (a, b) {
                    (Some(Value::Int(a)), Some(Value::Int(b))) => {
                        Value::Bool(a >= b)
                    }
                    (Some(Value::UInt(a)), Some(Value::UInt(b))) => {
                        Value::Bool(a >= b)
                    }
                    _ => unimplemented!("Greater-than-or-equal-to is not implemented for given types"),
                };

                vec![result]
            });
            5
        },
    )
}