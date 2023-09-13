// This module defines all of the Clarity native RUNTIME functions. The matching type definitions
// must be imported into modules IN THE SAME ORDER as their import order in the module. For example,
// if Walrus is used to generate the module, the type definitions must be imported in the same
// order as when imported into the Wasmtime module.

use std::io::Read;

use crate::ClarityWasmContext;
use clarity::vm::{
    types::{CharType, SequenceData},
    Value,
};
use wasmtime::{AsContext, AsContextMut, Caller, ExternRef, Func, Val};

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

/// Defines the `add` function.
#[inline]
pub fn define_add(mut store: impl AsContextMut) -> Func {
    Func::wrap(&mut store, |a: Option<ExternRef>, b: Option<ExternRef>| {
        let a = a.unwrap();
        let b = b.unwrap();

        let result = match a.data().downcast_ref::<Value>() {
            Some(Value::Int(int_a)) => {
                if let Some(Value::Int(int_b)) = b.data().downcast_ref::<Value>() {
                    let result = int_a.checked_add(*int_b).expect("Failed to add");
                    Some(ExternRef::new(Value::Int(result)))
                } else {
                    panic!(
                        "[add] Value type mismatch (int): b = {:?}",
                        b.data().downcast_ref::<Value>()
                    );
                }
            }
            Some(Value::UInt(uint_a)) => {
                if let Some(Value::UInt(uint_b)) = b.data().downcast_ref::<Value>() {
                    Some(ExternRef::new(Value::UInt(uint_a + uint_b)))
                } else {
                    panic!("Value type mismatch");
                }
            }
            _ => panic!("Invalid type..."),
        };

        Ok(result)
    })
}

#[inline]
pub fn define_add_native_int128(mut store: impl AsContextMut) -> Func {
    Func::wrap(
        &mut store,
        |a_low: i64, a_high: i64, b_low: i64, b_high: i64| {
            let a = ((a_high as u64) as u128) << 64 | ((a_low as u64) as u128);
            let b = ((b_high as u64) as u128) << 64 | ((b_low as u64) as u128);

            let result: i128 = a.checked_add(b).unwrap().try_into().unwrap();

            (
                (result & 0xFFFFFFFFFFFFFFFF) as i64,
                ((result >> 64) & 0xFFFFFFFFFFFFFFFF) as i64,
            )
        },
    )
}

#[inline]
pub fn define_add_native_int128_memory(mut store: impl AsContextMut<Data = ClarityWasmContext>) -> Func {
    Func::wrap(
        &mut store,
        |mut caller: Caller<'_, ClarityWasmContext>,
        a_ptr: i32,
        b_ptr: i32| -> i32 {
            let memory = caller
                .get_export("vm_mem")
                .unwrap()
                .into_memory()
                .unwrap();

            let mut a_buffer: [u8; 16] = [0; 16];
            let mut b_buffer: [u8; 16] = [0; 16];
            memory.read(&caller.as_context(), a_ptr as usize, &mut a_buffer)
                .expect("Failed to read memory for a_ptr.");
            memory.read(caller.as_context(), b_ptr as usize, &mut b_buffer)
                .expect("Failed to read memory for b_ptr.");

            let a = i128::from_le_bytes(a_buffer);
            let b = i128::from_le_bytes(b_buffer);

            let result = a.checked_add(b).expect("Failed to add two i128's");
            let result = result.to_le_bytes();
            memory.write(caller.as_context_mut(), 0, &result)
                .expect("Couldn't write result to memory");
            0
        },
    )
}

/// Defines the `mul` (multiply) function.
#[inline]
pub fn define_mul(mut store: impl AsContextMut) -> Func {
    Func::wrap(&mut store, |a: Option<ExternRef>, b: Option<ExternRef>| {
        let a = a.unwrap();
        let b = b.unwrap();

        let result = match a.data().downcast_ref::<Value>() {
            Some(Value::Int(int_a)) => {
                if let Some(Value::Int(int_b)) = b.data().downcast_ref::<Value>() {
                    let result = int_a.checked_mul(*int_b).expect("Failed to multiply");
                    Some(ExternRef::new(Value::Int(result)))
                } else {
                    panic!(
                        "[mul] Value type mismatch (int): b = {:?}",
                        b.data().downcast_ref::<Value>()
                    );
                }
            }
            Some(Value::UInt(uint_a)) => {
                if let Some(Value::UInt(uint_b)) = b.data().downcast_ref::<Value>() {
                    Some(ExternRef::new(Value::UInt(
                        uint_a.checked_mul(*uint_b).expect("Fail"),
                    )))
                } else {
                    panic!("Value type mismatch (uint)");
                }
            }
            _ => panic!("Invalid type..."),
        };

        Ok(result)
    })
}

