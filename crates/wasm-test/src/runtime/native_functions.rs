mod extref;
mod memory_serialize;
mod rustref;
mod rustref_stack;
mod wasm_native;

pub use extref::*;
pub use memory_serialize::*;
pub use rustref::*;
pub use rustref_stack::*;
pub use wasm_native::*;


// This module defines all of the Clarity native RUNTIME functions. The matching type definitions
// must be imported into modules IN THE SAME ORDER as their import order in the module. For example,
// if Walrus is used to generate the module, the type definitions must be imported in the same
// order as when imported into the Wasmtime module.

use wasmtime::{AsContextMut, Func};
use super::ClarityWasmContext;

/// Holds a native function name and function implementation.
#[derive(Debug)]
pub struct FuncMap {
    pub name: String,
    pub func: Func,
}

impl FuncMap {
    pub fn new(name: &str, func: Func) -> Self {
        FuncMap {
            name: name.to_string(),
            func,
        }
    }
}

#[inline]
pub fn get_all_functions(mut store: impl AsContextMut<Data = ClarityWasmContext>) -> Vec<FuncMap> {
    vec![
        // `add` functions
        FuncMap::new("add_extref", define_add_extref(&mut store)),
        FuncMap::new("add_native", define_add_native(&mut store)),
        FuncMap::new("add_memory", define_add_memory(&mut store)),
        FuncMap::new("add_rustref", define_add_rustref(&mut store)),
        FuncMap::new("define_add_rustref_stack", define_add_rustref_stack(&mut store)),
        // `mul` (multiplication) functions
        FuncMap::new("mul_extref", define_mul_extref(&mut store)),
        FuncMap::new("mul_rustref", define_mul_rustref(&mut store)),
        // `fold` functions
        FuncMap::new("fold_extref", define_fold_extref(&mut store)),
        FuncMap::new("fold_memory", define_fold_memory(&mut store)),
        FuncMap::new("fold_rustref", define_fold_rustref(&mut store)),
        // `drop_ptr` functions
        FuncMap::new("drop_ptr_rustref", define_drop_ptr_rustref(&mut store)),
    ]
}
