use self::wasm_generator::WasmGenerator;
use clarity::{
    types::StacksEpochId,
    vm::{
        analysis::{run_analysis, AnalysisDatabase, ContractAnalysis},
        ast::build_ast_with_diagnostics,
        costs::LimitedCostTracker,
        database::ClarityBackingStore,
        diagnostic::{DiagnosableError, Diagnostic},
        types::QualifiedContractIdentifier,
        ClarityVersion, Value,
    },
};
use walrus::{ir::InstrSeqId, FunctionBuilder, GlobalId, ImportId, LocalId, TableId, ValType};

// Sub-module definitions
mod traversals;
mod visitors;
mod wasm_generator;

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
            WasmGenerationError::EmptyListTraversal => "Attempted to traverse an empty list".to_string(),
            WasmGenerationError::UnknownFunction(name) => format!("Unknown function: {}", name),
        }
    }

    fn suggestion(&self) -> Option<String> {
        None
    }
}

#[derive(Debug)]
pub(crate) struct WasmFunctionContext {
    pub id: InstrSeqId,
    pub function_builder: FunctionBuilder,
    pub name: String,
    pub locals: Vec<LocalId>,
    pub params: Vec<ParameterDefinition>,
}

#[derive(Debug)]
pub struct ParameterDefinition {
    pub name: String,
    pub val_type: ValType,
    pub local_id: LocalId,
}

impl ParameterDefinition {
    pub fn new(name: &str, val_type: ValType, local_id: LocalId) -> Self {
        ParameterDefinition {
            name: name.to_string(),
            val_type,
            local_id,
        }
    }
}

#[derive(Debug)]
pub struct GlobalImportReference {
    pub global_id: GlobalId,
    pub import_id: ImportId,
}

#[derive(Debug)]
pub struct TableImportReference {
    pub table_id: TableId,
    pub import_id: ImportId,
    pub constants: Vec<VariableReference>,
}

impl TableImportReference {
    pub fn new(table_id: TableId, import_id: ImportId) -> Self {
        TableImportReference {
            table_id,
            import_id,
            constants: Vec::<VariableReference>::new(),
        }
    }

    pub fn add_const(&mut self, name: &str, value: &Value) -> usize {
        let index = self.constants.len();
        self.constants.push(VariableReference {
            name: name.to_string(),
            index,
            value: value.clone(),
        });
        index
    }
}

#[derive(Debug)]
pub struct VariableReference {
    pub name: String,
    pub index: usize,
    pub value: Value,
}

/// Perform a contract analysis on the provided Clarity contract source code.
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

/// Compile a WASM binary from the provided `ContractAnalysis`.
pub fn compile(contract_analysis: &ContractAnalysis) -> Result<CompileResult, CompileError> {
    let mut generator = WasmGenerator::new();

    generator
        .generate(contract_analysis.clone())
        .map_err(CompileError::Wasm)?;

    let module_bytes = generator.finalize();

    Ok(CompileResult {
        module_bytes: module_bytes.clone(),
    })
}
