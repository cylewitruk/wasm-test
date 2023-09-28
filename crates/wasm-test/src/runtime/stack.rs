use clarity::vm::Value;
use std::{cell::UnsafeCell, ops::Deref};

pub struct IndexTransition<T> {
    previous: T,
    next: T
}

impl<T> IndexTransition<T> {
    #[inline]
    pub fn new(previous: T, next: T) -> Self {
        Self { previous, next }
    }
}

/// Value type indicator, indicating the type of Clarity [Value] a given
/// [HostPtr] is pointing to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ValType {
    Int128,
    UInt128,
}

/// A simple trait to map a [Value] to a [ValType] with clean semantics.
pub trait AsValType {
    fn as_val_type(&self) -> ValType;
}

/// Implement [AsValType] for Clarity's [Value].
impl AsValType for Value {
    #[inline]
    fn as_val_type(&self) -> ValType {
        match self {
            Value::Int(_) => ValType::Int128,
            Value::UInt(_) => ValType::UInt128,
            _ => todo!(),
        }
    }
}

/// The external pointer type exposed by [StackFrame] which can be
/// used to safely work with data behind the pointers.
#[derive(Debug, Clone, Copy)]
pub struct HostPtr<'a> {
    stack: &'a Stack,
    inner: i32,
    val_type: ValType,
}

impl<'a> HostPtr<'a> {
    /// Instantiates a new [HostPtr] instance. Note that it is _critical_ that the
    /// `inner` parameter points to a valid index+reference in the backing [Vec]. 
    /// Failure to do so will almost certainly result in undefined behavior when trying to
    /// read back the [Value].
    #[inline]
    pub(crate) fn new(stack: &'a Stack, inner: i32, val_type: ValType) -> Self {
        HostPtr {
            stack,
            inner,
            val_type,
        }
    }

    /// Retrieve this [HostPtr] as a [usize].
    #[inline]
    pub(crate) fn as_usize(&self) -> usize {
        self.inner as usize
    }
}

/// i32 is probably the most commen cast, so we implement implicit deref from
/// [HostPtr] to [i32].
impl Deref for HostPtr<'_> {
    type Target = i32;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct FrameResult {}

/// A structure representing a virtual "Stack Frame". Not to be confused with
/// [StackFrame], which provides the public API for a [Stack]/[StackFrame]. This
/// structure maintains state regarding the frame in question.
#[derive(Debug, Clone)]
pub struct FrameContext {
    pub frame_index: usize,
    pub parent_frame_index: Option<usize>,
    pub lower_bound: usize,
}

/// Implementation of [FrameContext].
impl FrameContext {
    /// Instantiates a new [FrameContext] instance.
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

/// Implementation of the public API for a [StackFrame].
impl StackFrame<'_> {
    /// Pushes a new [Value] to the top of the [Stack] and returns a safe
    /// [HostPtr] pointer which can be used to retrieve the [Value] at
    /// a later time.
    #[inline]
    pub fn push(&self, value: Value) -> HostPtr {
        let (ptr, val_type) = unsafe { self.0.local_push(value) };
        HostPtr::new(self.0, ptr, val_type)
    }

    #[inline]
    pub fn push_unchecked(&self, value: Value) -> i32 {
        unsafe { self.0.local_push(value).0 }
    }

    /// Gets a value from this [Stack] using a previously received [HostPtr].
    /// 
    /// Note: The provided [HostPtr] can only be used to retrieve values from
    /// the same [Stack] which created it. Trying to pass a [HostPtr] created by
    /// another [Stack] instance will panic.
    #[inline]
    pub fn get(&self, ptr: HostPtr) -> Option<&Value> {
        assert_eq!(ptr.stack.id, self.0.id);
        unsafe { self.0.local_get(*ptr) }
    }

    /// Gets a value from this [Stack] by [i32] pointer.
    ///
    /// # Safety
    ///
    /// This function is unsafe because there are no checks that the [i32] pointer
    /// is:
    /// 1. A valid index in the backing [Vec]. If the index is out of bounds then
    /// an out-of-bounds panic will be thrown.
    /// 2. That a raw pointer held in the backing [Vec] is indeed pointing to the
    /// correct value.
    #[inline]
    pub unsafe fn get_unchecked(&self, ptr: i32) -> Option<&Value> {
        self.0.local_get(ptr)
    }

    #[inline]
    pub fn drop(&self, ptr: HostPtr) {
        self.0.local_drop(ptr)
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
        StackFrame(self.0)
    }
}

#[derive(Debug, Default)]
pub struct Stack {
    id: u64,
    current_local_idx: UnsafeCell<i32>,
    next_frame_idx: UnsafeCell<usize>,
    locals: UnsafeCell<Vec<*const Value>>,
    frames: UnsafeCell<Vec<FrameContext>>,
}

