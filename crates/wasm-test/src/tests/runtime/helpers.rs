use crate::runtime::{ClarityWasmContext, Stack};
use wasmtime::{Config, Engine, Store};

/// Helper function. Initializes a clean new `Store` using defaults, but
/// with WASM reference types enabled.
pub fn get_new_store<'a>() -> Store<ClarityWasmContext> {
    let mut config = Config::default();
    config.wasm_reference_types(true);
    let engine = Engine::new(&config).expect("Failed to initialize Wasmtime Engine.");
    let stack = Stack::new();
    let context = ClarityWasmContext::new();
    Store::new(&engine, context)
}
