pub(crate) mod alloc;
#[macro_use]
pub mod stack;

pub mod native_functions;
use crate::ValuesContext;
pub use native_functions::get_all_functions;
use wasmtime::Caller;
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
    pub values: ValuesContext,
    pub stack: Stack,
}

/// A trait which allows a consumer to receive an instance of a [Stack] from
/// an implementing structure.
pub trait AsStack {
    fn as_stack(&self) -> &Stack;
}

/// Implements [AsStack] for Wasmtime's [Caller] so that consumers of
/// `wrap` functions can easily receive an instance of this [ClarityWasmContext]'s
/// [Stack].
impl AsStack for Caller<'_, ClarityWasmContext> {
    #[inline]
    fn as_stack(&self) -> &Stack {
        &self.data().stack
    }
}
