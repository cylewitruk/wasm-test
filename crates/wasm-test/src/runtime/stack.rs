use clarity::vm::Value;
use std::{cell::UnsafeCell, ops::Deref};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ValType {
    Int128,
    UInt128
}

pub trait AsValType {
    fn as_val_type(&self) -> ValType;
}

impl AsValType for Value {
    fn as_val_type(&self) -> ValType {
        match self {
            Value::Int(_) => ValType::Int128,
            Value::UInt(_) => ValType::UInt128,
            _ => todo!()
        }
    }
}

/// The external pointer type exposed by [StackFrame] which can be
/// used to safely work with data behind the pointers.
#[derive(Debug, Clone, Copy)]
pub struct HostPtr {
    inner: i32,
    val_type: ValType
}

impl HostPtr {
    pub(crate) fn new(inner: i32, val_type: ValType) -> Self {
        HostPtr { inner , val_type }
    }

    pub(crate) fn as_usize(&self) -> usize {
        self.inner as usize
    }
}

impl Deref for HostPtr {
    type Target = i32;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct FrameResult {}

#[derive(Debug, Clone)]
pub struct FrameContext {
    pub frame_index: usize,
    pub parent_frame_index: Option<usize>,
    pub lower_bound: usize,
}

impl FrameContext {
    #[inline]
    pub fn new(frame_index: usize, parent_frame_index: Option<usize>, lower_bound: usize) -> Self {
        Self {
            frame_index,
            parent_frame_index,
            lower_bound,
        }
    }
}

/// A helper class which provides the "public" API towards consumers of
/// [Stack]'s `exec` API, as a number of [Stack] methods are unsafe and
/// can result in UB if used incorrectly.
#[derive(Debug, Clone)]
pub struct StackFrame<'a>(&'a Stack);

