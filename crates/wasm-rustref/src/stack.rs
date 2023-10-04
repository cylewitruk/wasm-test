use clarity::vm::Value;
use core::fmt;
use std::cell::UnsafeCell;
use wasmtime::Store;

use crate::{ValType, ClarityWasmContext, HostPtr, AsValType, frames::{FrameContext, FrameResult}, StackFrame, AsFrame};

/// [Stack] is the core of this library and provides all of the core functionality
/// for managing locals and providing a context within host [wasmtime::Func]
/// execution for working with locals.  
/// 
/// Please _read the **Safety** section_ prior to using!!
/// 
/// # Safety
/// 
/// The implementation of [Stack] uses [UnsafeCell] and raw pointers for fast
/// internal mutability. It stores references to Clarity [Value]'s as raw pointers
/// and thus expects **YOU** to ensure that the [Value] references provided to
/// the [Stack] instance _are not moved during its use_.
/// 
/// In general this should not be an issue as [Value]s should not be mutated/moved
/// during the execution of a Clarity contract - but you have been warned anyway.
#[derive(Debug)]
pub struct Stack {
    pub(crate) id: u64,
    pub(crate) current_local_idx: UnsafeCell<i32>,
    next_frame_idx: UnsafeCell<usize>,
    pub(crate) locals: UnsafeCell<Vec<*const Value>>,
    pub(crate) frames: UnsafeCell<Vec<FrameContext>>,
    result_buffer: UnsafeCell<Vec<*const Value>>,
    owned_values: UnsafeCell<Vec<Value>>,
}

/// Implementation of [fmt::Display] for [Stack].
impl fmt::Display for Stack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            writeln!(f, "\n>>> Stack dump")?;
            writeln!(f, "----------------------------------------------")?;
            writeln!(f, "id: {}", self.id)?;
            writeln!(f, "locals count: {}", self.local_count())?;
            writeln!(f, "locals index: {}", *self.current_local_idx.get() - 1)?;
            writeln!(f, "locals: {:?}", *self.locals.get())?;
            writeln!(f, "frame count: {}", self.get_frame_index())?;
            writeln!(f, "frames: {:?}", *self.frames.get())?;
            writeln!(f, "----------------------------------------------")?;
        }

        Ok(())
    }
}

/// Implementation of [Default] for [Stack].
impl Default for Stack {
    fn default() -> Self {
        Self {
            id: rand::random::<u64>(),
            current_local_idx: Default::default(),
            next_frame_idx: Default::default(),
            locals: Default::default(),
            frames: Default::default(),
            result_buffer: Default::default(),
            owned_values: Default::default(),
        }
    }
}

/// Implementation of [Stack].
impl Stack {
    #[inline]
    pub fn new() -> Self {
        let mut stack = Self {
            id: rand::random::<u64>(),
            current_local_idx: UnsafeCell::new(0),
            next_frame_idx: UnsafeCell::new(0),
            locals: UnsafeCell::new(Vec::with_capacity(1000)),
            frames: UnsafeCell::new(Vec::with_capacity(100)),
            result_buffer: UnsafeCell::new(Vec::with_capacity(15)),
            owned_values: UnsafeCell::new(Vec::with_capacity(100)),
        };

        stack.locals.get_mut().fill(std::ptr::null());
        stack
    }

    #[inline]
    pub fn frame(
        stack: &Stack,
        mut store: Store<ClarityWasmContext>,
        mut func: impl FnMut(&mut Store<ClarityWasmContext>, &StackFrame) -> Vec<Value>,
    ) -> Store<ClarityWasmContext> {
        unsafe {
            // Create a new virtual frame.
            let (frame, context) = stack.new_frame();

            // Call the provided function.
            let frame_result: Vec<Value> = func(&mut store, &frame);

            // Move the output values from the frame to the result buffer.
            stack.fill_result_buffer(frame_result);

            // Drop the frame.
            stack.drop_frame(context.frame_index);
        }

        store
    }