/// Defines the `fold` function.
#[inline]
pub fn define_fold(mut store: impl AsContextMut<Data = ClarityWasmContext>) -> Func {
    Func::wrap(
        &mut store,
        |mut caller: Caller<'_, ClarityWasmContext>,
         func: Option<Func>,
         seq: Option<ExternRef>,
         init: Option<ExternRef>| {
            let func = func.unwrap();
            let seq = seq.unwrap();
            let init = init.unwrap();

            // Verify that the provided function to fold over has a compatible type signature
            // TODO: Verify against allowed types, if possible?
            let fn_type = func.ty(caller.as_context());
            debug_assert_eq!(fn_type.params().len(), 2);
            debug_assert_eq!(fn_type.results().len(), 1);

            // Define our output parameters to be used for each iteration of fold.
            let results = &mut [
                Val::ExternRef(Some(ExternRef::new(Value::none()))), // Option<ExternRef>
            ];

            // Iterate through each item in the provided sequence.
            let result = match seq.data().downcast_ref::<Value>().unwrap() {
                Value::Sequence(SequenceData::List(list)) => {
                    let result = list.data.iter().fold(init, |acc, val| {
                        let val_ref = Some(ExternRef::new(val.clone()));
                        // Call the provided function to fold over.
                        func.call(
                            &mut caller,
                            &[Val::ExternRef(val_ref), Val::ExternRef(Some(acc))],
                            results,
                        )
                        .expect("Failed to call fold inner function");

                        // TODO: Verify that the returned value is of the same type as `init`.
                        results[0].unwrap_externref().unwrap()
                    });
                    Some(result)
                }
                Value::Sequence(SequenceData::Buffer(buff)) => {
                    let result = buff.data.iter().fold(init, |acc, val| {
                        let val_ref = Some(ExternRef::new(val.clone()));
                        func.call(
                            &mut caller,
                            &[Val::ExternRef(val_ref), Val::ExternRef(Some(acc))],
                            results,
                        )
                        .expect("Failed to call fold inner function");

                        // TODO: Verify that the returned value is of the same type as `init`.
                        results[0].unwrap_externref().unwrap()
                    });
                    Some(result)
                }
                Value::Sequence(SequenceData::String(char_type)) => {
                    match char_type {
                        CharType::ASCII(str) => {
                            let result = str.data.iter().fold(init, |acc, val| {
                                let val_ref = Some(ExternRef::new(val.clone()));
                                func.call(
                                    &mut caller,
                                    &[Val::ExternRef(val_ref), Val::ExternRef(Some(acc))],
                                    results,
                                )
                                .expect("Failed to call fold inner function");

                                // TODO: Verify that the returned value is of the same type as `init`.
                                results[0].unwrap_externref().unwrap()
                            });
                            Some(result)
                        }
                        CharType::UTF8(str) => {
                            // TODO: This should probably be converted to i32 and compared from there (utf8 is 4 bytes)
                            let result = str.data.iter().fold(init, |acc, val| {
                                let val_ref = Some(ExternRef::new(val.clone()));
                                func.call(
                                    &mut caller,
                                    &[Val::ExternRef(val_ref), Val::ExternRef(Some(acc))],
                                    results,
                                )
                                .expect("Failed to call fold inner function");

                                // TODO: Verify that the returned value is of the same type as `init`.
                                results[0].unwrap_externref().unwrap()
                            });
                            Some(result)
                        }
                    }
                }
                _ => panic!("Not a valid sequence type"),
            };

            Ok(result)
        },
    )
}

#[inline]
pub fn get_all_functions(mut store: impl AsContextMut<Data = ClarityWasmContext>) -> Vec<FuncMap> {
    let mut funcs = Vec::<FuncMap>::new();

    // NOTE: `ExternRef`s and `FuncRef`s must be passed as `Option`s in Wasmtime to be properly
    // type converted by the runtime.

    funcs.push(FuncMap::new("add", define_add(&mut store)));
    funcs.push(FuncMap::new(
        "native_add_i128",
        define_add_native_int128(&mut store),
    ));
    funcs.push(FuncMap::new("mul", define_mul(&mut store)));
    funcs.push(FuncMap::new("fold", define_fold(&mut store)));

    funcs
}
