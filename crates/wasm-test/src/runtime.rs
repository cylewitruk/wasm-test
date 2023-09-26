pub(crate) mod alloc;
pub(crate) mod objpool;
pub mod stack;

pub mod native_functions;

pub use native_functions::get_all_functions;

use crate::Ptr;
use num::FromPrimitive;
use num_derive::{FromPrimitive, ToPrimitive};

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum RuntimeError {
    InvalidRuntimeError = -1,
    None = 0,
    FunctionArgumentRequired = 1,
    FailedToDeserializeValueFromMemory = 2,
    FailedToDiscernSerializedType = 3,
    FunctionOnlySupportsIntegralValues = 4,
    ArgumentTypeMismatch = 5,
    ArithmeticOverflow = 6,
    FailedToWriteResultToMemory = 7,
}

pub type FuncResultMemory = (i32, i32, i32);

pub trait FuncResultMemoryTrait {
    fn err(error: RuntimeError) -> FuncResultMemory {
        (error as i32, 0, 0)
    }
    fn ok(ptr: Ptr) -> FuncResultMemory {
        (0, ptr.offset_i32(), ptr.len_i32())
    }
    fn is_success(&self) -> bool;
    fn get_error(&self) -> RuntimeError;
}

impl FuncResultMemoryTrait for FuncResultMemory {
    fn is_success(&self) -> bool {
        self.0 == 0
    }

    fn get_error(&self) -> RuntimeError {
        let e = RuntimeError::from_i32(self.0);
        match e {
            Some(err) => err,
            None => RuntimeError::InvalidRuntimeError,
        }
    }
}
