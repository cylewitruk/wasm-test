use clarity::{
    types::StacksEpochId,
    vm::{
        ClarityVersion,
        types::QualifiedContractIdentifier,
        ast::build_ast_with_diagnostics, 
        costs::LimitedCostTracker, 
        database::ClarityBackingStore, 
        analysis::{AnalysisDatabase, run_analysis, ContractAnalysis}, 
        diagnostic::{Diagnostic, DiagnosableError}, SymbolicExpression, SymbolicExpressionType, functions::{define::DefineFunctions, NativeFunctions}, ClarityName
    }
};
use lazy_static::lazy_static;
use walrus::{Module, ModuleConfig, FunctionBuilder, ValType, LocalId, InstrSeqBuilder};

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
    module_bytes: Vec<u8>
}

#[derive(Debug)]
pub struct AnalyzeResult {
    pub diagnostics: Vec<Diagnostic>,
    pub contract_analysis: ContractAnalysis
}

#[derive(Debug)]
pub enum CompileError {
    Wasm(WasmGenerationError)
}

#[derive(Debug)]
pub enum AnalyzeError {
    Generic(Vec<Diagnostic>),
    FailedToParseContract(Vec<Diagnostic>)
}

#[derive(Debug)]
pub enum WasmGenerationError {
    NotImplemented,
    UnknownFunction(String),
    InternalError(String),
    EmptyListTraversal
}

impl DiagnosableError for WasmGenerationError {
    fn message(&self) -> String {
        match self {
            WasmGenerationError::NotImplemented => "Not implemented".to_string(),
            WasmGenerationError::InternalError(msg) => format!("Internal error: {}", msg),
            WasmGenerationError::EmptyListTraversal => format!("Attempted to traverse an empty list"),
            WasmGenerationError::UnknownFunction(name) => format!("Unknown function: {}", name)
        }
    }

    fn suggestion(&self) -> Option<String> {
        None
    }
}

#[derive(Debug)]
struct WasmFunctionContext {
    pub function_builder: FunctionBuilder,
    pub name: String,
    pub locals: Vec<LocalId>
}

pub fn analyze_contract(
    source: &str, 
    contract_id: &QualifiedContractIdentifier,
    mut cost_tracker: LimitedCostTracker,
    clarity_version: ClarityVersion,
    epoch: StacksEpochId,
    datastore: &mut dyn ClarityBackingStore
) -> Result<AnalyzeResult, AnalyzeError> {

    // Parse the contract.
    let (mut ast, mut diagnostics, success) =
        build_ast_with_diagnostics(
            contract_id, 
            source, 
            &mut cost_tracker, 
            clarity_version, 
            epoch);

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

    generator.generate(contract_analysis.clone())
        .map_err(|e| CompileError::Wasm(e))?;

    let module_bytes = generator.finalize();

    Ok(CompileResult {
        module_bytes: module_bytes.clone(),
    })
}

#[derive(Debug)]
pub struct WasmGenerator {
    module: Module,
    current_fn: Option<WasmFunctionContext>
}

impl WasmGenerator {
    pub fn new() -> Self {
        // Construct a new Walrus module.
        let config = ModuleConfig::new();
        let module = Module::with_config(config);

        WasmGenerator {
            module,
            current_fn: None
        }
    }

    pub fn generate(&mut self, contract_analysis: ContractAnalysis) -> WasmGenerationResult {
        for expr in contract_analysis.expressions.iter() {
            self.traverse_expr(&expr)?
        }
        
        todo!()
    }

    /// Finalizes the module, consuming `self` and emitting the final WASM binary bytes.
    pub fn finalize(mut self) -> Vec<u8> {
        self.module.emit_wasm()
    }

    /// Puts the `WasmGenerator` in function-building mode.
    pub fn begin_function(&mut self, name: &str, params: &[ValType], results: &[ValType]) {
        // Initialize a new function builder.
        let function_builder = FunctionBuilder::new(
            &mut self.module.types,
            &params,
            &results,
        );

        let current_fn = WasmFunctionContext {
            function_builder: function_builder,
            name: name.to_string(),
            locals: Vec::<LocalId>::new()
        };

        // Set the current WASM function context for this generator.
        self.current_fn = Some(current_fn);
    }

    /// Finalizes a function by finishing the function builder, inserting the function into the
    /// module and, optionally, exporting it.
    pub fn end_function(&mut self, export: bool) {
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
        //println!("=====> {:?}", expr);
        match &expr.expr {
            SymbolicExpressionType::List(expressions) => self.traverse_list(expr, &*expressions)?,
            _ => Err(WasmGenerationError::NotImplemented)?,
        }
        Ok(())
    }

    /// Traverses the provided list, recursively. This method splits the first expression from the remaining expressions
    /// and uses the first expression as the name, and the rest as remaining arguments to be handled by further
    /// traversals and/or visits.
    fn traverse_list(&mut self, current_expr: &SymbolicExpression, list_expressions: &[SymbolicExpression]) -> WasmGenerationResult {
        let (function_name, args) = match list_expressions.split_first() {
            Some(result) => result,
            None => Err(WasmGenerationError::EmptyListTraversal)?
        };

        let function_name = match function_name.match_atom() {
            Some(name) => name,
            None => Err(WasmGenerationError::UnknownFunction(function_name.to_string()))?
        };

        // If the current expression is a define-function (as defined in the `DefineFunctions` enum),
        // traverse the function.
        if let Some(define_function) = DefineFunctions::lookup_by_name(function_name) {
            self.traverse_define(define_function, args)?;
        } else if let Some(native_function) = NativeFunctions::lookup_by_name_at_version(function_name, &ClarityVersion::latest()) {
            // traverse native
        }

        Ok(())
    }

    /// Traverses the provided define-function and its arguments.
    fn traverse_define(&mut self, function: DefineFunctions, expr: &[SymbolicExpression]) -> WasmGenerationResult {
        match function {
            //DefineFunctions::Constant => self.visit_define_constant(function, args),
            DefineFunctions::PublicFunction
            | DefineFunctions::PrivateFunction
            | DefineFunctions::ReadOnlyFunction => self.traverse_define_function(function, expr),
            _ => todo!()
        }
    }

    /// Traverses the define-function functions (define-public, define-private, define-read-only) and routes
    /// to the correct visitor.
    fn traverse_define_function(&mut self, function: DefineFunctions, expr: &[SymbolicExpression]) -> WasmGenerationResult {
        let signature = &expr[0];
        let body = &expr[1];
        let (name, parameters) = signature.match_list().unwrap().split_first().unwrap();
        let name = name.match_atom().unwrap();
        
        println!("==> NAME: {:?}", name);
        println!("==> PARAMS: {:?}", parameters);
        println!("==> BODY: {:?}", body);

        let mut locals = Vec::<LocalId>::new();
        let mut params = Vec::<ValType>::new();

        for arg in parameters {
            arg.match_list()
                .unwrap()
                .chunks(2)
                .for_each(|_| {
                    params.push(ValType::Externref);
                    locals.push(self.module.locals.add(ValType::Externref));
                });
        };

        self.begin_function(name, &params, &[ValType::Externref]);
        
        self.end_function(if function == DefineFunctions::PublicFunction { true } else { false });
        Ok(())
    }

    fn visit_define_constant(&self, name: &ClarityName, args: &[SymbolicExpression]) -> WasmGenerationResult {
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