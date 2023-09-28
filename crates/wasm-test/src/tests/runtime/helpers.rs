use wasmtime::{Store, Config, Engine};

use crate::runtime::ClarityWasmContext;

/// Helper function. Initializes a clean new `Store` using defaults, but
/// with WASM reference types enabled.
pub fn get_new_store() -> Store<ClarityWasmContext> {
    let mut config = Config::default();
    config.wasm_reference_types(true);
    let engine = Engine::new(&config).expect("Failed to initialize Wasmtime Engine.");
    let context = ClarityWasmContext::new();
    Store::new(&engine, context)
}
