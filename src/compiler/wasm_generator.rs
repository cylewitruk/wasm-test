use clarity::vm::analysis::ContractAnalysis;
use walrus::{FunctionBuilder, InstrSeqBuilder, LocalId, Module, ModuleConfig, ValType};

use super::{
    GlobalImportReference, ParameterDefinition, TableImportReference, WasmFunctionContext,
    WasmGenerationResult,
};

#[derive(Debug)]
pub struct WasmGenerator {
    pub(crate) module: Module,
    pub(crate) current_fn: Option<WasmFunctionContext>,
    pub(crate) cost_tracker_ref: GlobalImportReference,
    pub(crate) const_table: TableImportReference,
}

impl WasmGenerator {
    pub fn new() -> Self {
        // Construct a new Walrus module.
        let config = ModuleConfig::new();
        let mut module = Module::with_config(config);

        // Add a global for the cost tracker, stored in a global called `__cost_tracker_ref`.
        let (cost_tracker_global_id, cost_tracker_import_id) =
            module.add_import_global("clarity", "__cost_tracker_ref", ValType::Externref, false);

        // Create a table for constants, stored in a table called `__consts`.
        let (const_table_id, const_table_import_id) =
            module.add_import_table("clarity", "__consts", 0, None, ValType::Externref);

        WasmGenerator {
            module,
            current_fn: None,
            cost_tracker_ref: GlobalImportReference {
                global_id: cost_tracker_global_id,
                import_id: cost_tracker_import_id,
            },
            const_table: TableImportReference::new(const_table_id, const_table_import_id),
        }
    }

    /// Generate the
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
        let fn_params = params.iter().map(|x| x.val_type).collect::<Vec<ValType>>();

        // Initialize a new function builder.
        let function_builder = FunctionBuilder::new(&mut self.module.types, &fn_params, &results);

        let current_fn = WasmFunctionContext {
            id: function_builder.func_body_id(),
            function_builder,
            name: name.to_string(),
            locals: Vec::<LocalId>::new(),
            params,
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

    /// Retrieves the instruction sequence builder for the function currently being built.
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
}
