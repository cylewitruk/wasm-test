pub(crate) mod alloc;
pub(crate) mod native_functions;

pub use native_functions::get_all_functions;

#[repr(i32)]
pub enum RuntimeError {
    FunctionArgumentRequired = 1,
    FailedToDeserializeValueFromMemory = 2
}

pub type FuncResult = (i32, i32, i32);

pub trait FuncResultTrait {
    fn error(error: RuntimeError) -> FuncResult {
        (error as i32, 0, 0)
    }
}

impl FuncResultTrait for FuncResult {}