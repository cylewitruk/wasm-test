pub(crate) mod alloc;
#[macro_use]
pub mod stack;
pub mod native_functions;

use clarity::vm::Value;
use wasmtime::{Caller, Store, AsContextMut};
use crate::ValuesContext;

pub use native_functions::get_all_functions;
pub use crate::runtime::stack::*;
pub use crate::runtime::alloc::WasmAllocator;

/// The state object which is available in all Wasmtime host function
/// calls. This is where information/structures which may be needed
/// across multiple executions should be placed.
///
/// Note: To receive a [ClarityWasmContext] in a host function you must
/// use one of the `wrap` variants which accepts a Wasmtime [Caller] as
/// the first argument. Once you have a caller, you get an instance to
/// the [ClarityWasmContext] by using `caller.data()` or `caller.data_mut()`.
#[derive(Debug, Default)]
pub struct ClarityWasmContext {
    pub alloc: WasmAllocator,
    pub values: ValuesContext,
    pub stack: Stack,
}

impl ClarityWasmContext {
    pub fn new() -> Self {
        Self {
            alloc: Default::default(),
            values: Default::default(),
            stack: Stack::new()
        }
    }
}

/// A trait which allows a consumer to receive an instance of a [Stack] from
/// an implementing structure.
pub trait AsStack {
    fn as_stack(&self) -> &Stack;
}


/// Implements [AsStack] for Wasmtime's [Caller] so that consumers of
/// `wrap` functions can easily receive an instance of this [ClarityWasmContext]'s
/// [Stack].
impl AsStack for Caller<'_, ClarityWasmContext> {
    #[inline]
    fn as_stack(&self) -> &Stack {
        &self.data().stack
    }
}

impl AsStack for Store<ClarityWasmContext> {
    #[inline]
    fn as_stack(&self) -> &Stack {
        &self.data().stack
    }
}


pub trait AsStoreExec<'a> {
    fn exec (&'a mut self,
        stack: &'a Stack,
        // Added the for<> below just as a reminder in case we use lifetimes later
        func: impl FnOnce(StackFrame, &'a mut Store<ClarityWasmContext>) -> Vec<Value>,
    );
}

impl<'a> AsStoreExec<'a> for Store<ClarityWasmContext> {
    fn exec (
        &'a mut self,
        stack: &'a Stack,
        // Added the for<> below just as a reminder in case we use lifetimes later
        func: impl FnOnce(StackFrame, &'a mut Store<ClarityWasmContext>) -> Vec<Value>,
    ) {
        unsafe {
            // Create a new virtual frame.
            let (frame, frame_index) = stack.new_frame();
            // Call the provided function.
            let mut frame_result: Vec<Value> = func(frame, self);
            #[cfg(test)] eprintln!("Frame result count: {}", frame_result.len());
            // Move the output values from the frame to the result buffer.
            stack.fill_result_buffer(frame_result);
            // Drop the frame.
            stack.drop_frame(frame_index);
        }
    }
}

pub trait AsCallerExec<'a> {
    fn exec (&'a mut self,
        stack: &'a Stack,
        // Added the for<> below just as a reminder in case we use lifetimes later
        func: impl FnOnce(StackFrame, &'a mut Caller<'a, ClarityWasmContext>) -> Vec<Value>,
    );
}

impl<'a> AsCallerExec<'a> for Caller<'a, ClarityWasmContext> {
    fn exec (&'a mut self,
        stack: &'a Stack,
        // Added the for<> below just as a reminder in case we use lifetimes later
        func: impl FnOnce(StackFrame, &'a mut Caller<'a, ClarityWasmContext>) -> Vec<Value>,
    ) {
        unsafe {
            // Create a new virtual frame.
            let (frame, frame_index) = stack.new_frame();
            // Call the provided function.
            let mut frame_result: Vec<Value> = func(frame, self);
            #[cfg(test)] eprintln!("Frame result count: {}", frame_result.len());
            // Move the output values from the frame to the result buffer.
            stack.fill_result_buffer(frame_result);
            // Drop the frame.
            stack.drop_frame(frame_index);
        }
    }
}

#[cfg(test)]
mod test {
    use walrus::ValType;
    use wasmtime::{Store, Engine, Config, AsContextMut, Extern, Instance, Module};

    use crate::get_all_functions;

    use super::{Stack, ClarityWasmContext, AsStoreExec};

    #[test]
    fn test_as_store_exec() {
        let stack = Stack::default();
        let config = Config::default();
        let engine = Engine::new(&config).unwrap();
        let data = ClarityWasmContext::default();
        let mut store = Store::new(&engine, data);

        // Convert the (name, func) pairs to a vec of `Export`s (needed for the Instance).
        let imports = vec![
            Extern::Func(crate::runtime::native_functions::define_add_rustref_stack(&mut store)),
        ];

        // Construct a new Walrus module.
        let walrus_config = walrus::ModuleConfig::new();
        let mut walrus_module = walrus::Module::with_config(walrus_config);

        // Import the API definition for `add_rustref_stack`.
        let add_rustref_stack_ty = walrus_module
            .types
            .add(&[ValType::I32, ValType::I32], &[ValType::I32]);

        walrus_module
            .add_import_func("clarity", "add_rustref_stack", add_rustref_stack_ty);

        // Compile the module.
        let wasm_bytes = walrus_module.emit_wasm();
        let module = Module::new(&engine, &wasm_bytes).expect("Failed to construct new module");
        let instance = Instance::new(&mut store, &module, &imports).expect("Couldn't create new module instance");

        let instance_fn = instance
            .get_func(store.as_context_mut(), "add_rustref_stack")
            .expect("Failed to get fn");

        

        store.exec(&stack, |_frame, _store| {
            let mut s = &mut _store.as_context_mut();
            instance_fn.call(s, &[], &mut []);

            vec![]
        });
    }
}