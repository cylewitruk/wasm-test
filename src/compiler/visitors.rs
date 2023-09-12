use clarity::vm::{ClarityName, SymbolicExpression, Value};
use walrus::{ValType, ElementKind, InitExpr};
use wasmtime::ExternRef;

use super::{wasm_generator::WasmGenerator, WasmGenerationResult};

impl WasmGenerator {
    pub(crate) fn visit_add(
        &mut self,
        expr: &SymbolicExpression,
        operands: &[SymbolicExpression],
    ) -> WasmGenerationResult {
        println!("==> visit_add()");
        Ok(())
    }

    pub(crate) fn visit_literal_value(
        &mut self,
        expr: &SymbolicExpression,
        value: &Value
    ) -> WasmGenerationResult {
        println!("===> visit_literal_value({}): {}", value, expr);

        /*let index = self.const_table.add_const("_", value);
        let table = self.module.tables.get(self.const_table.table_id);
        let func = &self.current_fn.unwrap().id;

        let element = self.module.elements.add(
            ElementKind::Active { table: self.const_table.table_id, offset: InitExpr::Value(walrus::ir::Value::I32(0)) },
            ValType::Externref,
            Vec::<_>::new()
        );*/
        

        todo!()
    }

    pub(crate) fn visit_define_constant(
        &self,
        name: &ClarityName,
        expr: &[SymbolicExpression],
    ) -> WasmGenerationResult {
        todo!()
    }

    /*
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
    module.exports.add("add-square", add_square_fn); */
}
