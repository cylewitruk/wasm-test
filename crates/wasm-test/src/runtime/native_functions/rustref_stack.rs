use crate::runtime::{AsStack, ClarityWasmContext, StackFrame};
use clarity::vm::Value;
use log::*;
use wasmtime::{AsContextMut, Caller, Func};

#[inline]
pub fn define_add_rustref_stack(mut store: impl AsContextMut<Data = ClarityWasmContext>) -> Func {
    Func::wrap(
        &mut store,
        #[inline]
        |caller: Caller<'_, ClarityWasmContext>, a_ptr: i32, b_ptr: i32| -> i32 {
            debug!("[add_rustref_stack] executing in frame");
            caller.as_stack().exec(|frame: StackFrame<'_>| {
                debug!("[add_rustref_stack]:{}", frame);
                let a = unsafe { frame.get_unchecked(a_ptr) };
                let b = unsafe { frame.get_unchecked(b_ptr) };

                trace!("a_ptr={}", a_ptr);
                trace!("a={:?}", a.unwrap());
                trace!("b_ptr={}", b_ptr);
                trace!("b={:?}", b.unwrap());

                let result = match (a, b) {
                    (Some(Value::Int(a)), Some(Value::Int(b))) => {
                        Value::Int(a.checked_add(*b).unwrap())
                    }
                    (Some(Value::UInt(a)), Some(Value::UInt(b))) => {
                        Value::UInt(a.checked_add(*b).unwrap())
                    }
                    _ => todo!("Add not implemented for given types"),
                };

                vec![result]
            });
            5
        },
    )
}