impl Stack {
    #[inline]
    pub fn new() -> Self {
        let mut stack = Self {
            id: rand::random::<u64>(),
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
        let (index, _) = self.increment_frame_index();
        //println!("[new_frame] index={}, next_index={}", index, next_index);

        // Create a new frame context, which stores a little bit of information
        // about the frame that we'll need later.
        let context = FrameContext::new(
            index,
            if index == 0 { None } else { Some(index - 1) },
            self.local_count(),
        );

        // Get a mutable reference to our frames vec and push our new context.
        (*self.frames.get()).push(context);

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
        //println!(
        //    "[drop_frame] index={}, dropped_frame_index={}, current_index={:?}",
        //    index, dropped_frame_index, current_index
        //);
        assert_eq!(index, dropped_frame_index, "Dropped frame index did not match the index we received.");

        // Get a mutable reference to our frames vec.
        let frames = &mut *self.frames.get();

        // Remove the dropped frame, getting the removed `FrameContext`.
        //println!("[drop_frame] dropped frame: ptr={:?}, value={:?}", index, frames[index]);
        let dropped_frame = frames.remove(dropped_frame_index);
        //println!("[drop_frame] dropped frame: ptr={:?}, value={:?}", dropped_frame, dropped_frame);

        // Set the Stack's current locals index to the lower bound of the dropped frame.
        // This is the state just before the dropped frame was created.
        (self.current_local_idx.get()).replace(dropped_frame.lower_bound as i32);
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
        let next_frame_index_ptr = self.next_frame_idx.get();
        let next_frame_index = *next_frame_index_ptr;
        let current_frame_index = next_frame_index - 1;
        let target_frame_index = if current_frame_index > 0 {
            current_frame_index - 1
        } else {
            1
        };

        //println!("[decrement_frame_index] current_frame_index={}, next_frame_idx (upper)={}, target_frame_index={}",
        //    current_frame_index, next_frame_index, target_frame_index);

        if target_frame_index == 0 {
            //println!("[decrement_frame_index] target frame is 0, resetting...");
            *next_frame_index_ptr = 1;
            return (1, None);
        }

        if current_frame_index > 0 {
            *next_frame_index_ptr -= 1;
            //println!("[decrement_frame_index] returning ({:?}, {:?})", current_frame_index, *next_frame_index_ptr);
            (current_frame_index, Some(*next_frame_index_ptr))
        } else {
            next_frame_index_ptr.replace(0);
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

            (*self.locals.get()).push(ptr);

            *current_idx += 1;
            (idx, val_type)
        }
    }

    #[inline]
    fn local_drop(&self, ptr: HostPtr) {
        unsafe {
            (&mut *self.locals.get())[*ptr as usize] = std::ptr::null();
        }
    }

    #[inline]
    unsafe fn local_get(&self, ptr: i32) -> Option<&Value> {
        unsafe {
            let raw_ptr = (*self.locals.get())[ptr as usize];

            if raw_ptr.is_null() {
                None
            } else {
                Some(&*raw_ptr)
            }
        }
    }

    /// Clears all of the locals in this [Stack].
    ///
    /// # Safety
    ///
    /// This function is unsafe because clearing the [Stack] while pointers are still
    /// held by frames would result in UB.
    #[inline]
    pub unsafe fn clear_locals(&self) {
        unsafe {
            (*self.locals.get()).clear();
        }
    }

    #[inline]
    pub fn local_count(&self) -> usize {
        unsafe { (*self.locals.get()).len() }
    }
}

#[cfg(test)]
#[allow(unused_variables)]
mod test {
    use crate::runtime::stack::ValType;
    use super::Stack;
    use clarity::vm::Value;

    /// Implement helper methods for testing.
    impl Stack {
        fn get_current_local_idx(&self) -> i32 {
            unsafe { *self.current_local_idx.get() }
        }

        fn get_next_frame_idx(&self) -> usize {
            unsafe { *self.next_frame_idx.get() }
        }
    }

    #[test]
    fn push_and_get_with_multiple_values_in_frame() {
        let stack = Stack::new();
        let mut results = Vec::<i32>::new();

        let _result = stack.exec(&mut results, |f| {
            f.push(Value::Int(1));
            f.push(Value::Int(2));
            f.push(Value::Int(3));
            f.push(Value::Int(4));
            f.push(Value::Int(5));
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

    #[test]
    fn stack_tip_is_correctly_adjusted_when_creating_and_dropping_a_frame() {
        let stack = Stack::new();

        let _result = stack.exec(&mut vec![], |f1| {
            let ptr1 = f1.push(Value::Int(1));
            assert_eq!(ValType::Int128, ptr1.val_type);

            let ptr2 = f1.push(Value::UInt(2));
            assert_eq!(ValType::UInt128, ptr2.val_type);
            
            assert_eq!(2, stack.get_current_local_idx());
            assert_eq!(1, stack.get_next_frame_idx());

            stack.exec(&mut vec![], |f2 |{
                assert_eq!(2, stack.get_current_local_idx());
                assert_eq!(2, stack.get_next_frame_idx());

                let val1_1 = f2.get(ptr1);
                let val1_2 = f2.get(ptr1);
                let val2 = f2.get(ptr2);

                let ptr3 = f2.push(Value::UInt(3));
                
                assert!(val1_1.is_some());
                assert!(val1_2.is_some());
                assert_eq!(&Value::Int(1), val1_1.unwrap());
                assert_eq!(val1_1, val1_2);
                assert_eq!(&Value::UInt(2), val2.unwrap());
                assert_eq!(3, stack.get_current_local_idx());

                vec![]
            });

            vec![]
        });
    }

    #[test]
    fn stack_rewound_to_last_frame_tip_when_dropped() {
        let stack = Stack::new();

        let _result = stack.exec(&mut vec![], |f1| {
            let ptr1 = f1.push(Value::Int(1));
            assert_eq!(1, stack.get_current_local_idx());
            assert_eq!(1, stack.get_next_frame_idx());

            stack.exec(&mut vec![], |f2| {
                let ptr2 = f2.push(Value::Int(2));
                assert_eq!(2, stack.get_current_local_idx());
                assert_eq!(2, stack.get_next_frame_idx());

                vec![]
            });

            assert_eq!(1, stack.get_next_frame_idx());
            assert_eq!(1, stack.get_current_local_idx());

            vec![]
        });
    }
}
