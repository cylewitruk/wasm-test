use std::rc::Rc;

use crate::runtime::{ClarityWasmContext, Stack};
use wasmtime::{Config, Engine, Store};

/// Helper function. Initializes a clean new `Store` using defaults, but
/// with WASM reference types enabled.
pub fn get_new_store<'a>(stack: Stack) -> Store<ClarityWasmContext> {
    let mut config = Config::default();
    config.wasm_reference_types(true);
    let engine = Engine::new(&config).expect("Failed to initialize Wasmtime Engine.");
    let context = ClarityWasmContext::new(Rc::new(stack));
    let store = Store::new(&engine, context);

    store
}
