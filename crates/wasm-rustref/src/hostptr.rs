use std::{fmt, ops::Deref};

use crate::{Stack, ValType};

/// The external pointer type exposed by [StackFrame] which can be
/// used to safely work with data behind the pointers.
#[derive(Debug, Clone, Copy)]
pub struct HostPtr<'a> {
    pub(crate) stack: &'a Stack,
    inner: i32,
    pub(crate) val_type: ValType,
    is_owned: bool,
}

/// Implementation of [HostPtr].
impl<'a> HostPtr<'a> {
    /// Instantiates a new [HostPtr] instance. Note that it is _critical_ that the
    /// `inner` parameter points to a valid index+reference in the backing [Vec].
    /// Failure to do so will almost certainly result in undefined behavior when trying to
    /// read back the [clarity::vm::Value].
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn new(stack: &'a Stack, inner: i32, val_type: ValType, is_owned: bool) -> Self {
        HostPtr {
            stack,
            inner,
            val_type,
            is_owned,
        }
    }

    /// Retrieve this [HostPtr] as a [usize].
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn as_usize(&self) -> usize {
        self.inner as usize
    }

    /// Gets whether or not this [HostPtr] is owned by the [Stack] or not. Owned
    /// values are generally only intermediate values (created and dropped during 
    /// execution) or return values, which move ownership to the caller at the 
    /// end of execution.
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn is_owned(&self) -> bool {
        self.is_owned
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

/// Implementation of [fmt::Display] for [HostPtr].
impl fmt::Display for HostPtr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ stack id: {}, ptr: {}, type: {:?} }}",
            self.stack.id, self.inner, self.val_type
        )
    }
}