use clarity::vm::Value;
use criterion::{criterion_group, criterion_main, Criterion};
use walrus::FunctionId;
use wasm_test::{get_all_functions, serialization::serialize_clarity_value, ClarityWasmContext};
use wasmtime::{Config, Engine, Extern, ExternRef, Instance, Module, Store, Val};

/// Helper struct to store mappings between a function name andits module import id and function id.
#[derive(Debug, Clone)]
struct WasmFunctionMapping {
    name: String,
    function_id: FunctionId,
}

/// Implementation of `ImportedFunctionMapping` to provide a
/// simple constructor.
impl WasmFunctionMapping {
    pub fn new_import(name: &str, function_id: FunctionId) -> Self {
        WasmFunctionMapping {
            name: name.to_string(),
            function_id,
        }
    }

    pub fn new_export(name: &str, function_id: FunctionId) -> Self {
        WasmFunctionMapping {
            name: name.to_string(),
            function_id,
        }
    }
}

/// Trait used to extend slices containing function mappings with
/// useful functions.
trait GetImportedFunctionByName {
    fn get_by_name(&self, name: &str) -> Option<WasmFunctionMapping>;
}

/// Implement the `GetImportedFunctionByName` for slices of
/// `WasmFunctionMapping`.
impl GetImportedFunctionByName for &[WasmFunctionMapping] {
    fn get_by_name(&self, name: &str) -> Option<WasmFunctionMapping> {
        for func in self.iter() {
            if func.name == name {
                return Some(func.clone());
            }
        }
        None
    }
}

