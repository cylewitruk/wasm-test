#[macro_export]
macro_rules! count {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + count!($($xs)*));
}

#[macro_export]
macro_rules! ignore {
    ($x:ident) => {};
}

#[macro_export]
macro_rules! host_function {
    ($name:ident => { module = $module:literal, params = [$($args:ident),*] }) => ($crate::paste::paste! {
        use $crate::paste::paste;
        use $crate::{HostFunction, HostFunctionSignature};

        #[derive(Clone, Copy, Default)]
        pub struct [<$name:camel>] {}

        impl HostFunction for [<$name:camel>] {
            fn signature() -> HostFunctionSignature where Self: Sized {
                HostFunctionSignature::new($module, stringify!($name), $crate::count!($($args)*), 1)
            }

            fn wasmtime_func(mut store: impl wasmtime::AsContextMut<Data = ClarityWasmContext>) -> wasmtime::Func where Self: 'static {
                wasmtime::Func::wrap(
                    &mut store,
                    Self::exec
                )
            }

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
