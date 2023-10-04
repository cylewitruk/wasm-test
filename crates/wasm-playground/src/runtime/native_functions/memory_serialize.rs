use wasmtime::{AsContextMut, Caller, Func, Val};

use crate::{
    runtime::ClarityWasmContext,
    serialization::{
        deserialize_clarity_seq_to_ptrs, get_type_indicator_from_serialized_value,
        FuncResultMemory, FuncResultMemoryTrait, RuntimeError, TypeIndicator, HEADER_LEN,
    },
};

#[inline]
pub fn define_add_memory(mut store: impl AsContextMut<Data = ClarityWasmContext>) -> Func {
    Func::wrap(
        &mut store,
        |mut caller: Caller<'_, ClarityWasmContext>,
         a_ptr: i32,
         a_len: i32,
         b_ptr: i32,
         b_len: i32|
         -> FuncResultMemory {
            // Retrieve an instance of the `vm_mem` exported memory.
            let memory = caller.get_export("vm_mem").unwrap().into_memory().unwrap();
            // Get a handle to a slice representing the in-memory data.
            let data = memory.data(&caller);

            // Fetch the bytes for `a` from memory.
            let a_bytes: [u8; 16] = data
                [(a_ptr + HEADER_LEN) as usize..(a_ptr + HEADER_LEN + a_len - HEADER_LEN) as usize]
                .try_into()
                .map_err(|_| {
                    FuncResultMemory::err(RuntimeError::FailedToDeserializeValueFromMemory)
                })
                .unwrap();

            // Get the type of `a`.
            let a_ty = get_type_indicator_from_serialized_value(&a_bytes)
                .map_err(|_| FuncResultMemory::err(RuntimeError::FailedToDiscernSerializedType))
                .unwrap();

            // Assert that `a` is an integral type.
            if !a_ty.is_integer() {
                return FuncResultMemory::err(RuntimeError::FunctionOnlySupportsIntegralValues);
            }

            // Fetch the bytes for `b` from memory.
            let b_bytes: [u8; 16] = data
                [(b_ptr + HEADER_LEN) as usize..(b_ptr + HEADER_LEN + b_len - HEADER_LEN) as usize]
                .try_into()
                .map_err(|_| {
                    FuncResultMemory::err(RuntimeError::FailedToDeserializeValueFromMemory)
                })
                .unwrap();

            // Get the type of `b`.
            let b_ty = get_type_indicator_from_serialized_value(&b_bytes)
                .map_err(|_| FuncResultMemory::err(RuntimeError::FailedToDiscernSerializedType))
                .unwrap();

            // Assert that `b` is an integral type.
            if !b_ty.is_integer() {
                return FuncResultMemory::err(RuntimeError::FunctionOnlySupportsIntegralValues);
            }

            // Assert that `a` and `b` are of the same type.
            if a_ty != b_ty {
                return FuncResultMemory::err(RuntimeError::ArgumentTypeMismatch);
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
                    return FuncResultMemory::err(RuntimeError::ArithmeticOverflow);
                }
            } else if a_ty == TypeIndicator::UInt {
                // Handle case for unsigned integers
                let a = u128::from_le_bytes(a_bytes);
                let b = u128::from_le_bytes(b_bytes);
                if let Some(add_result) = a.checked_add(b) {
                    result = add_result.to_le_bytes();
                } else {
                    return FuncResultMemory::err(RuntimeError::ArithmeticOverflow);
                }
            }

            // Retrieve a memory ptr for the result.
            let alloc = caller.data_mut().alloc.alloc_for_buffer(&result);

            // Write the result to memory
            memory
                .write(&mut caller, alloc.offset as usize, &result)
                .map_err(|_| FuncResultMemory::err(RuntimeError::FailedToWriteResultToMemory))
                .unwrap();

            // Return
            FuncResultMemory::ok(alloc)
        },
    )
}

/// Defines the `fold_mem` function.
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
         -> FuncResultMemory {
            // The function to fold over must be supplied.
            if func.is_none() {
                return FuncResultMemory::err(RuntimeError::FunctionArgumentRequired);
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
                .map_err(|_| {
                    FuncResultMemory::err(RuntimeError::FailedToDeserializeValueFromMemory)
                })
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
