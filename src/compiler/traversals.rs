use clarity::vm::{
    functions::{define::DefineFunctions, NativeFunctions},
    ClarityVersion, SymbolicExpression, SymbolicExpressionType,
};
use walrus::ValType;

use super::{
    wasm_generator::WasmGenerator, ParameterDefinition, WasmGenerationError, WasmGenerationResult,
};

impl WasmGenerator {
    /// Traverses the provided expression. recursively.
    pub(crate) fn traverse_expr(&mut self, expr: &SymbolicExpression) -> WasmGenerationResult {
        match &expr.expr {
            SymbolicExpressionType::List(expressions) => self.traverse_list(expr, expressions)?,
            SymbolicExpressionType::Atom(clarity_name) => {
                println!("==> traverse_expr(Atom): {}", clarity_name);
                todo!()
            }
            SymbolicExpressionType::AtomValue(value) => {
                println!("==> traverse_expr(AtomValue): {}", value);
                todo!()
            }
            SymbolicExpressionType::Field(trait_id) => {
                println!("==> traverse_expr(Field): {}", trait_id);
                todo!()
            }
            SymbolicExpressionType::LiteralValue(value) => {
                println!("==> traverse_expr(LiteralValue): {}", value);
                self.visit_literal_value(expr, value)?;
            }
            SymbolicExpressionType::TraitReference(clarity_name, trait_def) => {
                println!(
                    "==> traverse_expr(TraitReference): {}, {:?}",
                    clarity_name, trait_def
                );
                todo!()
            }
        }
        Ok(())
    }

    /// Traverses the provided list, recursively. This method splits the first expression from the remaining expressions
    /// and uses the first expression as the name, and the rest as remaining arguments to be handled by further
    /// traversals and/or visits.
    pub(crate) fn traverse_list(
        &mut self,
        expr: &SymbolicExpression,
        list_expressions: &[SymbolicExpression],
    ) -> WasmGenerationResult {
        let (function_name, args) = match list_expressions.split_first() {
            Some(result) => result,
            None => Err(WasmGenerationError::EmptyListTraversal)?,
        };

        let function_name = match function_name.match_atom() {
            Some(name) => name,
            None => Err(WasmGenerationError::UnknownFunction(
                function_name.to_string(),
            ))?,
        };

        // If the current expression is a define-function (as defined in the `DefineFunctions` enum),
        // traverse the function.
        if let Some(define_function) = DefineFunctions::lookup_by_name(function_name) {
            self.traverse_define(define_function, args)?;
        // Or, if the current expression is a native-function call (as defined in the `NativeFunctions` enum),
        // traverse the function.
        } else if let Some(native_function) =
            NativeFunctions::lookup_by_name_at_version(function_name, &ClarityVersion::latest())
        {
            // traverse native
            self.traverse_native_function(native_function, expr, args)?;
            todo!("Native functions")
        }

        Ok(())
    }

    pub(crate) fn traverse_native_function(
        &mut self,
        function: NativeFunctions,
        expr: &SymbolicExpression,
        operands: &[SymbolicExpression],
    ) -> WasmGenerationResult {
        for op in operands {
            self.traverse_expr(op)?;
        }

        match function {
            NativeFunctions::Add => self.visit_add(expr, operands),
            _ => todo!("Function {} not implemented.", function),
        }
    }

    /// Traverses the provided define-function and its arguments.
    pub(crate) fn traverse_define(
        &mut self,
        function: DefineFunctions,
        expr: &[SymbolicExpression],
    ) -> WasmGenerationResult {
        match function {
            //DefineFunctions::Constant => self.visit_define_constant(function, args),
            DefineFunctions::PublicFunction
            | DefineFunctions::PrivateFunction
            | DefineFunctions::ReadOnlyFunction => self.traverse_define_function(function, expr),
            _ => todo!(),
        }
    }

    /// Traverses the define-function functions (define-public, define-private, define-read-only) and routes
    /// to the correct visitor.
    pub(crate) fn traverse_define_function(
        &mut self,
        function: DefineFunctions,
        expr: &[SymbolicExpression],
    ) -> WasmGenerationResult {
        // The method signature is included in the first expression in the list, and is itself an expression list.
        // In the signature expression list:
        // - The first expression includes the name of the defined function
        // - The remaining expressions describe the input parameters, in pairs.
        // - TODO: Return type?
        // So here, we extract both the name and parameters from the signature expression.
        let (name, parameters) = expr[0].match_list().unwrap().split_first().unwrap();

        // Convert the name expression to a `ClarityName`.
        let name = name.match_atom().unwrap();

        // The method body is included in the second expression in the list. This will be used for
        // further traversal to generate the WASM function body.
        let body = &expr[1];

        //println!("==> NAME: {:?}", name);
        //println!("==> PARAMS: {:?}", parameters);
        //println!("==> BODY: {:?}", body);

        let mut params = Vec::<ParameterDefinition>::new();

        for arg in parameters {
            arg.match_list().unwrap().chunks(2).for_each(|x| {
                // Add a new local to the module for the input parameter.
                let local_id = self.module.locals.add(ValType::Externref);
                // Create a new `ParameterDefinition` for the input parameter.
                let param_def = ParameterDefinition::new(
                    x[0].match_atom().unwrap(),
                    ValType::Externref,
                    local_id,
                );
                // Add the parameter to the input parameter definitions.
                params.push(param_def);
            });
        }

        // Begin the function.
        self.begin_function(name, params, &[ValType::Externref]);

        // Traverse the function's body expression, building the function along the way.
        self.traverse_expr(body)?;

        // Once the body traversal is finished, we can end the function. If this is a `public` function,
        // then we also need to export it from the module.
        self.end_function(function == DefineFunctions::PublicFunction);

        Ok(())
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
