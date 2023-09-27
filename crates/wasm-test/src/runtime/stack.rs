use std::cell::UnsafeCell;
use clarity::vm::Value;

const STACK_OVERFLOW_THRESHOLD: i32 = 10;

#[derive(Debug)]
pub struct Stack {
    current_local_idx: UnsafeCell<i32>,
    locals_heap: UnsafeCell<Vec<Option<Value>>>,
    locals_stack: UnsafeCell<[Option<Value>; STACK_OVERFLOW_THRESHOLD as usize]>,
    current_frame_id: UnsafeCell<u32>,
    tombstoned_ptrs: UnsafeCell<Vec<i32>>,
}

pub struct StackFrame<'a>(&'a Stack);

impl StackFrame<'_> {
    pub fn push(&self, value: Value) -> i32 {
        self.0.local_push(value)
    }

    pub fn get(&self, ptr: i32) -> Option<&Value> {
        self.0.local_get(ptr)
    }

    pub fn drop(&self, ptr: i32) {
        self.0.local_drop(ptr)
    }

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
    fn as_frame(&self) -> StackFrame {
        StackFrame(&*self.0)
    }
}

impl Stack {
    #[inline]
    pub fn new() -> Self {
        let mut stack = Self {
            current_local_idx: UnsafeCell::new(0),
            locals_heap: UnsafeCell::new(Vec::with_capacity(1000)),
            locals_stack: UnsafeCell::new(Default::default()),
            current_frame_id: UnsafeCell::new(100000),
            tombstoned_ptrs: UnsafeCell::new(Vec::with_capacity(1000))
        };

        stack.locals_heap.get_mut().fill(None);

        stack
    }

    #[inline]
    pub fn exec(
        &self, 
        results: &mut [Value], // Exec returns the actual deref'd [`Value`]'s.
        func: impl Fn(Frame) -> &[i32] // The closure returns pointers.
    ) -> FrameResult {
        let frame = unsafe { 
            Frame::new(
                self, 
                self.next_frame_id(), 
                0, 
                0, 
                None) 
        };
        func(frame);
        
        FrameResult {  }
    }

