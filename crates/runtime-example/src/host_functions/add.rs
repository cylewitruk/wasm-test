use clarity::vm::Value;
use wasm_rustref::{AsStack, ClarityWasmContext, StackFrame};

host_function!(add => {
    module = "clarity",
    params = [a_ptr, b_ptr]
});

impl Exec for Add {
    #[inline]
    fn exec(caller:wasmtime::Caller<'_,ClarityWasmContext>,a_ptr:i32,b_ptr:i32,) -> wasmtime::Result<()> {
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
        Ok(())
    }
}