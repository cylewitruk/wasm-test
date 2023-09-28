pub(crate) mod alloc;
#[macro_use]
pub mod stack;

pub mod native_functions;
pub use native_functions::get_all_functions;

use crate::ValuesContext;

use self::{alloc::WasmAllocator, stack::Stack};

#[derive(Debug)]
pub struct ClarityWasmContext {
    pub alloc: WasmAllocator,
    pub values: ValuesContext,
    pub stack: Stack,
}

impl Default for ClarityWasmContext {
    fn default() -> Self {
        Self {
            alloc: WasmAllocator::default(),
            values: ValuesContext::default(),
            stack: Stack::new(),
        }
    }
}

impl ClarityWasmContext {
    /// Creates a new instance of ClarityWasmContext, the data context which
    /// is passed around to host functions.
    pub fn new() -> Self {
        ClarityWasmContext::default()
    }
}