pub fn criterion_benchmark(c: &mut Criterion) {
    // Generate a wasm module (see `wasm_generator.rs`) which has a `toplevel` function
    // which in turn calls the below defined wrapped function `func`.
    let wasm_bytes = generate_wasm();

    // Initialize config which allows for reference types.
    let mut config = Config::new();
    config.wasm_reference_types(true);

    Engine::tls_eager_initialize();

    // Initialize the wasmtime engine.
    let engine = Engine::new(&config).expect("Failed to initialize engine");

    // Pre-compile the module.
    let precompiled = engine
        .precompile_module(&wasm_bytes)
        .expect("Failed to precompile module");

    // Initialize the wasmtime store (using a custom state type).
    let state = ClarityWasmContext::new();
    let mut store = Store::new(&engine, state);

    // Load the module generated above.
    //let module = Module::from_binary(store.engine(), &wasm_bytes).expect("Failed to load module");
    let module =
        unsafe { Module::deserialize(&engine, &precompiled).expect("Failed to load module") };

    // Get our list of host functions to be included in the instance.
    let native_fns = get_all_functions(&mut store);
    // Convert the (name, func) pairs to a vec of `Export`s (needed for the Instance).
    let imports = native_fns
        .iter()
        .map(|f| Extern::Func(f.func))
        .collect::<Vec<Extern>>();

    // We create a new instance and pass in any imported (host) functions.
    let instance =
        Instance::new(&mut store, &module, &imports).expect("Couldn't create new module instance");

    // ================================================================================
    // FOLD-ADD-SQUARE benchmark
    // ================================================================================
    c.bench_function("fold-add-square", |b| {
        let instance_fn = instance
            .get_func(&mut store, "fold_add_square_extref_test")
            .expect("Failed to get fn");

        // Define our output parameters. Note that we're using `Option`s as stated above.
        let results = &mut [
            Val::ExternRef(Some(ExternRef::new(Value::none()))), // Option<ExternRef>
        ];

        b.iter(|| {
            // Define our input parameters.
            let mut sequence_values = Vec::<Value>::with_capacity(8192);
            for i in 1..8193 {
                sequence_values.push(Value::Int(i));
            }

            let sequence = Value::list_from(sequence_values).expect("Failed to create list");
            let init = Value::Int(1);

            instance_fn
                .call(
                    &mut store,
                    &[
                        Val::ExternRef(Some(ExternRef::new(sequence))), // Option<ExternRef>
                        Val::ExternRef(Some(ExternRef::new(init))),     // Option<ExternRef>
                    ],
                    results,
                )
                .expect("Failed to call function")
        })
    });

    let mut add_group = c.benchmark_group("Add");

    // ================================================================================
    // ADD using EXTERNREF benchmark
    // ================================================================================
    add_group.bench_function("add_externref", |b| {
        let instance_fn = instance
            .get_func(&mut store, "add_extref_test")
            .expect("Failed to get fn");

        // Define our output parameters. Note that we're using `Option`s as stated above.
        let results = &mut [
            Val::ExternRef(Some(ExternRef::new(Value::none()))), // Option<ExternRef>
        ];

        b.iter(|| {
            instance_fn
                .call(
                    &mut store,
                    &[
                        Val::ExternRef(Some(ExternRef::new(Value::Int(1024)))),
                        Val::ExternRef(Some(ExternRef::new(Value::Int(2048)))),
                    ],
                    results,
                )
                .expect("Failed to call function")
        });

        let result = results[0].unwrap_externref().unwrap();
        let result = result.data().downcast_ref::<Value>().unwrap();
        assert_eq!(*result, Value::Int(3072));
    });

    // ================================================================================
    // ADD using native Wasm types benchmark
    // ================================================================================
    add_group.bench_function("add_native", |b| {
        let instance_fn = instance
            .get_func(&mut store, "add_native_test")
            .expect("Failed to get fn");

        // Define our output parameters. Note that we're using `Option`s as stated above.
        let results = &mut [Val::I64(0), Val::I64(0)];

        b.iter(|| {
            instance_fn
                .call(
                    &mut store,
                    &[Val::I64(1), Val::I64(0), Val::I64(2), Val::I64(0)],
                    results,
                )
                .expect("Failed to call function")
        });

        assert_eq!(results[0].i64(), Some(3));
        assert_eq!(results[1].i64(), Some(0));
    });

    // ================================================================================
    // ADD using memory + serialization benchmark
    // ================================================================================
    add_group.bench_function("add_memory", |b| {
        let instance_fn = instance
            .get_func(&mut store, "add_memory_test")
            .expect("Failed to get fn");

        // Define our output parameters. Note that we're using `Option`s as stated above.
        let results = &mut [Val::I32(0), Val::I32(0), Val::I32(0)];

        // Retrieve the Wasm memory.
        let mem = instance
            .get_memory(&mut store, "vm_mem")
            .expect("Failed to find 'vm_mem'.");

        // Define the two Clarity values we want to add.
        let a_val = Value::Int(5);
        let b_val = Value::Int(11);

        // Serialize the two values we want to add to their byte-representations.
        let a_bytes = serialize_clarity_value(&a_val).expect("Failed to serialize 'a'");
        let b_bytes = serialize_clarity_value(&b_val).expect("Failed to serialize 'b'");

        // Get pointers to both a and b slices.
        let a_ptr = store.data_mut().alloc.alloc_for_buffer(&a_bytes);
        let b_ptr = store.data_mut().alloc.alloc_for_buffer(&b_bytes);

        mem.write(&mut store, a_ptr.offset as usize, &a_bytes)
            .expect("Failed to write buffer for 'a'");
        mem.write(&mut store, b_ptr.offset as usize, &b_bytes)
            .expect("Failed to write buffer for 'b'");

        b.iter(|| {
            instance_fn
                .call(
                    &mut store,
                    &[
                        Val::I32(a_ptr.offset),
                        Val::I32(a_ptr.len),
                        Val::I32(b_ptr.offset),
                        Val::I32(b_ptr.len),
                    ],
                    results,
                )
                .expect("Failed to call function")
        });
    });

    add_group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

#[inline]
pub fn generate_wasm() -> Vec<u8> {
    use walrus::{ExportItem, Module, ModuleConfig};

    // Construct a new Walrus module.
    let config = ModuleConfig::new();
    let mut module = Module::with_config(config);

    let mut funcs = vec![
        // Define imported functions
        define_add_extref(&mut module),
        define_add_native(&mut module),
        define_add_memory(&mut module),
        define_mul_extref(&mut module),
        define_fold_extref(&mut module),
        define_fold_memory(&mut module),
    ];

    funcs.push(define_add_native_test(&mut module, &funcs));
    funcs.push(define_add_memory_test(&mut module, &funcs));
    funcs.push(define_add_extref_test(&mut module, &funcs));
    funcs.push(define_add_square_extref_test(&mut module, &funcs));
    funcs.push(define_fold_add_square_extref_test(&mut module, &funcs));

    // Create and export a Wasm memory
    let memory_id = module.memories.add_local(false, 1, None);
    module.exports.add("vm_mem", ExportItem::Memory(memory_id));

    // Compile the module.
    let wasm_bytes = module.emit_wasm();
    module
        .emit_wasm_file("target/out.wasm")
        .expect("Failed to write wasm file");

    wasm_bytes
}

/// ================================================================================
/// `fold_memory` function.
/// ================================================================================
fn define_fold_memory(module: &mut walrus::Module) -> WasmFunctionMapping {
    use walrus::ValType;

    // Import the API definition for `fold_memory`.
    let fold_memory_ty = module.types.add(
        &[
            ValType::Funcref,
            ValType::I32,
            ValType::I32,
            ValType::I32,
            ValType::I32,
        ],
        &[ValType::I32, ValType::I32, ValType::I32],
    );

    let (function_id, _) = module.add_import_func("clarity", "fold_memory", fold_memory_ty);
    WasmFunctionMapping::new_import("fold_memory", function_id)
}

/// ================================================================================
/// `mul_externref` function.
/// ================================================================================
fn define_mul_extref(module: &mut walrus::Module) -> WasmFunctionMapping {
    use walrus::ValType;

    // Import the API definition for `mul_extref`.
    let mul_extref_ty = module.types.add(
        &[ValType::Externref, ValType::Externref],
        &[ValType::Externref],
    );

    let (function_id, _) = module.add_import_func("clarity", "mul_extref", mul_extref_ty);
    WasmFunctionMapping::new_import("mul_extref", function_id)
}

/// ================================================================================
/// `fold_extref` function.
/// ================================================================================
fn define_fold_extref(module: &mut walrus::Module) -> WasmFunctionMapping {
    use walrus::ValType;

    // Import the API definition for `fold_extref`.
    let fold_extref_ty = module.types.add(
        &[ValType::Funcref, ValType::Externref, ValType::Externref],
        &[ValType::Externref],
    );

    let (function_id, _) = module.add_import_func("clarity", "fold_extref", fold_extref_ty);
    WasmFunctionMapping::new_import("fold_extref", function_id)
}

/// ================================================================================
/// `add_externref` function.
/// ================================================================================
fn define_add_extref(module: &mut walrus::Module) -> WasmFunctionMapping {
    use walrus::ValType;

    // Import the API definition for `add_extref`.
    let add_extref_ty = module.types.add(
        &[ValType::Externref, ValType::Externref],
        &[ValType::Externref],
    );

    let (function_id, _) = module.add_import_func("clarity", "add_extref", add_extref_ty);
    WasmFunctionMapping::new_import("add_extref", function_id)
}

/// ================================================================================
/// `add_native` function.
/// ================================================================================
fn define_add_native(module: &mut walrus::Module) -> WasmFunctionMapping {
    use walrus::ValType;

    // Import the API definition for `native_add_i128`.
    let add_native_ty = module.types.add(
        &[ValType::I64, ValType::I64, ValType::I64, ValType::I64],
        &[ValType::I64, ValType::I64],
    );

    let (function_id, _) = module.add_import_func("clarity", "add_native", add_native_ty);
    WasmFunctionMapping::new_import("add_native", function_id)
}

/// ================================================================================
/// `add_memory` function.
/// ================================================================================
fn define_add_memory(module: &mut walrus::Module) -> WasmFunctionMapping {
    use walrus::ValType;

    // Import the API definition for `memory_add_i128`.
    let add_memory_ty = module.types.add(
        &[ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        &[ValType::I32, ValType::I32, ValType::I32],
    );

    let (function_id, _) = module.add_import_func("clarity", "add_memory", add_memory_ty);
    WasmFunctionMapping::new_import("add_memory", function_id)
}

/// ================================================================================
/// `add_native_test` function.
/// ================================================================================
fn define_add_native_test(
    module: &mut walrus::Module,
    funcs: &[WasmFunctionMapping],
) -> WasmFunctionMapping {
    use walrus::{FunctionBuilder, ValType};

    let add_native_id = funcs.get_by_name("add_native").unwrap().function_id;

    // Define the Wasm test function.
    let mut native_add_i128 = FunctionBuilder::new(
        &mut module.types,
        &[ValType::I64, ValType::I64, ValType::I64, ValType::I64], // list + init
        &[ValType::I64, ValType::I64],
    );

    let a_low = module.locals.add(ValType::I64);
    let a_high = module.locals.add(ValType::I64);
    let b_low = module.locals.add(ValType::I64);
    let b_high = module.locals.add(ValType::I64);

    native_add_i128
        .func_body()
        .local_get(a_low)
        .local_get(a_high)
        .local_get(b_low)
        .local_get(b_high)
        .call(add_native_id);

    let native_add_i128_id =
        native_add_i128.finish(vec![a_low, a_high, b_low, b_high], &mut module.funcs);

    module.exports.add("add_native_test", native_add_i128_id);

    WasmFunctionMapping::new_export("add_native_test", native_add_i128_id)
}

/// ================================================================================
/// `memory_add_i128` function.
/// ================================================================================
fn define_add_memory_test(
    module: &mut walrus::Module,
    funcs: &[WasmFunctionMapping],
) -> WasmFunctionMapping {
    use walrus::{FunctionBuilder, ValType};

    let add_memory_id = funcs.get_by_name("add_memory").unwrap().function_id;

    // Define the Wasm test function.
    let mut add_memory = FunctionBuilder::new(
        &mut module.types,
        &[ValType::I32, ValType::I32, ValType::I32, ValType::I32], // list + init
        &[ValType::I32, ValType::I32, ValType::I32],
    );

    let a_ptr = module.locals.add(ValType::I32);
    let a_len = module.locals.add(ValType::I32);
    let b_ptr = module.locals.add(ValType::I32);
    let b_len = module.locals.add(ValType::I32);

    add_memory
        .func_body()
        .local_get(a_ptr)
        .local_get(a_len)
        .local_get(b_ptr)
        .local_get(b_len)
        .call(add_memory_id);

    let add_memory_id = add_memory.finish(vec![a_ptr, a_len, b_ptr, b_len], &mut module.funcs);

    module.exports.add("add_memory_test", add_memory_id);

    WasmFunctionMapping::new_export("add_memory_test", add_memory_id)
}

/// ================================================================================
/// `add_externref_test` function.
/// ================================================================================
fn define_add_extref_test(
    module: &mut walrus::Module,
    funcs: &[WasmFunctionMapping],
) -> WasmFunctionMapping {
    use walrus::{FunctionBuilder, ValType};

    let add_extref_id = funcs.get_by_name("add_extref").unwrap().function_id;

    // Define the Wasm test function.
    let mut add_extref_test_fn = FunctionBuilder::new(
        &mut module.types,
        &[ValType::Externref, ValType::Externref], // list + init
        &[ValType::Externref],
    );

    let a = module.locals.add(ValType::Externref);
    let b = module.locals.add(ValType::Externref);

    add_extref_test_fn
        .func_body()
        .local_get(a)
        .local_get(b)
        .call(add_extref_id);

    let add_extref_test_id = add_extref_test_fn.finish(vec![a, b], &mut module.funcs);
    module.exports.add("add_extref_test", add_extref_test_id);

    WasmFunctionMapping::new_export("add_extref_test", add_extref_test_id)
}

/// ================================================================================
/// `add_square_extref_test` function.
/// ================================================================================
fn define_add_square_extref_test(
    module: &mut walrus::Module,
    funcs: &[WasmFunctionMapping],
) -> WasmFunctionMapping {
    use walrus::{FunctionBuilder, ValType};

    let mul_extref_id = funcs.get_by_name("mul_extref").unwrap().function_id;
    let add_extref_id = funcs.get_by_name("add_extref").unwrap().function_id;

    let mut add_square_extref = FunctionBuilder::new(
        &mut module.types,
        &[ValType::Externref, ValType::Externref],
        &[ValType::Externref],
    );

    let a = module.locals.add(ValType::Externref);
    let b = module.locals.add(ValType::Externref);

    add_square_extref
        .func_body()
        .local_get(a)
        .local_get(a)
        .call(mul_extref_id)
        .local_get(b)
        .call(add_extref_id);

    let add_square_extref_id = add_square_extref.finish(vec![a, b], &mut module.funcs);
    module
        .exports
        .add("add_square_extref_test", add_square_extref_id);
    WasmFunctionMapping::new_export("add_square_extref_test", add_square_extref_id)
}

/// ================================================================================
/// `fold_add_square_extref_test` function.
/// ================================================================================
fn define_fold_add_square_extref_test(
    module: &mut walrus::Module,
    funcs: &[WasmFunctionMapping],
) -> WasmFunctionMapping {
    use walrus::{FunctionBuilder, ValType};

    let fold_extref_id = funcs.get_by_name("fold_extref").unwrap().function_id;
    let add_square_id = funcs
        .get_by_name("add_square_extref_test")
        .unwrap()
        .function_id;

    let mut fold_add_square = FunctionBuilder::new(
        &mut module.types,
        &[ValType::Externref, ValType::Externref], // list + init
        &[ValType::Externref],
    );

    let list = module.locals.add(ValType::Externref);
    let init = module.locals.add(ValType::Externref);

    fold_add_square
        .func_body()
        .ref_func(add_square_id)
        .local_get(list)
        .local_get(init)
        .call(fold_extref_id);

    let fold_add_square_extref_id = fold_add_square.finish(vec![list, init], &mut module.funcs);
    module
        .exports
        .add("fold_add_square_extref_test", fold_add_square_extref_id);
    WasmFunctionMapping::new_export("fold_add_square_extref_test", fold_add_square_extref_id)
}
