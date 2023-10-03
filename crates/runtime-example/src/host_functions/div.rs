use clarity::vm::Value;
use wasm_rustref::runtime::{ClarityWasmContext, StackFrame, AsStack};
use wasmtime::{Func, Caller, AsContextMut};

host_function!(div => {
    module = "clarity",
    params = [a_ptr, b_ptr]
});

impl Exec for Div {
    #[inline]
    fn exec(caller:Caller<'_,ClarityWasmContext>,a_ptr:i32,b_ptr:i32,) -> wasmtime::Result<()> {
        caller.as_stack().exec(|frame: StackFrame<'_>| {
            let a = unsafe { frame.get_unchecked(a_ptr) };
            let b = unsafe { frame.get_unchecked(b_ptr) };

            let result = match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => {
                    Value::Int(a.checked_div(*b).unwrap())
                }
                (Some(Value::UInt(a)), Some(Value::UInt(b))) => {
                    Value::UInt(a.checked_div(*b).unwrap())
                }
                _ => unimplemented!("Division is not implemented for given types"),
            };

            vec![result]
        });
        Ok(())
    }
}