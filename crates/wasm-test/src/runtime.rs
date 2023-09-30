pub(crate) mod alloc;
#[macro_use]
pub mod stack;
pub mod native_functions;

use clarity::vm::Value;
use wasmtime::{Caller, Store, AsContextMut};
use crate::ValuesContext;

pub use native_functions::get_all_functions;
pub use crate::runtime::stack::*;
pub use crate::runtime::alloc::WasmAllocator;

/// The state object which is available in all Wasmtime host function
/// calls. This is where information/structures which may be needed
/// across multiple executions should be placed.
///
/// Note: To receive a [ClarityWasmContext] in a host function you must
/// use one of the `wrap` variants which accepts a Wasmtime [Caller] as
/// the first argument. Once you have a caller, you get an instance to
/// the [ClarityWasmContext] by using `caller.data()` or `caller.data_mut()`.
#[derive(Debug, Default)]
pub struct ClarityWasmContext {
    pub alloc: WasmAllocator,
    pub values: ValuesContext
}

impl ClarityWasmContext {
    pub fn new() -> Self {
        Self {
            alloc: Default::default(),
            values: Default::default()
        }
    }
}

/// A trait which allows a consumer to receive an instance of a [Stack] from
/// an implementing structure.
pub trait AsStack {
    fn as_stack(&self) -> &Stack;
}

/*
/// Implements [AsStack] for Wasmtime's [Caller] so that consumers of
/// `wrap` functions can easily receive an instance of this [ClarityWasmContext]'s
/// [Stack].
impl AsStack for Caller<'_, ClarityWasmContext> {
    #[inline]
    fn as_stack(&self) -> &Stack {
        &self.data().stack
    }
}

impl AsStack for Store<ClarityWasmContext> {
    #[inline]
    fn as_stack(&self) -> &Stack {
        &self.data().stack
    }
}
*/

pub trait AsExec<'a> {
    fn exec (&'a mut self,
        stack: &'a Stack,
        // Added the for<> below just as a reminder in case we use lifetimes later
        func: impl FnOnce(StackFrame, &'a mut Store<ClarityWasmContext>) -> Vec<Value>,
    );
}

impl<'a> AsExec<'a> for Store<ClarityWasmContext> {
    fn exec (
        &'a mut self,
        stack: &'a Stack,
        // Added the for<> below just as a reminder in case we use lifetimes later
        func: impl FnOnce(StackFrame, &'a mut Store<ClarityWasmContext>) -> Vec<Value>,
    ) {
        unsafe {
            // Create a new virtual frame.
            let (frame, frame_index) = stack.new_frame();
            // Call the provided function.
            let mut frame_result: Vec<Value> = func(frame, self);
            #[cfg(test)] eprintln!("Frame result count: {}", frame_result.len());
            // Move the output values from the frame to the result buffer.
            stack.fill_result_buffer(frame_result);
            // Drop the frame.
            stack.drop_frame(frame_index);
        }
    }
}