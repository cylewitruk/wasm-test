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

use clarity::{address::b58::from, vm::Value};
use runtime::{alloc::WasmAllocator, stack::Stack};
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
    tombstones: Vec<i32>,
}

impl Default for ValuesContext {
    fn default() -> Self {
        Self {
            owned_values: Vec::<Option<Value>>::with_capacity(1000),
            tombstones: Vec::<i32>::with_capacity(1000),
        }
    }
}

impl ValuesContext {
    pub fn push(&mut self, value: Value) -> i32 {
        if let Some(idx) = self.tombstones.pop() {
            //eprintln!("[context] Pushing using tombstoned idx {}. Tombstones left: {}", idx, self.tombstones.len());
            self.owned_values[idx as usize] = Some(value);
            idx
        } else {
            let idx = self.owned_values.len();
            //eprintln!("[context] Pushing new idx: {}", idx);
            self.owned_values.push(Some(value));
            idx as i32
        }
    }

    pub fn take(&mut self, ptr: i32) -> Option<Value> {
        let value = self.owned_values[ptr as usize].take();
        self.tombstones.push(ptr);
        value
    }

    pub fn borrow(&self, ptr: i32) -> Option<&Value> {
        self.owned_values[ptr as usize].as_ref()
    }

    pub fn set(&mut self, ptr: i32, value: Value) -> &mut Self {
        //eprintln!("[context] Setting idx {} to {:?}", ptr, value);
        self.owned_values[ptr as usize] = Some(value);
        self
    }

    pub fn copy_into(&mut self, from_ptr: i32, to_ptr: i32) -> &mut Self {
        self.owned_values[to_ptr as usize] = self.owned_values[from_ptr as usize].clone();
        self
    }

    pub fn new_ptr(&mut self) -> i32 {
        if let Some(idx) = self.tombstones.pop() {
            //eprintln!("[context] New ptr using tombstoned idx {}. Tombstones left: {}", idx, self.tombstones.len());
            idx
        } else {
            let idx = self.owned_values.len();
            //eprintln!("[context] New ptr at new idx: {}", idx);
            self.owned_values.push(None);
            idx as i32
        }
    }

    pub fn drop(&mut self, ptr: i32) -> &mut Self {
        self.tombstones.push(ptr);
        self.owned_values[ptr as usize] = None;
        //eprintln!("[context] Dropped idx {}. Tombstones left: {}", ptr, self.tombstones.len());
        self
    }

    pub fn count(&self) -> usize {
        self.owned_values.len()
    }

    pub fn clear(&mut self) -> &mut Self {
        self.owned_values.clear();
        self.tombstones.clear();
        self
    }
}


