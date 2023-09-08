use clarity::vm::{types::{SequenceData, CharType}, Value};
use wasmtime::{
    AsContext, AsContextMut, Caller, Extern, ExternRef, Func, Val,
};

pub mod wasm_generator;

#[inline]
pub fn define_functions<T>(mut store: impl AsContextMut<Data = T>) -> Vec<Extern> {
    let mut externs = Vec::<Extern>::new();

    // NOTE: `ExternRef`s and `FuncRef`s must be passed as `Option`s in Wasmtime to be properly
    // type converted by the runtime.

    // This defines a HOST function which receives ExternRef values and adds them together, returning the result.
    let fn_add = Func::wrap(&mut store, |a: Option<ExternRef>, b: Option<ExternRef>| {
        let a = a.unwrap();
        let b = b.unwrap();

        let result = match a.data().downcast_ref::<Value>() {
            Some(Value::Int(int_a)) => {
                if let Some(Value::Int(int_b)) = b.data().downcast_ref::<Value>() {
                    let result = int_a.checked_add(*int_b).expect("Failed to add");
                    Some(ExternRef::new(Value::Int(result)))
                } else {
                    panic!("[add] Value type mismatch (int): b = {:?}", b.data().downcast_ref::<Value>());
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
    });

    // Define the `mul` (multiply) host function.
    let fn_mul = Func::wrap(&mut store, |a: Option<ExternRef>, b: Option<ExternRef>| {
        let a = a.unwrap();
        let b = b.unwrap();

        let result = match a.data().downcast_ref::<Value>() {
            Some(Value::Int(int_a)) => {
                if let Some(Value::Int(int_b)) = b.data().downcast_ref::<Value>() {
                    let result = int_a.checked_mul(*int_b).expect("Failed to multiply");
                    Some(ExternRef::new(Value::Int(result)))
                } else {
                    panic!("[mul] Value type mismatch (int): b = {:?}", b.data().downcast_ref::<Value>());
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
    });

    // Define the `fold` host function.
    let fn_fold = Func::wrap(
        &mut store,
        |mut caller: Caller<'_, T>,
         func: Option<Func>,
         seq: Option<ExternRef>,
         init: Option<ExternRef>| {
            let func = func.unwrap();
            let seq = seq.unwrap();
            let init = init.unwrap();

            let fn_type = func.ty(caller.as_context());
            debug_assert_eq!(fn_type.params().len(), 2);
            debug_assert_eq!(fn_type.results().len(), 1);

            // Define our output parameters to be used for each iteration of fold.
            let results = &mut [
                Val::ExternRef(Some(ExternRef::new(Value::none()))), // Option<ExternRef>
            ];

            let result = match seq.data().downcast_ref::<Value>().unwrap() {
                Value::Sequence(SequenceData::List(list)) => {
                    let result = list.data.iter().fold(init, |acc, val| {
                        let val_ref = Some(ExternRef::new(val.clone()));
                        func.call(
                            &mut caller, 
                            &[Val::ExternRef(val_ref), Val::ExternRef(Some(acc))], 
                            results
                        ).expect("Failed to call fold inner function");

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
                            results
                        ).expect("Failed to call fold inner function");

                        // TODO: Verify that the returned value is of the same type as `init`.
                        results[0].unwrap_externref().unwrap()
                    });
                    Some(result)
                }
                Value::Sequence(SequenceData::String(char_type)) => {
                    match char_type {
                        CharType::ASCII(str) =>  {
                            let result = str.data.iter().fold(init, |acc, val| {
                                let val_ref = Some(ExternRef::new(val.clone()));
                                func.call(
                                    &mut caller, 
                                    &[Val::ExternRef(val_ref), Val::ExternRef(Some(acc))], 
                                    results
                                ).expect("Failed to call fold inner function");
        
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
                                    results
                                ).expect("Failed to call fold inner function");
        
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
    );

    // Create `Extern`s for each of the functions and return the list.
    externs.append(&mut vec![
        Extern::Func(fn_add),
        Extern::Func(fn_mul),
        Extern::Func(fn_fold),
    ]);
    externs
}

#[derive(Debug, Copy, Clone)]
pub struct MyApplicationState {}