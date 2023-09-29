use wasmtime::{Func, AsContextMut};

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

// /// Defines the `fold_native` function.
//#[inline]
//pub fn define_fold_native(mut store: impl AsContextMut<Data = ClarityWasmContext>) -> Func {
//    todo!()
//}