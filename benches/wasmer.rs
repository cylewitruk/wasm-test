use criterion::{criterion_group, criterion_main, Criterion};
use mimalloc::MiMalloc;

pub fn criterion_benchmark(c: &mut Criterion) {
    use clarity::vm::Value as ClarityValue;
    use wasmer::{
        imports, Engine, ExternRef, Features, Function, FunctionEnv, FunctionEnvMut, Instance,
        Module, Store, TypedFunction,
    };
    use wasmer_compiler_llvm::LLVM;

    // Generate a wasm module (see `wasm_generator.rs`) which has a `toplevel` function
    // which in turn calls the below defined wrapped function `func`.
    let wasm_bytes = generate_wasm();

    let mut features = Features::default();
    features.memory64 = true;
    features.multi_memory = true;
    features.multi_value = true;
    features.reference_types = true;
    features.simd = true;
    features.threads = true;

    // Use LLVM compiler with default settings.
    let mut compiler = LLVM::new();
    compiler.opt_level(wasmer_compiler_llvm::LLVMOptLevel::Aggressive);

    // Create the store.
    let mut store = Store::new(compiler);
    //let engine = Engine::default();
    //let mut store = Store::new(engine);

    // Create a new environment.
    let env = FunctionEnv::new(&mut store, ());

    // Compile the WASM module
    let module = Module::new(&store, wasm_bytes).expect("Failed to compile module");

    // ================================================================================
    // `add` function.
    // ================================================================================
    let add_function = Function::new_typed_with_env(
        &mut store,
        &env,
        |mut env: FunctionEnvMut<()>,
         a: Option<ExternRef>,
         b: Option<ExternRef>|
         -> Option<ExternRef> {
            let a: &ClarityValue = a.as_ref().unwrap().downcast(&env).unwrap();
            let b: &ClarityValue = b.as_ref().unwrap().downcast(&env).unwrap();

            match (a.clone(), b.clone()) {
                (ClarityValue::Int(a), ClarityValue::Int(b)) => {
                    let c = a.checked_add(b).expect("Failed to add");
                    Some(ExternRef::new(&mut env, ClarityValue::Int(c)))
                }
                (ClarityValue::UInt(a), ClarityValue::UInt(b)) => {
                    Some(ExternRef::new(&mut env, ClarityValue::UInt(a + b)))
                }
                _ => unimplemented!(),
            }
        },
    );
    // ////////////////////////////////////////////////////////////////////////////////

    // ================================================================================
    // `add` function.
    // ================================================================================
    let native_add_i128_function = Function::new_typed_with_env(
        &mut store,
        &env,
        |mut env: FunctionEnvMut<()>,
         a_low: i64,
         a_high: i64,
         b_low: i64,
         b_high: i64|
         -> (i64, i64) {
            let a = ((a_high as u64) as u128) << 64 | ((a_low as u64) as u128);
            let b = ((b_high as u64) as u128) << 64 | ((b_low as u64) as u128);

            let result: i128 = a.checked_add(b).unwrap().try_into().unwrap();

            (
                (result & 0xFFFFFFFFFFFFFFFF) as i64,
                ((result >> 64) & 0xFFFFFFFFFFFFFFFF) as i64,
            )
        },
    );
    // ////////////////////////////////////////////////////////////////////////////////

    // Import definitions
    let import_object = imports! {
        "clarity" => {
            "add" => add_function,
            "native_add_i128" => native_add_i128_function
        }
    };

    // Instantiate the module
    let instance =
        Instance::new(&mut store, &module, &import_object).expect("Failed to instantiate module");

    // Bench `add`
    c.bench_function("add", |b| {
        let instance_fn: TypedFunction<(Option<ExternRef>, Option<ExternRef>), Option<ExternRef>> =
            instance
                .exports
                .get_typed_function(&store, "top_level")
                .expect("Failed to get fn");

        b.iter(|| {
            let a1 = ExternRef::new(&mut store, ClarityValue::Int(1));
            let a2 = ExternRef::new(&mut store, ClarityValue::Int(2));

            instance_fn
                .call(&mut store, Some(a1), Some(a2))
                .expect("Failed to call add function");
        });
    });

    // Bench `add`
    c.bench_function("native_add_i128", |b| {
        let instance_fn: TypedFunction<(i64, i64, i64, i64), (i64, i64)> = instance
            .exports
            .get_typed_function(&store, "native_add_i128")
            .expect("Failed to get fn");

        b.iter(|| {
            instance_fn
                .call(&mut store, 5i64, 0i64, 10i64, 0i64)
                .expect("Failed to call add function");
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

#[inline]
pub fn generate_wasm() -> Vec<u8> {
    use walrus::{FunctionBuilder, Module, ModuleConfig, ValType};

    // Construct a new Walrus module.
    let config = ModuleConfig::new();
    let mut module = Module::with_config(config);

    // Import the API definition for `add`.
    let add_ty = module.types.add(
        &[ValType::Externref, ValType::Externref],
        &[ValType::Externref],
    );
    let (add_id, _) = module.add_import_func("clarity", "add", add_ty);

    // Import the API definition for `native_add_i128`.
    let native_add_i128_ty = module.types.add(
        &[ValType::I64, ValType::I64, ValType::I64, ValType::I64],
        &[ValType::I64, ValType::I64],
    );
    let (native_add_i128_id, _) =
        module.add_import_func("clarity", "native_add_i128", native_add_i128_ty);

    // ================================================================================
    // `top_level` function.
    // ================================================================================
    let mut top_level_fb = FunctionBuilder::new(
        &mut module.types,
        &[ValType::Externref, ValType::Externref],
        &[ValType::Externref],
    );

    let a = module.locals.add(ValType::Externref);
    let b = module.locals.add(ValType::Externref);

    top_level_fb
        .func_body()
        .local_get(a)
        .local_get(b)
        .call(add_id);

    let top_level_fn = top_level_fb.finish(vec![a, b], &mut module.funcs);
    module.exports.add("top_level", top_level_fn);
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
    module.exports.add("native_add_i128", native_add_i128_fn);
    // ////////////////////////////////////////////////////////////////////////////////

    // Compile the module.
    let wasm_bytes = module.emit_wasm();
    module
        .emit_wasm_file("target/out.wasm")
        .expect("Failed to write wasm file");

    wasm_bytes
}