/// Implementation of the public methods for a [StackFrame].
impl StackFrame<'_> {
    #[inline]
    pub fn push(&self, value: Value) -> HostPtr {
        let (ptr, val_type) = unsafe {
            self.0.local_push(value)
        };
        HostPtr::new(ptr, val_type)
    }

    #[inline]
    pub fn push_unchecked(&self, value: Value) -> i32 {
        unsafe {
            self.0.local_push(value).0
        }
    }

    #[inline]
    pub fn get(&self, ptr: HostPtr) -> Option<&Value> {
        unsafe {
            self.0.local_get(*ptr)
        }
    }

    #[inline]
    pub unsafe fn get_unchecked(&self, ptr: i32) -> Option<&Value> {
        self.0.local_get(ptr)
    }

    #[inline]
    pub fn drop(&self, ptr: HostPtr) {
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

#[derive(Debug)]
pub struct Stack {
    current_local_idx: UnsafeCell<i32>,
    next_frame_idx: UnsafeCell<usize>,
    locals: UnsafeCell<Vec<*const Value>>,
    frames: UnsafeCell<Vec<*const FrameContext>>,
}

impl Stack {
    #[inline]
    pub fn new() -> Self {
        let mut stack = Self {
            current_local_idx: UnsafeCell::new(0),
            next_frame_idx: UnsafeCell::new(0),
            locals: UnsafeCell::new(Vec::with_capacity(1000)),
            frames: UnsafeCell::new(Vec::with_capacity(100)),
        };

        stack.locals.get_mut().fill(std::ptr::null());
        stack
    }

    #[inline]
    pub fn exec(
        &self,
        results: &mut Vec<i32>,
        // Added the for<> below just as a reminder in case we use lifetimes later
        func: impl Fn(StackFrame) -> Vec<i32>,
    ) -> FrameResult {
        unsafe {
            let (frame, frame_index) = self.new_frame();
            func(frame);
            self.drop_frame(frame_index);
        }

        results.push(0);
        FrameResult {}
    }

    /// Creates a new virtual frame on the [Stack]. It is a virtual frame because it is
    /// fully backed by the [Stack] implementation and the "frame" keeps track of
    /// state via pointers and counters.
    #[inline]
    unsafe fn new_frame(&self) -> (StackFrame, usize) {
        // Retrieve the index for a new frame and increment the frame index.
        //println!("[new_frame] pre-increment={}", &*self.next_frame_idx.get());
        let (index, next_index) = self.increment_frame_index();
        //println!("[new_frame] index={}, next_index={}", index, next_index);

        // Create a new frame context, which stores a little bit of information
        // about the frame that we'll need later.
        let context = FrameContext::new(
            index,
            if index == 0 { None } else { Some(index - 1) },
            self.local_count(),
        );

        // Get a mutable reference to our frames vec and push our new context.
        (&mut *self.frames.get()).push(&context as *const _);

        (self.as_frame(), index)
    }

    /// Drops the frame at the specified index. This results in the current frame index
    /// being decremented and the top of the stack becoming its ancestor frame. This
    /// function returns a [FrameContext] representing the frame at the top of the stack.
    #[inline]
    unsafe fn drop_frame(&self, index: usize) {
        // Decrement the frame index, receiving the dropped frame index (should match `index`)
        // and the index of the frame now at the top of the stack.
        let (dropped_frame_index, current_index) = self.decrement_frame_index();
        /*eprintln!(
            "[drop frame] index={}, dropped_frame_index={}, current_index={:?}",
            index, dropped_frame_index, current_index
        );*/
        assert_eq!(index, dropped_frame_index);

        // Get a mutable reference to our frames vec.
        let frames = &mut *self.frames.get();

        // Remove the dropped frame, getting the removed `FrameContext`.
        let dropped_frame = frames.remove(dropped_frame_index);

        // Set the Stack's current locals index to the lower bound of the dropped frame.
        // This is the state just before the dropped frame was created.
        (self.current_local_idx.get()).replace(((*dropped_frame).lower_bound) as i32);
    }

    /// Returns the index of the current (top) frame in this [Stack].
    #[inline]
    unsafe fn get_frame_index(&self) -> usize {
        *self.next_frame_idx.get()
    }

    /// Increments the current frame index and returns a tuple of (`last_value`, `new_value`),
    /// where `last_value` is the index prior to the increment, and `new_value` is the index
    /// after the increment. This function is not meant to be called externally, it is used
    /// by `new_frame`.
    #[inline]
    unsafe fn increment_frame_index(&self) -> (usize, usize) {
        let ptr = self.next_frame_idx.get();
        let current = *ptr;
        *ptr += 1;
        (current, *ptr)
    }

    /// Decrements the current frame index and returns a tuple of (`last_value`, `new_value`),
    /// where `last_value` is the index prior to the decrement, and `new_value` is the index
    /// after the decrement. This function is not meant to be called externally, it is used
    /// by `drop_frame` to remove frames after they have returned.
    #[inline]
    unsafe fn decrement_frame_index(&self) -> (usize, Option<usize>) {
        let ptr = self.next_frame_idx.get();
        let next_index = *ptr;
        //println!("[decrement_frame_index] next_frame_idx={next_index}");

        if next_index > 1 {
            let current_index = next_index - 1;
            *ptr -= 1;
            (current_index, Some(*ptr))
        } else {
            ptr.replace(0);
            (0, None)
        }
    }

    /// Pushes a value to the stack.
    #[inline]
    unsafe fn local_push(&self, value: Value) -> (i32, ValType) {
        unsafe {
            let current_idx = self.current_local_idx.get();
            let idx = *current_idx;
            let val_type = value.as_val_type();
            let ptr = &value as *const Value;
            //println!("[local_push] index={}, value={:?}, ptr={:?}", idx, &value, ptr);

            (&mut *self.locals.get())
                .push(ptr);

            *current_idx += 1;
            (idx, val_type)
        }
    }

    #[inline]
    fn local_drop(&self, ptr: HostPtr) {
        unsafe {
            (&mut *self.locals.get())[*ptr as usize] = std::ptr::null();
            //(&mut *self.tombstoned_ptrs.get()).push(ptr);
        }
    }

    #[inline]
    unsafe fn local_get(&self, ptr: i32) -> Option<&Value> {
        unsafe { 
            let raw_ptr = (*self.locals.get())[ptr as usize];
            
            if raw_ptr == std::ptr::null() {
                None
            } else {
                Some(&*raw_ptr)
            }
        }
    }

    #[inline]
    pub fn clear_locals(&self) {
        unsafe {
            (&mut *self.locals.get()).clear();
            //(&mut *self.tombstoned_ptrs.get()).clear();
        }
    }

    #[inline]
    pub fn local_count(&self) -> usize {
        unsafe { (&mut *self.locals.get()).len() }
    }
}

#[cfg(test)]
mod test {
    use super::Stack;
    use clarity::vm::Value;

    #[test]
    fn test() {
        let stack = Stack::new();
        let mut results = Vec::<i32>::new();

        let _result = stack.exec(&mut results, |f| {
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
            let ptr5 = f.push(Value::UInt(11));
            f.push(Value::UInt(12));
            f.push(Value::UInt(13));
            f.push(Value::Int(14));
            let ptr8 = f.push(Value::Int(15));

            let val5 = f.get(ptr5);
            assert_eq!(true, val5.is_some());
            assert_eq!(&Value::UInt(11), val5.unwrap());

            let val8 = f.get(ptr8);
            assert_eq!(true, val8.is_some());
            assert_eq!(&Value::Int(15), val8.unwrap());

            println!("val5: {:?}, val8: {:?}", val5, val8);

            // return dummy value
            Vec::default()
        });

        unsafe {
            eprintln!("heap locals: {:?}", *stack.locals.get());
        }
    }
}