    #[inline]
    pub fn exec(
        &self,
        // Added the for<> below just as a reminder in case we use lifetimes later
        func: impl Fn(StackFrame) -> Vec<Value>,
    ) -> FrameResult {
        unsafe {
            // Create a new virtual frame.
            let (frame, context) = self.new_frame();

            // Call the provided function.
            let frame_result: Vec<Value> = func(frame);
            
            // Move the output values from the frame to the result buffer.
            self.fill_result_buffer(frame_result);

            // Drop the frame.
            self.drop_frame(context.frame_index);
        }

        FrameResult {}
    }

    #[inline]
    pub fn exec2(
        self,
        mut store: Store<ClarityWasmContext>,
        // Added the for<> below just as a reminder in case we use lifetimes later
        mut func: impl FnMut(&mut Store<ClarityWasmContext>, &StackFrame) -> Vec<Value>,
    ) -> FrameResult {
        unsafe {
            // Create a new virtual frame.
            let (frame, context) = self.new_frame();

            // Call the provided function.
            let frame_result: Vec<Value> = func(&mut store, &frame);

            // Move the output values from the frame to the result buffer.
            self.fill_result_buffer(frame_result);

            // Drop the frame.
            self.drop_frame(context.frame_index);
        }

        FrameResult {}
    }

    /// Clears and fills this [Stack]'s result buffer with raw pointers to the values
    /// contained in the provided `results` [Vec], consuming it.
    #[inline]
    pub(crate) fn fill_result_buffer(&self, results: Vec<Value>) {
        unsafe {
            // TODO: Implement re-use of result buffer slots using index, returning a
            // slice to the results.
            let buffer = &mut *self.result_buffer.get();
            buffer.clear();
            for result in results {
                let result_ref = self.give_owned_value(result);
                buffer.push(result_ref as *const _)
            }
        }
    }

    /// Converts the current result buffer to a [Vec] of &[Value]s.
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn result_buffer_to_vec(&self) -> Vec<&Value> {
        unsafe {
            let buffer = &(*self.result_buffer.get());
            let mut values = Vec::<&Value>::with_capacity(buffer.len());
            for val in buffer.iter() {
                values.push(&**val);
            }
            values
        }
    }

    /// Creates a new virtual frame on the [Stack]. It is a virtual frame because it is
    /// fully backed by the [Stack] implementation and the "frame" keeps track of
    /// state via pointers and counters.
    #[inline]
    pub(crate) fn new_frame(&self) -> (StackFrame, &FrameContext) {
        // Retrieve the index for a new frame and increment the frame index.
        let (index, _) = unsafe { self.increment_frame_index() };

        // Create a new frame context, which stores a little bit of information
        // about the frame that we'll need later.
        let context = FrameContext::new(
            index,
            if index == 0 { None } else { Some(index - 1) },
            self.local_count(),
        );

        // Get a mutable reference to our frames vec and push our new context.
        unsafe { (*self.frames.get()).push(context) };
        // Get a reference to the context (because it was moved to Stack ownership above).
        let context_ref = unsafe { &(*self.frames.get())[index] };
        // Return our StackFrame and the reference to the FrameContext.
        (self.as_frame(), context_ref)
    }

    /// Drops the frame at the specified index. This results in the current frame index
    /// being decremented and the top of the stack becoming its ancestor frame. This
    /// function returns a [FrameContext] representing the frame at the top of the stack.
    #[inline]
    pub(crate) unsafe fn drop_frame(&self, index: usize) {
        // Decrement the frame index, receiving the dropped frame index (should match `index`)
        // and the index of the frame now at the top of the stack.
        let (dropped_frame_index, _) = self.decrement_frame_index();

        assert_eq!(
            index, dropped_frame_index,
            "Dropped frame index did not match the index we received."
        );

        // Get a mutable reference to our frames vec.
        let frames = &mut *self.frames.get();

        // Remove the dropped frame, getting the removed `FrameContext`.
        let dropped_frame = frames.remove(dropped_frame_index);

        // Set the Stack's current locals index to the lower bound of the dropped frame.
        // This is the state just before the dropped frame was created.
        (self.current_local_idx.get()).replace(dropped_frame.lower_bound as i32);
    }

