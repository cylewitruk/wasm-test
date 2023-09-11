use clarity::{
    types::StacksEpochId,
    vm::{
        analysis::{run_analysis, AnalysisDatabase, ContractAnalysis},
        ast::build_ast_with_diagnostics,
        costs::LimitedCostTracker,
        database::ClarityBackingStore,
        diagnostic::{DiagnosableError, Diagnostic},
        functions::{define::DefineFunctions, NativeFunctions},
        types::QualifiedContractIdentifier,
        ClarityName, ClarityVersion, SymbolicExpression, SymbolicExpressionType,
    },
};
use lazy_static::lazy_static;
use walrus::{
    ir::InstrSeqId, FunctionBuilder, GlobalId, ImportId, InstrSeqBuilder, LocalId, Module,
    ModuleConfig, ValType,
};

lazy_static! {
    // Since the AST Visitor may be used before other checks have been performed,
    // we may need a default value for some expressions. This can be used for a
    // missing `ClarityName`.
    static ref DEFAULT_NAME: ClarityName = ClarityName::from("placeholder__");
    static ref DEFAULT_EXPR: SymbolicExpression = SymbolicExpression::atom(DEFAULT_NAME.clone());
}

type WasmGenerationResult = Result<(), WasmGenerationError>;

#[derive(Debug)]
pub struct CompileResult {
    module_bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct AnalyzeResult {
    pub diagnostics: Vec<Diagnostic>,
    pub contract_analysis: ContractAnalysis,
}

#[derive(Debug)]
pub enum CompileError {
    Wasm(WasmGenerationError),
}

#[derive(Debug)]
pub enum AnalyzeError {
    Generic(Vec<Diagnostic>),
    FailedToParseContract(Vec<Diagnostic>),
}

#[derive(Debug)]
pub enum WasmGenerationError {
    NotImplemented,
    UnknownFunction(String),
    InternalError(String),
    EmptyListTraversal,
}

impl DiagnosableError for WasmGenerationError {
    fn message(&self) -> String {
        match self {
            WasmGenerationError::NotImplemented => "Not implemented".to_string(),
            WasmGenerationError::InternalError(msg) => format!("Internal error: {}", msg),
            WasmGenerationError::EmptyListTraversal => {
                format!("Attempted to traverse an empty list")
            }
            WasmGenerationError::UnknownFunction(name) => format!("Unknown function: {}", name),
        }
    }

    fn suggestion(&self) -> Option<String> {
        None
    }
}

#[derive(Debug)]
struct WasmFunctionContext {
    pub id: InstrSeqId,
    pub function_builder: FunctionBuilder,
    pub name: String,
    pub locals: Vec<LocalId>,
    pub params: Vec<ParameterDefinition>
}

pub fn analyze_contract(
    source: &str,
    contract_id: &QualifiedContractIdentifier,
    mut cost_tracker: LimitedCostTracker,
    clarity_version: ClarityVersion,
    epoch: StacksEpochId,
    datastore: &mut dyn ClarityBackingStore,
) -> Result<AnalyzeResult, AnalyzeError> {
    // Parse the contract.
    let (mut ast, mut diagnostics, success) = build_ast_with_diagnostics(
        contract_id,
        source,
        &mut cost_tracker,
        clarity_version,
        epoch,
    );

    // If parsing failed, return an error.
    if !success {
        return Err(AnalyzeError::FailedToParseContract(diagnostics));
    }

    // Create a new analysis database.
    let mut analysis_db = AnalysisDatabase::new(datastore);

    // Run the analysis passes
    let contract_analysis = match run_analysis(
        contract_id,
        &mut ast.expressions,
        &mut analysis_db,
        false,
        cost_tracker,
        epoch,
        clarity_version,
    ) {
        Ok(contract_analysis) => contract_analysis,
        Err((e, _)) => {
            diagnostics.push(Diagnostic::err(&e.err));
            return Err(AnalyzeError::Generic(diagnostics));
        }
    };

    Ok(AnalyzeResult {
        contract_analysis,
        diagnostics,
    })
}

pub fn compile(contract_analysis: &ContractAnalysis) -> Result<CompileResult, CompileError> {
    let mut generator = WasmGenerator::new();

    generator
        .generate(contract_analysis.clone())
        .map_err(|e| CompileError::Wasm(e))?;

    let module_bytes = generator.finalize();

    Ok(CompileResult {
        module_bytes: module_bytes.clone(),
    })
}

#[derive(Debug)]
pub struct ParameterDefinition {
    pub name: String,
    pub val_type: ValType,
    pub local_id: LocalId
}

impl ParameterDefinition {
    pub fn new(name: &str, val_type: ValType, local_id: LocalId) -> Self {
        ParameterDefinition { 
            name: name.to_string(), 
            val_type, 
            local_id 
        }
    }
}

#[derive(Debug)]
pub struct GlobalImportReference {
    pub global_id: GlobalId,
    pub import_id: ImportId,
}

#[derive(Debug)]
pub struct WasmGenerator {
    module: Module,
    current_fn: Option<WasmFunctionContext>,
    cost_tracker_ref: GlobalImportReference,
}

impl WasmGenerator {
    pub fn new() -> Self {
        // Construct a new Walrus module.
        let config = ModuleConfig::new();
        let mut module = Module::with_config(config);

        // Add a global for the cost tracker.
        let (global_id, import_id) =
            module.add_import_global("clarity", "__cost_tracker_ref", ValType::Externref, false);

        WasmGenerator {
            module,
            current_fn: None,
            cost_tracker_ref: GlobalImportReference {
                global_id,
                import_id,
            },
        }
    }

