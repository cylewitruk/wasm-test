mod macros;
mod stack;
mod hostptr;
mod frames;

#[macro_use]
pub extern crate log;
#[allow(unused_imports)]
#[macro_use]
pub extern crate paste;

use std::rc::Rc;
use wasmtime::{AsContextMut, Caller, Func, Store};
use clarity::vm::Value;

pub use stack::Stack;
pub use hostptr::HostPtr;
pub use frames::{AsFrame, StackFrame};


/// Value type indicator, indicating the type of Clarity [Value] a given
/// [HostPtr] is pointing to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ValType {
    Int128,
    UInt128,
}

/// A simple trait to map a [Value] to a [ValType] with clean semantics.
pub trait AsValType {
    fn as_val_type(&self) -> ValType;
}

/// Implement [AsValType] for Clarity's [Value].
impl AsValType for Value {
    #[inline]
    fn as_val_type(&self) -> ValType {
        match self {
            Value::Int(_) => ValType::Int128,
            Value::UInt(_) => ValType::UInt128,
            _ => todo!(),
        }
    }
}

pub trait HostFunction {
    fn signature() -> HostFunctionSignature
    where
        Self: Sized;
    fn wasmtime_func(store: impl AsContextMut<Data = ClarityWasmContext>) -> Func
    where
        Self: 'static;
    fn walrus_import(module: &mut walrus::Module) -> WalrusImportResult;
}

pub struct WalrusImportResult {
    pub import_id: walrus::ImportId,
    pub function_id: walrus::FunctionId,
}

pub struct HostFunctionSignature {
    pub module: String,
    pub name: String,
    pub param_count: usize,
    pub result_count: usize,
}

impl HostFunctionSignature {
    pub fn new(module: &str, name: &str, param_count: usize, result_count: usize) -> Self {
        HostFunctionSignature {
            module: module.to_string(),
            name: name.to_string(),
            param_count,
            result_count,
        }
    }
}

/// The state object which is available in all Wasmtime host function
/// calls. This is where information/structures which may be needed
/// across multiple executions should be placed.
///
/// Note: To receive a [ClarityWasmContext] in a host function you must
/// use one of the `wrap` variants which accepts a Wasmtime [Caller] as
/// the first argument. Once you have a caller, you get an instance to
/// the [ClarityWasmContext] by using `caller.data()` or `caller.data_mut()`.
#[derive(Debug)]
pub struct ClarityWasmContext {
    pub stack: Rc<Stack>,
}

impl ClarityWasmContext {
    #[inline]
    pub fn new(stack: Rc<Stack>) -> Self {
        Self { stack }
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

/// Implements [AsStack] for Wasmtime's [Store].
impl AsStack for Store<ClarityWasmContext> {
    #[inline]
    fn as_stack(&self) -> &Stack {
        &self.data().stack
    }
}

/// Defines functionality enabling a user to execute in a [StackFrame] directly
/// from a Wasmtime [Store]. This method is meant to be used when the [Stack]
/// is externally owned.
pub trait AsStoreExec<'a> {
    fn exec(
        &'a mut self,
        stack: Rc<Stack>,
        func: impl FnOnce(StackFrame, &'a mut Store<ClarityWasmContext>) -> Vec<Value>,
    );
}

impl<'a> AsStoreExec<'a> for Store<ClarityWasmContext> {
    #[inline]
    fn exec(
        &'a mut self,
        stack: Rc<Stack>,
        func: impl FnOnce(StackFrame, &'a mut Store<ClarityWasmContext>) -> Vec<Value>,
    ) {
        unsafe {
            // Create a new virtual frame.
            let (frame, frame_index) = stack.new_frame();
            // Call the provided function.
            let frame_result: Vec<Value> = func(frame, self);
            debug!("Frame result count: {}", &frame_result.len());
            debug!("Frame results: {:?}", &frame_result);
            // Move the output values from the frame to the result buffer.
            stack.fill_result_buffer(frame_result);
            // Drop the frame.
            stack.drop_frame(frame_index);
        }
    }
}

pub trait AsCallerExec<'a> {
    fn exec(
        &'a mut self,
        stack: &'a Stack,
        func: impl FnOnce(StackFrame, &'a mut Caller<'a, ClarityWasmContext>) -> Vec<Value>,
    );
}

impl<'a> AsCallerExec<'a> for Caller<'a, ClarityWasmContext> {
    #[inline]
    fn exec(
        &'a mut self,
        stack: &'a Stack,
        func: impl FnOnce(StackFrame, &'a mut Caller<'a, ClarityWasmContext>) -> Vec<Value>,
    ) {
        unsafe {
            // Create a new virtual frame.
            let (frame, frame_index) = stack.new_frame();
            // Call the provided function.
            let frame_result: Vec<Value> = func(frame, self);
            debug!("Frame result count: {}", frame_result.len());
            debug!("Frame results: {:?}", &frame_result);
            // Move the output values from the frame to the result buffer.
            stack.fill_result_buffer(frame_result);
            // Drop the frame.
            stack.drop_frame(frame_index);
        }
    }
}

#[cfg(test)]
mod test {
    use log::*;
    use std::rc::Rc;

