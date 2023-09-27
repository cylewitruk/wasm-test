use std::cell::UnsafeCell;
use clarity::vm::Value;

#[derive(Debug)]
pub struct Stack {
    current_local_idx: UnsafeCell<i32>,
    current_frame_idx: UnsafeCell<usize>,
    current_frame_lb: UnsafeCell<usize>,
    locals_heap: UnsafeCell<Vec<Option<Value>>>,
    tombstoned_ptrs: UnsafeCell<Vec<i32>>,
}

#[derive(Debug, Clone)]
pub struct StackFrame<'a>(&'a Stack);

impl StackFrame<'_> {
    #[inline]
    pub fn push(&self, value: Value) -> i32 {
        self.0.local_push(value)
    }

    #[inline]
    pub fn get(&self, ptr: i32) -> Option<&Value> {
        self.0.local_get(ptr)
    }

    #[inline]
    pub fn drop(&self, ptr: i32) {
        self.0.local_drop(ptr)
    }

    #[inline]
    pub fn clear(&self) {
        self.0.clear_locals()
    }
}

pub trait AsFrame {
    fn as_frame(&self) -> StackFrame;
}

impl AsFrame for Stack {
    #[inline]
    fn as_frame(&self) -> StackFrame {
        StackFrame(self)
    }
}

impl AsFrame for StackFrame<'_> {
    #[inline]
    fn as_frame(&self) -> StackFrame {
        StackFrame(&*self.0)
    }
}

impl Stack {
    #[inline]
    pub fn new() -> Self {
        let mut stack = Self {
            current_local_idx: UnsafeCell::new(0),
            current_frame_idx: UnsafeCell::new(0),
            current_frame_lb: UnsafeCell::new(0),
            locals_heap: UnsafeCell::new(Vec::with_capacity(1000)),
            tombstoned_ptrs: UnsafeCell::new(Vec::with_capacity(1000))
        };

        stack.locals_heap.get_mut().fill(None);

        stack
    }

    #[inline]
    pub fn exec(
        &self,
        results: &mut [i32],
        // Added the for<> below just as a reminder in case we use lifetimes later
        func: impl for<> Fn(StackFrame) -> Vec<i32>
    ) -> FrameResult {
        let frame = self.as_frame();
        func(frame);
        FrameResult {  }
    }

    #[inline]
    unsafe fn next_frame_id(&self) -> usize {
        let current_frame_id = self.current_frame_idx.get();
        let next_frame_id = *current_frame_id;
        *current_frame_id += 1;
        next_frame_id
    }

    #[inline]
    fn local_push(&self, value: Value) -> i32 {
        unsafe {
            let current_idx = self.current_local_idx.get();
            let idx = *current_idx;
            let heap_locals = &mut *self.locals_heap.get();
            heap_locals.push(Some(value));

            *current_idx += 1;
            idx
        }
    }

    #[inline]
    fn local_drop(&self, ptr: i32) {
        unsafe {
            (&mut *self.locals_heap.get())[ptr as usize] = None;
            //(&mut *self.tombstoned_ptrs.get()).push(ptr);
        }
    }

    #[inline]
    fn local_get(&self, ptr: i32) -> Option<&Value> {
        unsafe {
            (*self.locals_heap.get())[ptr as usize].as_ref()
        }
    }

    #[inline]
    pub fn clear_locals(&self) {
        unsafe {
            (&mut *self.locals_heap.get()).clear();
            //(&mut *self.tombstoned_ptrs.get()).clear();
        }
    }
}

pub trait FrameTrait {
    fn push(&self, value: Value) -> i32;
}

impl FrameTrait for Stack {
    fn push(&self, value: Value) -> i32 {
        self.local_push(value)
    }
}

pub struct FrameResult {

}

#[cfg(test)]
mod test {
    use clarity::vm::Value;
    use super::{Stack, FrameTrait, StackFrame, AsFrame};

    #[test]
    fn test() {
        let stack = Stack::new();
        let mut results = Vec::<i32>::new();

        let result = stack.exec(&mut results, |f| {
            f.push(Value::Int(1));
            f.push(Value::Int(2));
            f.push(Value::Int(3));
            f.push(Value::Int(4));
            f.push(Value::Int(5));
            /*f.frame(0, 0, |f2| {
                eprintln!("frame2: {:?}", f2);
                f2.local_push(Value::Int(6));
                f2.local_push(Value::Int(7));
                f2.local_push(Value::Int(8));
                f2.local_push(Value::Int(9));
                f2.local_push(Value::Int(10));
            });*/
            f.push(Value::UInt(11));
            f.push(Value::UInt(12));
            f.push(Value::UInt(13));
            f.push(Value::UInt(14));
            f.push(Value::UInt(15));

            // return dummy value
            Vec::default()
        });

        unsafe {
            eprintln!("heap locals: {:?}", *stack.locals_heap.get());
        }
        
    }
}