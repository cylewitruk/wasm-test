pub(crate) mod alloc;
pub(crate) mod native_functions;

pub use native_functions::get_all_functions;
use num::FromPrimitive;
use num_derive::{FromPrimitive, ToPrimitive};

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum RuntimeError {
    InvalidRuntimeError = -1,
    None = 0,
    FunctionArgumentRequired = 1,
    FailedToDeserializeValueFromMemory = 2,
}

pub type FuncResult = (i32, i32, i32);

pub trait FuncResultTrait {
    fn error(error: RuntimeError) -> FuncResult {
        (error as i32, 0, 0)
    }
    fn is_success(&self) -> bool;
    fn get_error(&self) -> RuntimeError;
}

impl FuncResultTrait for FuncResult {
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
