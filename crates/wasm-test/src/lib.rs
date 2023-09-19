//use mimalloc::MiMalloc;

//#[global_allocator]
//static GLOBAL: MiMalloc = MiMalloc;

// Private modules

// Public modules
pub mod compiler;
pub mod runtime;
pub mod serialization;

use runtime::alloc::WasmAllocator;
// Public exports
pub use runtime::get_all_functions;

// Test-related
#[cfg(test)]
mod tests;

#[derive(Debug, Copy, Clone)]
pub struct ClarityWasmContext {
    pub alloc: WasmAllocator,
}

impl ClarityWasmContext {
    pub fn new() -> Self {
        ClarityWasmContext {
            alloc: WasmAllocator::new(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Ptr {
    pub offset: u32,
    pub len: u32,
}

impl Ptr {
    pub fn new(offset: u32, len: u32) -> Self {
        Ptr { offset, len }
    }

    pub fn new_uint(offset: u32, len: u32) -> Self {
        Ptr {
            offset: offset as u32,
            len: len as u32,
        }
    }

    pub fn offset_i32(&self) -> i32 {
        self.offset as i32
    }

    pub fn len_i32(&self) -> i32 {
        self.len as i32
    }

    pub(crate) fn set_offset(&mut self, offset: u32) {
        self.offset = offset;
    }

    pub(crate) fn set_len(&mut self, len: u32) {
        self.len = len;
    }
}
