/// Helper macro for determining the number of arguments provided to the macro
/// declaration. Once metavariable expressions are available in Rust stable then 
/// this can be replaced by `${length()}`.
#[macro_export]
macro_rules! count {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + count!($($xs)*));
}

/// Macro for creating the boilerplate implementation for a Wasmtime 
/// [`host function`](wasmtime::Func).
/// 
/// # Examples
/// ```
/// # #[macro_use]
/// # extern crate wasm_rustref;
/// 
/// host_function!(add => {
///     module = "clarity",
///     params = [a_ptr, b_ptr]
/// });
/// ```
/// This will generate the following boilerplate implementation for the `add`
/// host function:
/// ```
/// use $crate::paste::paste;
/// use $crate::{HostFunction, HostFunctionSignature};
/// 
/// /// The macro generates a marker struct with the `name` (first parameter)
/// /// provided in the macro declaration, however in PascalCase.
/// [derive(Clone, Copy, Default)]
/// pub struct Add {}
///
/// /// The macro will implement the [`HostFunction`](wasm_rustref::HostFunction)
/// /// trait for the generated marker struct, in this case [Add]. This trait
/// /// provides information about the host function signature and generates
/// /// methods both for generating imports for Wasmtime [`Instances`](wasmtime::Instance)
/// /// and Walrus [`Modules`](walrus::Module).
/// impl HostFunction for Add {
///     /// Gets the function signature based on the provided `params` argument
///     /// in the macro declaration.
///     fn signature() -> HostFunctionSignature
///     where
///         Self: Sized,
///     {
///         /// The [HostFunctionSignature] uses the `module` and `name` from the
///         /// macro declaration.
///         HostFunctionSignature::new("clarity", "add", 1usize + 1usize + 0usize, 1)
///     }
///
///     /// Function for generating a Wasmtime [`Func`](wasmtime::Func).
///     fn wasmtime_func(
///         mut store: impl wasmtime::AsContextMut<Data = ClarityWasmContext>,
///     ) -> wasmtime::Func
///     where
///         Self: 'static,
///     {
///         /// The Wasmtime Func definition is created using the user-provided
///         /// implementation of the [Exec] trait below.
///         wasmtime::Func::wrap(&mut store, Self::exec)
///     }
/// 
///     /// Function for importing the function signature into a Walrus [walrus::Module].
///     fn walrus_import(module: &mut walrus::Module) -> $crate::WalrusImportResult {
///         use walrus::ValType;
/// 
///         let sig = Self::signature();
///         let function_ty = module.types.add(
///             &[
///                 #[doc = "a_ptr"]
///                 ValType::I32,
///                 #[doc = "b_ptr"]
///                 ValType::I32,
///             ],
///             &[ValType::I32],
///         );
/// 
///         let (function_id, import_id) = module.add_import_func(&sig.module, &sig.name, function_ty);
/// 
///         $crate::WalrusImportResult {
///             import_id,
///             function_id,
///         }
///     }
/// }
/// 
/// /// The macro generates an [Exec] trait based on the `params` provided in the
/// /// macro declaration. 
/// 
/// /// **You must implement this trait for the generated marker
/// /// struct ([Add] in this case).**
/// trait Exec {
///     fn exec(
///         caller: wasmtime::Caller<'_, ClarityWasmContext>,
///         a_ptr: i32,
///         b_ptr: i32,
///     ) -> wasmtime::Result<()>;
/// }
/// ```
#[macro_export]
macro_rules! host_function {
    ($name:ident => { module = $module:literal, params = [$($args:ident),*] }) => ($crate::paste::paste! {
        use $crate::paste::paste;
        use $crate::{HostFunction, HostFunctionSignature};

        #[derive(Clone, Copy, Default)]
        pub struct [<$name:camel>] {}

        impl HostFunction for [<$name:camel>] {
            /// Gets the signature of this host function.
            fn signature() -> HostFunctionSignature where Self: Sized {
                HostFunctionSignature::new($module, stringify!($name), $crate::count!($($args)*), 1)
            }

            /// Function for generating the [wasmtime::Func] for this host function.
            fn wasmtime_func(mut store: impl wasmtime::AsContextMut<Data = ClarityWasmContext>) -> wasmtime::Func where Self: 'static {
                wasmtime::Func::wrap(
                    &mut store,
                    Self::exec
                )
            }

            /// Function for importing this host function signature into a Walrus
            /// module.
            fn walrus_import(module: &mut walrus::Module) -> $crate::WalrusImportResult {
                use walrus::ValType;

                let sig = Self::signature();

                let function_ty = module.types.add(
                    &[ $( #[doc = stringify!($args)] ValType::I32, )* ],
                    &[ValType::I32]
                );

                let (function_id, import_id) = module.add_import_func(&sig.module, &sig.name, function_ty);
                $crate::WalrusImportResult { import_id, function_id }
            }
        }

        /// Generated trait for this host function which must be implemented by
        /// the user.
        trait Exec {
            fn exec(caller: wasmtime::Caller<'_, ClarityWasmContext>, $($args: i32,)*) -> wasmtime::Result<()>;
        }
    });
}

#[macro_export]
macro_rules! host_functions {
    ($module_name:ident => $($func:ident),*) => ($crate::paste::paste! {
        pub(crate) mod $module_name {
            $( pub(crate) mod $func; )*

            /// Retrieves a [Vec] of [`Externs`](wasmtime::Extern) which can be used to
            /// import into a Wasmtime [`Instance`](wasmtime::Instance).
            /// 
            /// # Examples
            /// ```
            /// #[macro_use]
            /// extern crate wasm_rustref;
            /// 
            /// host_functions!(host_functions =>
            ///     // Arithmetic operations
            ///     add, sub, div, mul,
            ///     // Other
            ///     fold
            /// );
            /// 
            /// // This is the call to the generated function.
            /// let imports = super::host_functions::get_wasmtime_imports(&mut store);
            /// // Use the generated imports to create a new wasmtime Instance.
            /// let instance = Instance::new(&mut store, &module, &imports)?;
            /// ```
            pub fn get_wasmtime_imports(mut store: impl wasmtime::AsContextMut<Data = $crate::ClarityWasmContext>) -> Vec<wasmtime::Extern>  {
                use $crate::HostFunction;
                let mut ret: Vec<wasmtime::Extern> = Default::default();
                $( ret.push(wasmtime::Extern::Func($func :: [<$func:camel>] :: wasmtime_func(&mut store))); )*
                ret
            }

            pub fn import_into_walrus_module(module: &mut walrus::Module) -> Vec<$crate::WalrusImportResult> {
                use $crate::HostFunction;
                let mut import_results: Vec<$crate::WalrusImportResult> = Default::default();
                $(
                    import_results.push( $func :: [<$func:camel>] :: walrus_import(module) );
                )*
                import_results
            }
        }
    });
}
