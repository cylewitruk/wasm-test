//use mimalloc::MiMalloc;

//#[global_allocator]
//static GLOBAL: MiMalloc = MiMalloc;

// Private modules

// Public modules
pub mod compiler;
pub mod runtime;
pub mod serialization;

//use ahash::AHashMap;
use fxhash::FxHashMap;

use clarity::{vm::Value, address::b58::from};
use runtime::alloc::WasmAllocator;
// Public exports
pub use runtime::get_all_functions;

// Test-related
#[cfg(test)]
mod tests;
/*
#[derive(Debug, Clone)]
pub struct ClarityWasmContext {
    pub alloc: WasmAllocator,
    owned_values: FxHashMap<i32, Value>,
    owned_counter: i32
}

impl Default for ClarityWasmContext {
    fn default() -> Self {
        Self {
            alloc: WasmAllocator::new(),
            owned_values: FxHashMap::<i32, Value>::default(),
            owned_counter: i32::MIN
        }
    }
}

impl ClarityWasmContext {
    /// Creates a new instance of ClarityWasmContext, the data context which
    /// is passed around to host functions.
    pub fn new() -> Self {
        ClarityWasmContext::default()
    }

    pub fn push_value(&mut self, value: Value) -> i32 {
        let idx = self.owned_counter;
        self.owned_counter += 1;
        self.owned_values.insert(idx, value);
        idx
    }

    pub fn get_value(&self, ptr: i32) -> Value {
        self.owned_values.get(&ptr).unwrap().to_owned()
    }

    pub fn set_value(&mut self, ptr: i32, value: Value) {
        self.owned_values.insert(ptr, value);
    }

    pub fn copy_value_into(&mut self, from_ptr: i32, to_ptr: i32) {
        self.owned_values.insert(
            to_ptr, 
            self.owned_values.get(&from_ptr).unwrap().clone()
        );
    }

    pub fn new_ptr(&mut self) -> i32 {
        let idx = self.owned_counter;
        self.owned_counter += 1;
        idx
    }

    pub fn drop_ptr(&mut self, ptr: i32) {
        self.owned_values.remove(&ptr);
    }

    pub fn value_count(&self) -> usize {
        self.owned_values.len()
    }

    pub fn clear_values(&mut self) {
        self.owned_values.clear();
    }
}*/

#[derive(Debug, Clone)]
pub struct ValuesContext {
    owned_values: Vec<Option<Value>>,
    tombstones: Vec<i32>
}

impl Default for ValuesContext {
    fn default() -> Self {
        Self { 
            owned_values: Vec::<Option<Value>>::with_capacity(1000), 
            tombstones: Vec::<i32>::with_capacity(1000)
        }
    }
}

impl ValuesContext {
    pub fn push(&mut self, value: Value) -> i32 {
        if let Some(idx) = self.tombstones.pop() {
            self.owned_values[idx as usize] = Some(value);
            idx
        } else {
            let idx = self.owned_values.len();
            self.owned_values.push(Some(value));
            idx as i32
        }
    }

    pub fn take(&mut self, ptr: i32) -> Option<Value> {
        let value = self.owned_values[ptr as usize].take();
        value
    }

    pub fn borrow(&self, ptr: i32) -> Option<&Value> {
        self.owned_values[ptr as usize].as_ref()
    }

    pub fn set(&mut self, ptr: i32, value: Value) {
        self.owned_values[ptr as usize] = Some(value);
    }

    pub fn copy_into(&mut self, from_ptr: i32, to_ptr: i32) {
        self.owned_values[to_ptr as usize] = self.owned_values[from_ptr as usize].clone();
    }

    pub fn new_ptr(&mut self) -> i32 {
        if let Some(idx) = self.tombstones.pop() {
            idx
        } else {
            let idx = self.owned_values.len();
            self.owned_values.push(None);
            idx as i32
        }
    }

    pub fn drop(&mut self, ptr: i32) {
        self.tombstones.push(ptr);
        self.owned_values[ptr as usize] = None;

    }

    pub fn count(&self) -> usize {
        self.owned_values.len()
    }

    pub fn clear(&mut self) {
        self.owned_values.clear();
        self.tombstones.clear();
    }
}

#[derive(Debug, Clone)]
pub struct ClarityWasmContext {
    pub alloc: WasmAllocator,
    pub values: ValuesContext
}

impl Default for ClarityWasmContext {
    fn default() -> Self {
        Self {
            alloc: WasmAllocator::default(),
            values: ValuesContext::default()
        }
    }
}

impl ClarityWasmContext {
    /// Creates a new instance of ClarityWasmContext, the data context which
    /// is passed around to host functions.
    pub fn new() -> Self {
        ClarityWasmContext::default()
    }

    pub fn borrow(&mut self) -> &mut Self {
        self
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
            offset,
            len,
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
