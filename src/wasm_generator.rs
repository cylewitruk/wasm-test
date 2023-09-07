use walrus::{ModuleConfig, Module, ValType, FunctionBuilder};

pub fn generate_wasm() -> Vec<u8> {
    // Construct a new Walrus module.
    let config = ModuleConfig::new();
    let mut module = Module::with_config(config);

    // Import the `log` function.
    let add_ty = module.types.add(&[ValType::Externref, ValType::Externref], &[ValType::Externref]);
    let (add, _) = module.add_import_func("env", "add", add_ty);

    let mut top_level = FunctionBuilder::new(
        &mut module.types,
        &[ValType::Externref, ValType::Externref],
        &[ValType::Externref]
    );

    let a = module.locals.add(ValType::Externref);
    let b = module.locals.add(ValType::Externref);
    //let ret = module.locals.add(ValType::Externref);

    top_level
        .func_body()
        .local_get(a)
        .local_get(b)
        .call(add);

    let top_level_fn = top_level.finish(vec![a, b], &mut module.funcs);
    module.exports.add("toplevel", top_level_fn);

    let wasm_bytes = module.emit_wasm();
    module.emit_wasm_file("target/out.wasm")
        .expect("Failed to write wasm file");

    wasm_bytes
}