//use clarity::vm::Value;
use wasm_rustref::{AsStack, ClarityWasmContext, StackFrame};

// Generate boilerplate code for the `add` method.
host_function!(fold => {
    module = "clarity",
    params = [func_ptr, list_ptr, init_ptr]
});

impl Exec for Fold {
    #[inline]
    fn exec(caller:wasmtime::Caller<'_,ClarityWasmContext>, func_ptr:i32, list_ptr:i32, init_ptr:i32,) -> wasmtime::Result<()> {
        caller.as_stack().exec(|frame: StackFrame<'_>| {
            let _func = unsafe { frame.get_unchecked(func_ptr) };
            let _list = unsafe { frame.get_unchecked(list_ptr) };
            let _init = unsafe { frame.get_unchecked(init_ptr) };

            todo!()
        });
        todo!()
    }
}