    use crate::{AsStack, StackFrame};

    use super::{AsStoreExec, ClarityWasmContext, Stack};
    use clarity::vm::Value;
    use walrus::{FunctionBuilder, ValType};
    use wasmtime::{
        AsContextMut, Caller, Config, Engine, Extern, Func, Instance, Module, Store, Val,
    };

    #[test]
    fn test_as_store_exec() {
        let stack = Stack::default();
        let stack_rc = Rc::new(stack);
        let config = Config::default();
        let engine = Engine::new(&config).unwrap();
        let data = ClarityWasmContext::new(Rc::clone(&stack_rc));
        let mut store = Store::new(&engine, data);

        // Convert the (name, func) pairs to a vec of `Export`s (needed for the Instance).
        let imports = vec![Extern::Func(define_add_rustref_stack(&mut store))];

        // Construct a new Walrus module.
        let walrus_config = walrus::ModuleConfig::new();
        let mut walrus_module = walrus::Module::with_config(walrus_config);

        // Import the API definition for `add_rustref_stack`.
        let add_rustref_stack_ty = walrus_module
            .types
            .add(&[ValType::I32, ValType::I32], &[ValType::I32]);

        let (function_id, _) =
            walrus_module.add_import_func("clarity", "add_rustref_stack", add_rustref_stack_ty);

        // Define the Wasm test function.
        let mut add_rustref_stack_test_fn = FunctionBuilder::new(
            &mut walrus_module.types,
            &[ValType::I32, ValType::I32], // list + init
            &[ValType::I32],
        );

        let a = walrus_module.locals.add(ValType::I32);
        let b = walrus_module.locals.add(ValType::I32);

        add_rustref_stack_test_fn
            .func_body()
            .local_get(a)
            .local_get(b)
            .call(function_id);

        let add_rustref_test_id =
            add_rustref_stack_test_fn.finish(vec![a, b], &mut walrus_module.funcs);
        walrus_module
            .exports
            .add("add_rustref_stack_test", add_rustref_test_id);

        // Compile the module.
        let wasm_bytes = walrus_module.emit_wasm();
        let module = Module::new(&engine, &wasm_bytes).expect("Failed to construct new module");
        let instance = Instance::new(&mut store, &module, &imports)
            .expect("Couldn't create new module instance");

        let instance_fn = instance
            .get_func(&mut store, "add_rustref_stack_test")
            .expect("Failed to get fn");

        for x in 0..5 {
            trace!("\n\n[test] >>>> ITERATION {x}\n");

            store.exec(Rc::clone(&stack_rc), |frame, store| {
                let ptr1 = frame.push(&Value::Int(1024));
                let ptr2 = frame.push(&Value::Int(2048));

                trace!("[test] calling function");
                let results = &mut [Val::null()];

                instance_fn
                    .call(store, &[Val::I32(*ptr1), Val::I32(*ptr2)], results)
                    .map_err(|e| panic!("[test] error: {:?}", e))
                    .expect("failed to call function.");

                trace!("[test] call result: {:?}", results);

                vec![]
            });
        }
    }

    fn define_add_rustref_stack(mut store: impl AsContextMut<Data = ClarityWasmContext>) -> Func {
        Func::wrap(
            &mut store,
            #[inline]
            |caller: Caller<'_, ClarityWasmContext>, a_ptr: i32, b_ptr: i32| -> i32 {
                caller.as_stack().exec(|frame: StackFrame<'_>| {
                    let a = unsafe { frame.get_unchecked(a_ptr) };
                    let b = unsafe { frame.get_unchecked(b_ptr) };

                    let result = match (a, b) {
                        (Some(Value::Int(a)), Some(Value::Int(b))) => {
                            Value::Int(a.checked_add(*b).unwrap())
                        }
                        (Some(Value::UInt(a)), Some(Value::UInt(b))) => {
                            Value::UInt(a.checked_add(*b).unwrap())
                        }
                        _ => todo!("Add not implemented for given types"),
                    };

                    vec![result]
                });
                5
            },
        )
    }
}
