use std::fmt;

use clarity::vm::Value;

use crate::{HostPtr, Stack};

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
    pub fn push(&self, value: &Value) -> HostPtr {
        let (ptr, val_type) = self.0.local_push(value);
        HostPtr::new(self.0, ptr, val_type, false)
    }

    /// Pushes a new [Value] to the top of the [Stack] and returns an _unsafe_
    /// pointer (as an [i32]). The value can later be retrieved using the
    /// [get_unchecked](StackFrame::get_unchecked) function. Note that while
    /// this function is safe, retrieving a value using an [i32] pointer is **not**.
    #[inline]
    pub fn push_unchecked(&self, value: &Value) -> i32 {
        self.0.local_push(value).0
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

    /// Gets a [Value]] from this [Stack] by [i32] pointer.
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
        debug!(
            "[get_unchecked] calling into stack to retrieve value for ptr {}",
            ptr
        );
        self.0.local_get(ptr)
    }

    /// Attempts to get a [Value] from this [Stack] by [i32] pointer. If an invalid
    /// pointer is provided then [None] will be returned, otherwise **a** [Value]
    /// will be returned.
    ///
    /// # Safety
    ///
    /// Please be aware that while this function is not marked as `unsafe`, there
    /// are _no guarantees_ that you will get the [Value] you want here unless you
    /// _know for a fact_ that the [Value] has not been dropped or moved.
    #[inline]
    pub fn try_get(&self, ptr: i32) -> Option<&Value> {
        assert_eq!(ptr as u64, self.0.id);
        unsafe { self.0.local_try_get(ptr) }
    }

    /// Drops the specified [HostPtr].
    #[inline]
    pub fn drop(&self, ptr: HostPtr) {
        self.0.local_drop(ptr)
    }
}

/// Implementation of [fmt::Display] for [StackFrame].
impl std::fmt::Display for StackFrame<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            writeln!(f, "\n>>> Stack dump")?;
            writeln!(f, "----------------------------------------------")?;
            writeln!(f, "id: {}", self.0.id)?;
            writeln!(f, "locals count: {}", self.0.local_count())?;
            writeln!(f, "locals index: {}", *self.0.current_local_idx.get() - 1)?;
            writeln!(f, "locals: {:?}", *self.0.locals.get())?;
            writeln!(f, "frame count: {}", self.0.get_frame_index())?;
            writeln!(f, "frames: {:?}", *self.0.frames.get())?;
            writeln!(f, "----------------------------------------------\n")?;
        }

        Ok(())
    }
}

/// Defines functionality for receiving a [StackFrame] from another object.
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