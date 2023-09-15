//use mimalloc::MiMalloc;

//#[global_allocator]
//static GLOBAL: MiMalloc = MiMalloc;

// Private modules

// Public modules
pub mod compiler;
pub mod runtime;
pub mod serialization;

// Public exports
pub use runtime::get_all_functions;

// Test-related
#[cfg(test)]
mod tests;

#[derive(Debug, Copy, Clone)]
pub struct ClarityWasmContext {}
