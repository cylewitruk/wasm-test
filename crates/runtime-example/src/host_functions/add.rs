use clarity::vm::Value;
use wasm_rustref::runtime::{AsStack, ClarityWasmContext, StackFrame};
use wasmtime::{AsContextMut, Caller, Func};

host_function!(add => {
    module = "clarity",
    params = [a_ptr, b_ptr]
});

impl Exec for Add {
    fn exec(caller:Caller<'_,ClarityWasmContext>,a_ptr:i32,b_ptr:i32,) -> wasmtime::Result<()> {
        todo!()
    }
}

#[allow(dead_code)]
fn fn_add(mut store: impl AsContextMut<Data = ClarityWasmContext>) -> Func {
    Func::wrap(
        &mut store,
        #[inline]
        |caller: Caller<'_, ClarityWasmContext>, a_ptr: i32, b_ptr: i32| -> i32 {
            caller.as_stack().exec(|frame: StackFrame<'_>| {
                let a = unsafe { frame.get_unchecked(a_ptr) };
                let b = unsafe { frame.get_unchecked(b_ptr) };

                let result = match (a, b) {
                    (Some(Value::Int(a)), Some(Value::Int(b))) => {
                        Value::Int(a.checked_add(*b).unwrap())
                    }
                    (Some(Value::UInt(a)), Some(Value::UInt(b))) => {
                        Value::UInt(a.checked_add(*b).unwrap())
                    }
                    _ => unimplemented!("Add is not implemented for given types"),
                };

                vec![result]
            });
            5
        },
    )
}
