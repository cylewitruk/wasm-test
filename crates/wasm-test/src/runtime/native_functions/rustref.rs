use clarity::vm::{Value, types::SequenceData};
use wasmtime::{Func, AsContextMut, Caller, Val};

use crate::runtime::ClarityWasmContext;

/// Defines the `drop_ptr` function which allows Wasm to drop unused RustRef
/// pointers (such as intermediate results, etc.).
#[inline]
pub fn define_drop_ptr_rustref(mut store: impl AsContextMut<Data = ClarityWasmContext>) -> Func {
    Func::wrap(
        &mut store,
        |mut caller: Caller<'_, ClarityWasmContext>, ptr: i32| {
            //eprintln!("[drop_ptr] Dropping ptr: {}", ptr);
            caller.data_mut().values.drop(ptr);
        },
    )
}

#[inline]
pub fn define_add_rustref(mut store: impl AsContextMut<Data = ClarityWasmContext>) -> Func {
    Func::wrap(
        &mut store,
        |mut caller: Caller<'_, ClarityWasmContext>, a_ptr: i32, b_ptr: i32| -> i32 {
            let data = &mut caller.data_mut();
            let a = data.values.borrow(a_ptr).unwrap();
            let b = data.values.borrow(b_ptr).unwrap();

            let result = match (a, b) {
                (Value::Int(a), Value::Int(b)) => Value::Int(a + b),
                (Value::UInt(a), Value::UInt(b)) => Value::UInt(a.checked_add(*b).unwrap()),
                _ => todo!("Add not implemented for given types"),
            };

            data.values.push(result)
        },
    )
}

#[inline]
pub fn define_mul_rustref(mut store: impl AsContextMut<Data = ClarityWasmContext>) -> Func {
    Func::wrap(
        &mut store,
        |mut caller: Caller<'_, ClarityWasmContext>, a_ptr: i32, b_ptr: i32| -> i32 {
            //eprintln!("[mul] a_ptr: {}, b_ptr: {}", a_ptr, b_ptr);
            let data = caller.data_mut();
            let a = data.values.borrow(a_ptr).unwrap();
            let b = data.values.borrow(b_ptr).unwrap();

            let result = match (a, b) {
                (Value::Int(a), Value::Int(b)) => Value::Int(a.checked_mul(*b).unwrap()),
                (Value::UInt(a), Value::UInt(b)) => Value::UInt(a.checked_mul(*b).unwrap()),
                _ => todo!("Add not implemented for given types"),
            };

            data.values.push(result)
        },
    )
}

#[inline]
pub fn define_fold_rustref(mut store: impl AsContextMut<Data = ClarityWasmContext>) -> Func {
    Func::wrap(
        &mut store,
        |mut caller: Caller<'_, ClarityWasmContext>,
         func: Option<Func>,
         seq_ptr: i32,
         init_ptr: i32|
         -> i32 {
            //eprintln!("[fold] seq_ptr: {}, init_ptr: {}", seq_ptr, init_ptr);
            // Assert that we got a function to fold over.
            let func = func.expect("Fold function reference was not provided.");

            // This should be a pointer to a Clarity `Value::Sequence`.
            let seq = caller.data_mut().values.take(seq_ptr).unwrap();

            // Pre-allocate the results array for Wasmtime `call`. We will re-use this array for
            // each iteration.
            let results = &mut [Val::null()];

            // Create an empty pointer which we will re-use for each value in the iteration below
            let val_ptr = caller.data_mut().values.new_ptr();
            let mut last_result_ptr: Option<i32> = None;

            let result = match &seq {
                Value::Sequence(SequenceData::List(list)) => {
                    list.data.iter().fold(init_ptr, |acc_inner_ptr, val| {
                        caller.data_mut().values.set(val_ptr, val.to_owned());

                        func.call(
                            &mut caller,
                            &[Val::I32(val_ptr), Val::I32(acc_inner_ptr)],
                            results,
                        )
                        .expect("Failed to call fold inner function");

                        let result_ptr = results[0].unwrap_i32();
                        caller.data_mut().values.drop(acc_inner_ptr);
                        last_result_ptr = Some(result_ptr);

                        result_ptr
                    })
                }
                _ => panic!("Not a valid sequence type"),
            };

            // Drop any pointers which aren't needed any longer by this function and put
            // the `seq` and `init` values back.
            caller.data_mut().values.drop(val_ptr);

            // Our result will be the last accumulator value, so we return that pointer.
            result
        },
    )
}