    /// Returns the index of the current (top) frame in this [Stack].
    #[inline]
    pub(crate) fn get_frame_index(&self) -> usize {
        unsafe { *self.next_frame_idx.get() }
    }

    /// Increments the current frame index and returns a tuple of (`last_value`, `new_value`),
    /// where `last_value` is the index prior to the increment, and `new_value` is the index
    /// after the increment. This function is not meant to be called externally, it is used
    /// by `new_frame`.
    #[inline]
    pub(crate) unsafe fn increment_frame_index(&self) -> (usize, usize) {
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
    pub(crate) unsafe fn decrement_frame_index(&self) -> (usize, Option<usize>) {
        let next_frame_index_ptr = self.next_frame_idx.get();
        let next_frame_index = *next_frame_index_ptr;
        let current_frame_index = next_frame_index - 1;
        let target_frame_index = if current_frame_index > 0 {
            current_frame_index - 1
        } else {
            1
        };

        if target_frame_index == 0 {
            *next_frame_index_ptr = 1;
            return (1, None);
        }

        if current_frame_index > 0 {
            *next_frame_index_ptr -= 1;
            (current_frame_index, Some(*next_frame_index_ptr))
        } else {
            next_frame_index_ptr.replace(0);
            (0, None)
        }
    }

    /// Gives an owned [Value] to the [Stack] and returns a reference to the value.
    #[inline]
    pub(crate) fn give_owned_value(&self, value: Value) -> &Value {
        unsafe {
            (*self.owned_values.get()).push(value);
            let owned_values_len = (*self.owned_values.get()).len();
            &(*self.owned_values.get())[owned_values_len - 1]
        }
    }

    /// Pushes a value to the stack.
    #[inline]
    pub(crate) fn local_push(&self, value: &Value) -> (i32, ValType) {
        unsafe {
            let backing_vec_len = (*self.locals.get()).len();
            let current_idx = self.current_local_idx.get();
            let current_idx_usize = *current_idx as usize;

            let idx = *current_idx;
            let val_type = value.as_val_type();
            let ptr = value as *const _;

            if current_idx_usize < backing_vec_len {
                (*self.locals.get())[current_idx_usize] = ptr;
            } else {
                (*self.locals.get()).push(ptr);
            }

            *current_idx += 1;

            (idx, val_type)
        }
    }

    /// Drops the provided [HostPtr], freeing its value slot for use.
    #[inline]
    pub(crate) fn local_drop(&self, ptr: HostPtr) {
        unsafe {
            (&mut *self.locals.get())[*ptr as usize] = std::ptr::null();
        }
    }

    /// Attempts to retrieve a value given the provided [i32] pointer.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it naively attempts to access the backing [Vec]
    /// with the provided index, without checking that the index is within the [Vec]'s
    /// length.
    ///
    /// Additionally, if the index _is_ valid but the intended [Value] has been dropped,
    /// then you may not receive the [Value] you were expecting.
    #[inline]
    pub(crate) unsafe fn local_get(&self, ptr: i32) -> Option<&Value> {
        unsafe {
            let raw_ptr = (*self.locals.get())[ptr as usize];
            if raw_ptr.is_null() {
                None
            } else {
                Some(&*raw_ptr)
            }
        }
    }

    /// Attempts to retrieve a value given the provided [i32] pointer. If the provided pointer
    /// is not a valid index within the backing [Vec] this function will return [None].
    ///
    /// # Safety
    ///
    /// While this function is marked `unsafe`, it is perfectly safe to _call_.
    /// The reason for the function being `unsafe` is because there are no
    /// guarantees that the [Value] returned is the expected one.
    #[inline]
    pub(crate) unsafe fn local_try_get(&self, ptr: i32) -> Option<&Value> {
        unsafe {
            if ptr as usize >= (*self.locals.get()).len() {
                None
            } else {
                self.local_get(ptr)
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
        (*self.locals.get()).clear();
    }

    #[inline]
    pub fn local_count(&self) -> usize {
        unsafe { *self.current_local_idx.get() as usize }
    }
}

/// Implementations for [Stack] which are intended to be used by benchmarks etc.
/// We gate these behind the `bench` feature so that they have to be explicitly
/// enabled as they are not intended for public consumption and will almost
/// definitely crash and burn if used incorrectly.
#[cfg(any(feature = "bench", rust_analyzer))]
impl Stack {
    #[inline]
    pub fn _local_push(&self, value: &Value) -> (i32, ValType) {
        self.local_push(value)
    }

    #[inline]
    pub fn _new_frame(&self) -> (StackFrame, &FrameContext) {
        self.new_frame()
    }

    #[inline]
    pub unsafe fn _drop_frame(&self, index: usize) {
        self.drop_frame(index);
    }
}

#[cfg(test)]
#[allow(unused_variables)]
mod tests {
    use super::Stack;
    use crate::stack::ValType;
    use clarity::vm::Value;
    use log::*;

    /// Initialize logging if the `logging` feature is enabled.
    fn init_logging() {
        #[cfg(feature = "logging")]
        {
            let _ = env_logger::Builder::from_env(
                env_logger::Env::default().default_filter_or("wasm_rustref"),
            )
            .is_test(true)
            .try_init();
        }
    }

    /// Implement helper methods for testing.
    impl Stack {
        fn get_current_local_idx(&self) -> i32 {
            unsafe { *self.current_local_idx.get() }
        }

        fn get_next_frame_idx(&self) -> usize {
            unsafe { *self.next_frame_idx.get() }
        }

        pub fn get_locals_vec_len(&self) -> usize {
            unsafe { (*self.locals.get()).len() }
        }
    }

    #[test]
    fn new_frame_and_drop() {
        init_logging();
        let stack = Stack::new();

        let (frame, context) = stack.new_frame();
        assert_eq!(1, stack.get_next_frame_idx());

        unsafe { stack.drop_frame(context.frame_index) };
    }

    #[test]
    fn push_and_get_with_multiple_values_in_frame() {
        init_logging();
        let stack = Stack::new();

        let _result = stack.exec(|f| {
            f.push(&Value::Int(1));
            f.push(&Value::Int(2));
            f.push(&Value::Int(3));
            f.push(&Value::Int(4));
            f.push(&Value::Int(5));
            let ptr5 = f.push(&Value::UInt(11));
            f.push(&Value::UInt(12));
            f.push(&Value::UInt(13));
            f.push(&Value::Int(14));
            let ptr8 = f.push(&Value::Int(15));

            let val5 = f.get(ptr5);
            assert_eq!(true, val5.is_some());
            assert_eq!(&Value::UInt(11), val5.unwrap());

            let val8 = f.get(ptr8);
            assert_eq!(true, val8.is_some());
            assert_eq!(&Value::Int(15), val8.unwrap());

            trace!("val5: {:?}, val8: {:?}", val5, val8);

            vec![]
        });

        unsafe {
            trace!("heap locals: {:?}", *stack.locals.get());
        }
    }

    #[test]
    fn stack_tip_is_correctly_adjusted_when_creating_and_dropping_a_frame() {
        init_logging();
        let stack = Stack::new();

        let _result = stack.exec(|f1| {
            let ptr1 = f1.push(&Value::Int(1));
            assert_eq!(ValType::Int128, ptr1.val_type);

            let ptr2 = f1.push(&Value::UInt(2));
            assert_eq!(ValType::UInt128, ptr2.val_type);

            assert_eq!(2, stack.get_current_local_idx());
            assert_eq!(1, stack.get_next_frame_idx());

            stack.exec(|f2| {
                assert_eq!(2, stack.get_current_local_idx());
                assert_eq!(2, stack.get_next_frame_idx());

                let val1_1 = f2.get(ptr1);
                let val1_2 = f2.get(ptr1);
                let val2 = f2.get(ptr2);

                let ptr3 = f2.push(&Value::UInt(3));

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
        init_logging();
        let stack = Stack::new();

        let _result = stack.exec(|f1| {
            let ptr1 = f1.push(&Value::Int(1));
            assert_eq!(1, stack.get_current_local_idx());
            assert_eq!(1, stack.get_next_frame_idx());

            stack.exec(|f2| {
                let ptr2 = f2.push(&Value::Int(2));
                assert_eq!(2, stack.get_current_local_idx());
                assert_eq!(2, stack.get_next_frame_idx());

                vec![]
            });

            assert_eq!(1, stack.get_next_frame_idx());
            assert_eq!(1, stack.get_current_local_idx());

            vec![]
        });
    }

    #[test]
    fn test_stack_in_loop() {
        init_logging();
        let stack = Stack::new();

        let (a_ptr, _) = stack.local_push(&Value::Int(1024));
        let (b_ptr, _) = stack.local_push(&Value::Int(2048));
        assert_eq!(2, stack.local_count());

        (1..=5).into_iter().for_each(|i| {
            trace!("--------------------------------------------");
            trace!("Iteration #{i}");
            trace!("--------------------------------------------");
            assert_eq!(2, stack.get_current_local_idx());

            stack.exec(|frame| {
                let a = unsafe { frame.get_unchecked(a_ptr) };
                let b = unsafe { frame.get_unchecked(b_ptr) };
                trace!(
                    "[test] current_local_idx: {}",
                    stack.get_current_local_idx()
                );

                let result = match (a, b) {
                    (Some(Value::Int(a)), Some(Value::Int(b))) => Value::Int(a + b),
                    (Some(Value::UInt(a)), Some(Value::UInt(b))) => {
                        Value::UInt(a.checked_add(*b).unwrap())
                    }
                    _ => todo!("Add not implemented for given types"),
                };

                // Push an extra dummy value so we can make sure it gets properly dropped
                frame.push(&result.clone());
                trace!(
                    "[test] current_local_idx: {}",
                    stack.get_current_local_idx()
                );

                vec![result]
            });

            trace!(
                "[test] current_local_idx: {}, vec len: {}",
                stack.get_current_local_idx(),
                stack.get_locals_vec_len()
            );
            //assert_eq!(2, stack.local_count());
            assert_eq!(2, stack.get_current_local_idx());
        });
    }

    #[test]
    fn stack_new_sets_random_id() {
        let stack1 = Stack::new();
        let stack2 = Stack::new();

        assert!(stack1.id != 0 && stack2.id != 0);
        assert_ne!(stack1.id, stack2.id);
    }

    /// Assert that giving an owned value to a [Stack] works as expected, and
    /// that the [Value] is correct.
    #[test]
    fn test_give_owned_value() {
        let stack = Stack::new();

        let value = Value::Int(1);
        let value_clone = value.clone();
        stack.give_owned_value(value);

        unsafe {
            assert_eq!(1, (*stack.owned_values.get()).len());
            assert_eq!(value_clone, (*stack.owned_values.get())[0]);
        }
    }

    #[test]
    fn test_result_buffer() {
        let stack = Stack::new();

        let val1 = Value::Int(1);
        let val1_clone = val1.clone();
        let val2 = Value::Int(2);
        let result_buffer = vec![val1, val2];
        let result_buffer_expected = result_buffer.clone();

        stack.fill_result_buffer(result_buffer);
        let results = stack.result_buffer_to_vec();

        assert_eq!(&val1_clone, results[0]);
    }
}
