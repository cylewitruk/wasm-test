/// A slightly adapted version of the Buddy Allocator provided by `toyrs-rs` crate:
/// https://github.com/emk/toyos-rs/blob/master/crates/alloc_buddy_simple/src/heap.rs
/// The original version is for bare-metal `nostd` applications, while this version
/// is adapted to work as an external memory allocator for Wasm memory. Most
/// notably, alignment has been removed since we're not working directly with memory.

use std::{collections::HashMap, ops::{Deref, DerefMut}};

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

const MIN_BLOCK_SIZE: u32 = 8;

pub struct WrappedPtr {
    id: u32,
    ptr: Ptr
}

impl WrappedPtr {
    pub fn new(id: u32, offset: i32, len: i32) -> Self {
        WrappedPtr { 
            id, 
            ptr: Ptr::new(offset, len)
        }
    }
    pub fn from_ptr(id: u32, ptr: Ptr) -> Self {
        WrappedPtr { id, ptr }
    }
}

impl Deref for WrappedPtr {
    type Target = Ptr;

    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl DerefMut for WrappedPtr {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ptr
    }
}

pub struct WasmAllocator2 {
    size: u32,
    free_lists: Vec<Vec<Ptr>>,
    allocations: HashMap<i32, u32>,
    current_id: u32
}

impl WasmAllocator2 {
    pub fn new(size: u32) -> Self {
        // The memory size must be a power of two.
        if !Self::is_power_of_two(size) {
            panic!("Allocator size must be a power of two, got '{}'.", size);
        }

        // The memory size must be able to hold at least one block, defined above as 8 bytes.
        if size < MIN_BLOCK_SIZE {
            panic!("Allocator size must be large enough to hold at least one block (8 bytes).");
        }

        let mut free_lists = Vec::<Vec<Ptr>>::with_capacity(128);
        let allocations = HashMap::new();

        // Create a free list for each possible block size.
        let free_list_count = size.ilog2();
        println!("Creating a free list count with size '{free_list_count}'.");
        for _ in 0..free_list_count {
            free_lists.push(Vec::<Ptr>::with_capacity(8));
        }

        free_lists[free_list_count as usize - 1].push(Ptr::new_uint(0, size));

        WasmAllocator2 { 
            size: size, 
            free_lists,
            allocations,
            current_id: 0
        }
    }

    /// Returns whether or not the provided uint is a power of two.
    fn is_power_of_two(n: u32) -> bool {
        n & (n - 1) == 0
    }

    /// Determines the block size that we'll need to fulfill an allocation request. This is
    /// deterministic and does not depend on what's already been allocated. In particular,
    /// it's important to be able to calculate the same `allocation_size` when freeing
    /// memory as we did when allocating it.
    pub fn allocation_size(&self, mut size: u32) -> Option<u32> {
        // We can't allocate blocks smaller than `MIN_BLOCK_SIZE`
        size = u32::max(size, MIN_BLOCK_SIZE);

        // Round up to the next power of two.
        size = size.next_power_of_two();

        // We can't allocate a block bigger than our memory space.
        if size > self.size {
            println!("Requested allocation size is greater than the managed memory spage.");
            return None;
        }

        Some(size)
    }

    /// The "order" of an allocation is how many times we need to double `MIN_BLOCK_SIZE`in
    /// order to get a large enough block, as well as the index we use into `free_lists`.
    pub fn allocation_order(&self, size: u32) -> Option<u32> {
        self.allocation_size(size).map(|s| {
            s.ilog2() - MIN_BLOCK_SIZE.ilog2()
        })
    }

    /// The size of the blocks we allocate for a given order.
    pub fn order_size(&self, order: u32) -> u32 {
        1 << (MIN_BLOCK_SIZE.ilog2() + order)
    }

    /// Pop a block from the appropriate free list.
    pub fn free_list_pop(&mut self, order: usize) -> Option<Ptr> {
        let list = &mut self.free_lists[order];
        list.pop()
    }

    /// Insert `block` of order `order`onto the appropriate free list.
    pub fn free_list_push(&mut self, order: usize, block: Ptr) {
        let list = &mut self.free_lists[order];
        list.push(block);
    }

    /// Removes the specified `block` in the provided `order` free_list. This is used during
    /// the splitting of blocks, to remove the larger block from the larger free_list (which
    /// gets replaced by two smaller blocks in a lower free_list).
    pub fn free_list_remove(&mut self, order: usize, block: Ptr) -> bool {
        let list = &mut self.free_lists[order];
        
        for i in 0..list.len() {
            if list[i].offset == block.offset {
                list.remove(i);
                return true;
            }
        }

        false
    }

    /// Split a `block` of order `order` down into a block of order `order_needed`,
    /// placing any unused chunks on the free list.
    pub fn split_free_block(&mut self, mut block: Ptr, mut order: u32, order_needed: u32) {
        // Get the size of our starting block.
        let mut size_to_split = self.order_size(order);

        println!("order_needed = {order_needed}; order = {order}; size_to_split = {size_to_split}");
        // Remove this block from the current order list.
        if !self.free_list_remove(order as usize, block) {
            panic!("Unable to free block {:?} from order {order}", block)
        }

        while order > order_needed {
            size_to_split >>= 1;
            order -= 1;

            println!("order_needed = {order_needed}; order = {order}; size_to_split = {size_to_split}");
            
        }
    }


}