    pub fn generate(&mut self, contract_analysis: ContractAnalysis) -> WasmGenerationResult {
        // Traverse and visit all of the expressions from the provided `ContractAnalysis`.
        for expr in contract_analysis.expressions.iter() {
            self.traverse_expr(&expr)?
        }

        Ok(())
    }

    /// Finalizes the module, consuming `self` and emitting the final WASM binary bytes.
    pub fn finalize(mut self) -> Vec<u8> {
        self.module.emit_wasm()
    }

    /// Puts the `WasmGenerator` in function-building mode.
    pub fn begin_function(
        &mut self,
        name: &str,
        params: Vec<ParameterDefinition>,
        results: &[ValType],
    ) -> InstrSeqBuilder {
        // Convert the parameters to a list of `ValType`s which is required for the Walrus `FunctionBuilder`.
        let fn_params = params
            .iter()
            .map(|x| x.val_type)
            .collect::<Vec<ValType>>();

        // Initialize a new function builder.
        let function_builder = FunctionBuilder::new(&mut self.module.types, &fn_params, &results);

        let current_fn = WasmFunctionContext {
            id: function_builder.func_body_id(),
            function_builder,
            name: name.to_string(),
            locals: Vec::<LocalId>::new(),
            params
        };

        // Set the current WASM function context for this generator.
        self.current_fn = Some(current_fn);

        let body = self
            .current_fn
            .as_mut()
            .unwrap()
            .function_builder
            .func_body();
        body
    }

    pub fn get_function(&mut self) -> InstrSeqBuilder {
        if self.current_fn.is_none() {
            panic!("Attempt to retrieve a function when no function is being built.");
        }

        let body_id = self.current_fn.as_ref().unwrap().id;
        let body = self
            .current_fn
            .as_mut()
            .unwrap()
            .function_builder
            .instr_seq(body_id);
        body
    }

    /// Finalizes a function by finishing the function builder, inserting the function into the
    /// module and, optionally, exporting it.
    pub fn end_function(&mut self, export: bool) {
        if self.current_fn.is_none() {
            panic!("Attempt to end a function when no function is being built.");
        }

        let func = self.current_fn.take().unwrap();
        let locals = func.locals;
        let function_id = func.function_builder.finish(locals, &mut self.module.funcs);

        if export {
            self.module.exports.add(&func.name, function_id);
        }
    }

    /// Gets whether or not the WasmGenerator is currently in the middle of building
    /// a function.
    pub fn is_building_function(&self) -> bool {
        self.current_fn.is_some()
    }

    /// Traverses the provided expression. recursively.
    fn traverse_expr(&mut self, expr: &SymbolicExpression) -> WasmGenerationResult {
        match &expr.expr {
            SymbolicExpressionType::List(expressions) => self.traverse_list(expr, &*expressions)?,
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
                todo!()
            }
            SymbolicExpressionType::TraitReference(clarity_name, trait_def) => {
                println!("==> traverse_expr(TraitReference): {}, {:?}", clarity_name, trait_def);
                todo!()
            }
        }
        Ok(())
    }

    /// Traverses the provided list, recursively. This method splits the first expression from the remaining expressions
    /// and uses the first expression as the name, and the rest as remaining arguments to be handled by further
    /// traversals and/or visits.
    fn traverse_list(
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

    fn traverse_native_function(
        &mut self,
        function: NativeFunctions,
        expr: &SymbolicExpression,
        operands: &[SymbolicExpression]
    ) -> WasmGenerationResult {
        for op in operands {
            self.traverse_expr(op)?;
        }

        match function {
            NativeFunctions::Add => self.visit_add(expr, operands),
            _ => todo!("Function {} not implemented.", function)
        }
    }

    fn visit_add(&mut self, expr: &SymbolicExpression, operands: &[SymbolicExpression]) -> WasmGenerationResult {
        println!("==> visit_add()");
        Ok(())
    }

    /// Traverses the provided define-function and its arguments.
    fn traverse_define(
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
    fn traverse_define_function(
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
        let (name, parameters) = expr[0]
            .match_list()
            .unwrap()
            .split_first()
            .unwrap();

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
                    local_id
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
        self.end_function(if function == DefineFunctions::PublicFunction {
            true
        } else {
            false
        });

        Ok(())
    }

    fn visit_define_constant(
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
