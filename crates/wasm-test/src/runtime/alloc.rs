use crate::Ptr;

#[derive(Debug, Copy, Clone)]
pub struct WasmAllocator {
    next_offset: i32,
}

/// A simple bump allocator for handling Wasm memory.
impl WasmAllocator {
    /// Creates a new `WasmAllocator` with its next offset set to `0`.
    pub fn new() -> Self {
        WasmAllocator { next_offset: 0 }
    }

    /// Retrieve a pointer to the next available offset for the given size.
    pub fn alloc_for_size(&mut self, size: usize) -> Ptr {
        let len = size as i32;
        let ptr = Ptr::new(self.next_offset, len);
        self.next_offset += len;
        return ptr;
    }

    /// Retrieve a pointer to the next available offset which can store the given
    /// data slice.
    pub fn alloc_for_buffer(&mut self, data: &[u8]) -> Ptr {
        self.alloc_for_size(data.len())
    }
}
