use walrus::{FunctionBuilder, Module, ModuleConfig, ValType};

#[inline]
pub fn generate_wasm() -> Vec<u8> {
    // Construct a new Walrus module.
    let config = ModuleConfig::new();
    let mut module = Module::with_config(config);

    // Import the API definition for `add`.
    let add_ty = module.types.add(
        &[ValType::Externref, ValType::Externref],
        &[ValType::Externref],
    );
    let (add, _) = module.add_import_func("env", "add", add_ty);

    // Import the API definition for `mul`.
    let mul_ty = module.types.add(
        &[ValType::Externref, ValType::Externref],
        &[ValType::Externref],
    );
    let (mul, _) = module.add_import_func("env", "mul", mul_ty);

    // Import the API definition for `fold`.
    let fold_ty = module.types.add(
        &[ValType::Funcref, ValType::Externref, ValType::Externref],
        &[ValType::Externref]
    );
    let (fold, _) = module.add_import_func("env", "fold", fold_ty);

    // * * * * * * * * * * * * *
    // `add-square` function.
    // * * * * * * * * * * * * *
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
        .call(mul)
        .local_get(b)
        .call(add);

    let add_square_fn = add_square.finish(vec![a, b], &mut module.funcs);
    module.exports.add("add-square", add_square_fn);

    // * * * * * * * * * * * * *
    // `fold-add-square` function.
    // * * * * * * * * * * * * *
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
        .call(fold);

    let fold_add_square_fn = fold_add_square.finish(vec![list, init], &mut module.funcs);
    module.exports.add("fold-add-square", fold_add_square_fn);

    // Compile the module.
    let wasm_bytes = module.emit_wasm();
    module
        .emit_wasm_file("target/out.wasm")
        .expect("Failed to write wasm file");

    wasm_bytes
}
