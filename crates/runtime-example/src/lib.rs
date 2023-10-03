#[macro_use]
extern crate wasm_rustref;

host_functions!(host_functions => 
    // Arithmetic operations
    add, sub, div, mul,
    // Other
    fold
);

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use wasm_rustref::runtime::{ClarityWasmContext, Stack};
    use wasmtime::{Config, Engine, Store};

    #[test]
    fn test_wasmtime_imports() {
        // Setup.
        let mut store = new_store();

        // Generate imports vec for Wasmtime module instantiation.
        let wasmtime_imports = super::host_functions::get_wasmtime_imports(&mut store);

        // Assert that we've got four functions.
        assert_eq!(5, wasmtime_imports.len());
    }

    #[test]
    fn test_walrus_imports() {
        use walrus::*;

        // Create a new, default module.
        let mut module = Module::default();
        
        // Attempt to import our function definitions into the module.
        super::host_functions::import_into_walrus_module(&mut module);

        let import_count = module.imports.iter().count();
        assert_eq!(5, import_count);
    }

    /// Helper function for creating a new Wasmtime [Store].
    fn new_store() -> Store<ClarityWasmContext> {
        let config = Config::default();
        let engine = Engine::new(&config).expect("Failed to initialize engine");
        let stack = Rc::new(Stack::default());
        let data = ClarityWasmContext::new(Rc::clone(&stack));
        Store::new(&engine, data)
    }
}