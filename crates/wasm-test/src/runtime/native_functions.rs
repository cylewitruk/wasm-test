// This module defines all of the Clarity native RUNTIME functions. The matching type definitions
// must be imported into modules IN THE SAME ORDER as their import order in the module. For example,
// if Walrus is used to generate the module, the type definitions must be imported in the same
// order as when imported into the Wasmtime module.

use crate::runtime::FuncResultTrait;
use crate::serialization::{
    deserialize_clarity_seq_to_ptrs, deserialize_clarity_value,
    get_type_indicator_from_serialized_value, serialize_clarity_value, TypeIndicator, HEADER_LEN,
};
use crate::ClarityWasmContext;
use clarity::vm::{
    types::{CharType, SequenceData},
    Value,
};
use wasmtime::{AsContext, AsContextMut, Caller, ExternRef, Func, Val};

use super::{FuncResult, RuntimeError};

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

/// Defines the `add_extref` function. This function makes full use of `ExternRef`s
/// instead of value types or memory, meaning that the values coming across are
/// pure references to real Clarity `Value` enum variants.
#[inline]
pub fn define_add_extref(mut store: impl AsContextMut) -> Func {
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

/// Defines the `add_native_int128` function. This function makes use of Wasm "native"
/// types for parameters and return values. As Wasm doesn't have support for 128-bit
/// integers, we must pass two sets of low/high i64's and return one set of high/low i64's.
#[inline]
pub fn define_add_native(mut store: impl AsContextMut) -> Func {
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
pub fn define_add_memory(mut store: impl AsContextMut<Data = ClarityWasmContext>) -> Func {
    Func::wrap(
        &mut store,
        |mut caller: Caller<'_, ClarityWasmContext>,
         a_ptr: i32,
         a_len: i32,
         b_ptr: i32,
         b_len: i32|
         -> FuncResult {
            // Retrieve an instance of the `vm_mem` exported memory.
            let memory = caller.get_export("vm_mem").unwrap().into_memory().unwrap();
            // Get a handle to a slice representing the in-memory data.
            let data = memory.data(&caller);

            // Fetch the bytes for `a` from memory.
            let a_bytes: [u8; 16] = data
                [(a_ptr + HEADER_LEN) as usize..(a_ptr + a_len - HEADER_LEN) as usize]
                .try_into()
                .map_err(|_| FuncResult::err(RuntimeError::FailedToDeserializeValueFromMemory))
                .unwrap();

            // Get the type of `a`.
            let a_ty = get_type_indicator_from_serialized_value(&a_bytes)
                .map_err(|_| FuncResult::err(RuntimeError::FailedToDiscernSerializedType))
                .unwrap();

            // Assert that `a` is an integral type.
            if !a_ty.is_integer() {
                return FuncResult::err(RuntimeError::FunctionOnlySupportsIntegralValues);
            }

            // Fetch the bytes for `b` from memory.
            let b_bytes: [u8; 16] = data
                [(b_ptr + HEADER_LEN) as usize..(b_ptr + b_len - HEADER_LEN) as usize]
                .try_into()
                .map_err(|_| FuncResult::err(RuntimeError::FailedToDeserializeValueFromMemory))
                .unwrap();

            // Get the type of `b`.
            let b_ty = get_type_indicator_from_serialized_value(&b_bytes)
                .map_err(|_| FuncResult::err(RuntimeError::FailedToDiscernSerializedType))
                .unwrap();

            // Assert that `b` is an integral type.
            if !b_ty.is_integer() {
                return FuncResult::err(RuntimeError::FunctionOnlySupportsIntegralValues);
            }

            // Assert that `a` and `b` are of the same type.
            if a_ty != b_ty {
                return FuncResult::err(RuntimeError::ArgumentTypeMismatch);
            }

            // Result buffer
            let mut result: [u8; 16] = [0; 16];

            if a_ty == TypeIndicator::Int {
                // Handle case for signed integers
                let a = i128::from_le_bytes(a_bytes);
                let b = i128::from_le_bytes(b_bytes);
                if let Some(add_result) = a.checked_add(b) {
                    result = add_result.to_le_bytes();
                } else {
                    return FuncResult::err(RuntimeError::ArithmeticOverflow);
                }
            } else if a_ty == TypeIndicator::UInt {
                // Handle case for unsigned integers
                let a = u128::from_le_bytes(a_bytes);
                let b = u128::from_le_bytes(b_bytes);
                if let Some(add_result) = a.checked_add(b) {
                    result = add_result.to_le_bytes();
                } else {
                    return FuncResult::err(RuntimeError::ArithmeticOverflow);
                }
            }

            // Retrieve a memory ptr for the result.
            let alloc = caller.data_mut().alloc.alloc_for_buffer(&result);

            // Write the result to memory
            memory
                .write(&mut caller, alloc.offset as usize, &result)
                .map_err(|_| FuncResult::err(RuntimeError::FailedToWriteResultToMemory))
                .unwrap();

            // Return
            FuncResult::ok(alloc)
        },
    )
}

/// Defines the `mul` (multiply) function.
#[inline]
pub fn define_mul_extref(mut store: impl AsContextMut) -> Func {
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

#[inline]
pub fn define_fold_memory(mut store: impl AsContextMut<Data = ClarityWasmContext>) -> Func {
    Func::wrap(
        &mut store,
        |mut caller: Caller<'_, ClarityWasmContext>,
         func: Option<Func>,
         seq_ptr: i32,
         seq_len: i32,
         init_ptr: i32,
         init_len: i32|
         -> FuncResult {
            // The function to fold over must be supplied.
            if func.is_none() {
                return FuncResult::err(RuntimeError::FunctionArgumentRequired);
            }

            // Retrieve an instance of the `vm_mem` exported memory.
            let memory = caller.get_export("vm_mem").unwrap().into_memory().unwrap();
            // Get a handle to a slice representing the in-memory data.
            let data = memory.data(&caller);
            // Extract the raw serialized sequence.
            let seq_data = &data[seq_ptr as usize..seq_len as usize];
            // Deserialize the sequence to a list of pointers to its values (we don't actually care about
            // the values in this function, so we don't need to deserialize them).
            let sequence_ptrs = deserialize_clarity_seq_to_ptrs(seq_data)
                .map_err(|_| FuncResult::err(RuntimeError::FailedToDeserializeValueFromMemory))
                .unwrap();

            // Grab our function to fold over.
            let func = func.unwrap();

            // We use the `init` value for the first round, and the result of the
            // function call for further rounds.
            let mut is_first = true;

            // We'll re-use the same result array to avoid re-allocations.
            let mut result = [Val::I32(0), Val::I32(0), Val::I32(0)];

            // Iterate through each of the (pointers-to) the values of the sequence and call the
            // provided function to fold over.
            for ptr in sequence_ptrs {
                if is_first {
                    func.call(
                        &mut caller,
                        &[
                            Val::I32(ptr.offset_i32()),
                            Val::I32(ptr.len_i32()),
                            Val::I32(init_ptr),
                            Val::I32(init_len),
                        ],
                        &mut result,
                    )
                    .unwrap();
                    is_first = false;
                } else {
                    func.call(
                        &mut caller,
                        &[
                            Val::I32(ptr.offset_i32()),
                            Val::I32(ptr.len_i32()),
                            result[1].clone(),
                            result[2].clone(),
                        ],
                        &mut result,
                    )
                    .unwrap();
                }
            }

            (0, result[1].unwrap_i32(), result[2].unwrap_i32())
        },
    )
}

/// Defines the `fold` function.
#[inline]
pub fn define_fold_extref(mut store: impl AsContextMut<Data = ClarityWasmContext>) -> Func {
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
                        let val_ref = Some(ExternRef::new(*val));
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
                                let val_ref = Some(ExternRef::new(*val));
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
    vec![
        // `add` functions
        FuncMap::new("add_extref", define_add_extref(&mut store)),
        FuncMap::new("add_native", define_add_native(&mut store)),
        FuncMap::new("add_memory", define_add_memory(&mut store)),
        // `mul` (multiplication) functions
        FuncMap::new("mul_extref", define_mul_extref(&mut store)),
        // `fold` functions
        FuncMap::new("fold_extref", define_fold_extref(&mut store)),
        FuncMap::new("fold_memory", define_fold_memory(&mut store)),
    ]
}