    #[inline]
    pub fn exec2<'a>(
        &self,
        results: &mut [Value],
        // Added the for<> below just as a reminder in case we use lifetimes later
        func: impl for<> Fn(StackFrame) -> Vec<Value>
    ) -> FrameResult {
        let frame = self.as_frame();
        func(frame);
        FrameResult {  }
    }

    #[inline]
    unsafe fn next_frame_id(&self) -> u32 {
        let current_frame_id = self.current_frame_id.get();
        let next_frame_id = *current_frame_id;
        *current_frame_id += 1;
        next_frame_id
    }

    #[inline]
    fn tombstone<'a>(&self, frame: &'a Frame<'_>) {
        //println!("Tombstoning frame: {}", frame.id);
        //println!("This frame has {} pointers to clean up.", frame.pointers.borrow().len());

        unsafe {
            let locals = &mut *self.locals_heap.get();

            for ptr in (&*frame.pointers.get()).iter() {
                locals[*ptr as usize] = None;
            }

            //(&mut *self.tombstoned_ptrs.get())
            //    .append(&mut *frame.pointers.get());
        }
    }

    #[inline]
    fn local_push<'a>(&self, value: Value) -> i32 {
        unsafe {
            let current_idx = self.current_local_idx.get();
            let idx = *current_idx;

            if *current_idx < STACK_OVERFLOW_THRESHOLD {
                let stack_locals = &mut *self.locals_stack.get();
                stack_locals[idx as usize] = Some(value);
            } else {
                let heap_locals = &mut *self.locals_heap.get();
                heap_locals.push(Some(value));
            };

            *current_idx += 1;
            idx
        }
    }

    #[inline]
    fn local_drop(&self, ptr: i32) {
        unsafe {
            if ptr < STACK_OVERFLOW_THRESHOLD {
                (&mut *self.locals_stack.get())[ptr as usize] = None;
            } else {
                (&mut *self.locals_heap.get())[ptr as usize] = None;
            }
            //(&mut *self.tombstoned_ptrs.get()).push(ptr);
        }
    }

    #[inline]
    fn local_get(&self, ptr: i32) -> Option<&Value> {
        unsafe {
            if ptr < STACK_OVERFLOW_THRESHOLD {
                (*self.locals_stack.get())[ptr as usize].as_ref()
            } else {
                (*self.locals_heap.get())[ptr as usize].as_ref()
            }
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

#[derive(Debug)]
pub struct Frame<'a> {
    stack: &'a Stack,
    id: u32,
    arg_count: u8,
    result_count: u8,
    ancestor: Option<&'a Frame<'a>>,
    pointers: UnsafeCell<Vec<i32>>,
    
}

impl<'a> Frame<'_> {
    #[inline]
    pub fn new(
        stack: &'a Stack,
        id: u32,
        arg_count: u8,
        result_count: u8,
        ancestor: Option<&'a Frame<'a>>
    ) -> Frame<'a> {
        Frame {
            stack,
            id,
            arg_count,
            result_count,
            ancestor: ancestor.map(|frame| frame),
            pointers: Default::default()
        }
    }

    #[inline]
    pub fn frame(&'a self, arg_count: u8, result_count: u8, func: impl Fn(Frame)) -> FrameResult {
        let frame = unsafe { 
            Frame::new(
                self.stack,
                self.stack.next_frame_id(),
                arg_count,
                result_count,
                Some(self)
            ) 
        };

        func(frame);
        FrameResult {  }
    }

    #[inline]
    pub fn local_push(&self, value: Value) -> i32 {
        let ptr = self.stack.local_push(value);
        //unsafe { 
        //    (&mut *self.pointers.get()).push(ptr)
        //};
        ptr
    }

    #[inline]
    pub fn local_get(&self, ptr: i32) -> Option<&Value> {
        self.stack.local_get(ptr)
    }
}

impl<'a> Drop for Frame<'_> {
    #[inline]
    fn drop(&mut self) {
       //self.stack.tombstone(self);
    }
}

pub struct FrameResult {

}

macro_rules! push {
    ($frame:ident, $value:expr) => {
        $frame.local_push($value);
    };
}

macro_rules! get {
    ($frame:ident, $ptr:literal) => {
        $frame.local_get($ptr);
    }
}

#[cfg(test)]
mod test {
    use clarity::vm::Value;
    use super::{Stack, FrameTrait, StackFrame, AsFrame};

    #[test]
    fn test() {
        let stack = Stack::new();
        let mut results = Vec::<Value>::new();

        stack.exec2(&mut results, |f: StackFrame| {
            f.push(Value::Int(5000));
            Vec::default()
        });

        let result = stack.exec(&mut results, |f| {
            f.local_push(Value::Int(1));
            f.local_push(Value::Int(2));
            f.local_push(Value::Int(3));
            f.local_push(Value::Int(4));
            f.local_push(Value::Int(5));
            let ptr = push!(f, Value::UInt(100));
            let x = get!(f, 5);
            eprintln!("x: {:?}", x);
            eprintln!("frame1: {:?}", f);
            f.frame(0, 0, |f2| {
                eprintln!("frame2: {:?}", f2);
                f2.local_push(Value::Int(6));
                f2.local_push(Value::Int(7));
                f2.local_push(Value::Int(8));
                f2.local_push(Value::Int(9));
                f2.local_push(Value::Int(10));
            });
            f.local_push(Value::UInt(11));
            f.local_push(Value::UInt(12));
            f.local_push(Value::UInt(13));
            f.local_push(Value::UInt(14));
            f.local_push(Value::UInt(15));
            &[1, 2]
        });

        unsafe {
            eprintln!("stack locals: {:?}", *stack.locals_stack.get());
            eprintln!("heap locals: {:?}", *stack.locals_heap.get());
        }
        
    }
}