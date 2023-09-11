// Public modules
pub mod compiler;
pub mod runtime;
pub mod wasm_generator;

// Private modules
mod native_functions;

// Public exports
pub use native_functions::get_all_functions;

// Test-related
#[cfg(test)]
mod tests;

#[derive(Debug, Copy, Clone)]
pub struct ClarityWasmContext {}
