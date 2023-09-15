use clarity::vm::Value;
use criterion::{criterion_group, criterion_main, Criterion};
use wasm_test::{get_all_functions, ClarityWasmContext};
use wasmtime::{Config, Engine, Extern, ExternRef, Instance, Module, Store, Val};

pub fn criterion_benchmark(c: &mut Criterion) {
    // Generate a wasm module (see `wasm_generator.rs`) which has a `toplevel` function
    // which in turn calls the below defined wrapped function `func`.
    let wasm_bytes = generate_wasm();

    // Initialize config which allows for reference types.
    let mut config = Config::new();
    config.wasm_reference_types(true);
    config.wasm_threads(true);

    Engine::tls_eager_initialize();

    // Initialize the wasmtime engine.
    let engine = Engine::new(&config).expect("Failed to initialize engine");

    // Pre-compile the module.
    let precompiled = engine
        .precompile_module(&wasm_bytes)
        .expect("Failed to precompile module");

    // Initialize the wasmtime store (using a custom state type).
    let state = ClarityWasmContext {};
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

    c.bench_function("fold-add-square", |b| {
        let instance_fn = instance
            .get_func(&mut store, "fold-add-square")
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

    c.bench_function("add_externref", |b| {
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

    c.bench_function("native_add_i128", |b| {
        let instance_fn = instance
            .get_func(&mut store, "native_add_i128_test")
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

    c.bench_function("memory_add_i128", |b| {
        let instance_fn = instance
            .get_func(&mut store, "memory_add_i128_test")
            .expect("Failed to get fn");

        // Define our output parameters. Note that we're using `Option`s as stated above.
        let results = &mut [Val::I32(0)];

        let mem = instance
            .get_memory(&mut store, "vm_mem")
            .expect("Failed to find 'vm_mem'.");

        mem.write(&mut store, 0, &[0; 32])
            .expect("Couldn't write memory");

        b.iter(|| {
            instance_fn
                .call(&mut store, &[Val::I32(0)], results)
                .expect("Failed to call function")
        });

        assert_eq!(results[0].i32(), Some(32));
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

#[inline]
pub fn generate_wasm() -> Vec<u8> {
    use walrus::{ExportItem, FunctionBuilder, Module, ModuleConfig, ValType};

    // Construct a new Walrus module.
    let config = ModuleConfig::new();
    let mut module = Module::with_config(config);

    // Import the API definition for `add_extref`.
    let add_extref_ty = module.types.add(
        &[ValType::Externref, ValType::Externref],
        &[ValType::Externref],
    );
    let (add_extref_id, _) = module.add_import_func("clarity", "add_extref", add_extref_ty);

    // Import the API definition for `native_add_i128`.
    let native_add_i128_ty = module.types.add(
        &[ValType::I64, ValType::I64, ValType::I64, ValType::I64],
        &[ValType::I64, ValType::I64],
    );
    let (native_add_i128_id, _) =
        module.add_import_func("clarity", "native_add_i128", native_add_i128_ty);

    // Import the API definition for `memory_add_i128`.
    let memory_add_i128_ty = module.types.add(&[ValType::I32], &[ValType::I32]);
    let (memory_add_i128_id, _) =
        module.add_import_func("clarity", "memory_add_i128", memory_add_i128_ty);

    // Import the API definition for `mul_extref`.
    let mul_extref_ty = module.types.add(
        &[ValType::Externref, ValType::Externref],
        &[ValType::Externref],
    );
    let (mul_extref_id, _) = module.add_import_func("clarity", "mul", mul_extref_ty);

    // Import the API definition for `fold_extref`.
    let fold_extref_ty = module.types.add(
        &[ValType::Funcref, ValType::Externref, ValType::Externref],
        &[ValType::Externref],
    );
    let (fold_extref_id, _) = module.add_import_func("clarity", "fold_extref", fold_extref_ty);

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
    let (fold_memory_id, _) = module.add_import_func("clarity", "fold_memory", fold_memory_ty);

    // ================================================================================
    // `add-square` function.
    // ================================================================================
    let mut add_square = FunctionBuilder::new(
        &mut module.types,
        &[ValType::Externref, ValType::Externref],
        &[ValType::Externref],
    );

    let a = module.locals.add(ValType::Externref);
    let b = module.locals.add(ValType::Externref);

    add_square
        .func_body()
        .local_get(a)
        .local_get(a)
        .call(mul_extref_id)
        .local_get(b)
        .call(add_extref_id);

    let add_square_fn = add_square.finish(vec![a, b], &mut module.funcs);
    module.exports.add("add-square", add_square_fn);
    // ////////////////////////////////////////////////////////////////////////////////

    // ================================================================================
    // `fold-add-square` function.
    // ================================================================================
    let mut fold_add_square = FunctionBuilder::new(
        &mut module.types,
        &[ValType::Externref, ValType::Externref], // list + init
        &[ValType::Externref],
    );

    let list = module.locals.add(ValType::Externref);
    let init = module.locals.add(ValType::Externref);

    fold_add_square
        .func_body()
        .ref_func(add_square_fn)
        .local_get(list)
        .local_get(init)
        .call(fold_extref_id);

    let fold_add_square_fn = fold_add_square.finish(vec![list, init], &mut module.funcs);
    module.exports.add("fold-add-square", fold_add_square_fn);
    // ////////////////////////////////////////////////////////////////////////////////

    // ================================================================================
    // `add` function.
    // ================================================================================
    let mut add = FunctionBuilder::new(
        &mut module.types,
        &[ValType::Externref, ValType::Externref], // list + init
        &[ValType::Externref],
    );

    let a = module.locals.add(ValType::Externref);
    let b = module.locals.add(ValType::Externref);

    add.func_body()
        .local_get(a)
        .local_get(b)
        .call(add_extref_id);

    let add_fn = add.finish(vec![a, b], &mut module.funcs);
    module.exports.add("add_extref_test", add_fn);
    // ////////////////////////////////////////////////////////////////////////////////

    // ================================================================================
    // `add_native_i128` function.
    // ================================================================================
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
        .call(native_add_i128_id);

    let native_add_i128_fn =
        native_add_i128.finish(vec![a_low, a_high, b_low, b_high], &mut module.funcs);
    module
        .exports
        .add("native_add_i128_test", native_add_i128_fn);
    // ////////////////////////////////////////////////////////////////////////////////

    // ================================================================================
    // `memory_add_i128` function.
    // ================================================================================
    let mut memory_add_i128 = FunctionBuilder::new(
        &mut module.types,
        &[ValType::I32], // list + init
        &[ValType::I32],
    );

    let ptr = module.locals.add(ValType::I32);

    memory_add_i128
        .func_body()
        .local_get(ptr)
        .call(memory_add_i128_id);

    let memory_add_i128_fn = memory_add_i128.finish(vec![ptr], &mut module.funcs);
    module
        .exports
        .add("memory_add_i128_test", memory_add_i128_fn);
    // ////////////////////////////////////////////////////////////////////////////////

    let memory_id = module.memories.add_local(false, 1024, None);
    module.exports.add("vm_mem", ExportItem::Memory(memory_id));

    // Compile the module.
    let wasm_bytes = module.emit_wasm();
    module
        .emit_wasm_file("target/out.wasm")
        .expect("Failed to write wasm file");

    wasm_bytes
}
