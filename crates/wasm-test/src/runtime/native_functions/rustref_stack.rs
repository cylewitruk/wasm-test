use clarity::vm::Value;
use wasmtime::{AsContextMut, Func, Caller};

use crate::runtime::{ClarityWasmContext, AsStack};

#[inline]
pub fn define_add_rustref_stack(mut store: impl AsContextMut<Data = ClarityWasmContext>) -> Func {
    Func::wrap(
        &mut store,
        |caller: Caller<'_, ClarityWasmContext>, a_ptr: i32, b_ptr: i32| -> i32 {
            caller.as_stack().exec(|frame| {
                let a = unsafe { frame.get_unchecked(a_ptr) };
                let b = unsafe { frame.get_unchecked(b_ptr) };

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