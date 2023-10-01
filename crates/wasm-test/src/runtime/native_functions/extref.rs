use clarity::vm::{
    types::{CharType, SequenceData},
    Value,
};
use wasmtime::{AsContext, AsContextMut, Caller, ExternRef, Func, Val};

use crate::runtime::ClarityWasmContext;

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

/// Defines the `fold_extref` function.
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
