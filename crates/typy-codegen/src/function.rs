//! Emits one TyIR function as an LLVM function: blocks, phi nodes for block parameters, and slots as allocas.

use std::collections::BTreeMap;

use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::{
    BasicMetadataValueEnum, BasicValueEnum, FunctionValue, PhiValue, PointerValue,
};
use typy_diagnostics::SourceLookup;
use typy_tyir::{TyirFunction, TyirModule, ValueId};

use crate::abi::value_type;
use crate::error::CodegenError;
use crate::statics::Statics;

/// What every function's emission shares: the LLVM context and module, the declared functions and the static objects.
pub(crate) struct Shared<'module, 'ctx> {
    pub(crate) context: &'ctx Context,
    pub(crate) module: &'module Module<'ctx>,
    pub(crate) statics: &'module mut Statics<'ctx>,
    pub(crate) tyir: &'module TyirModule,
    pub(crate) functions: &'module [FunctionValue<'ctx>],
    pub(crate) raising: &'module [bool],
    pub(crate) lookup: &'module dyn SourceLookup,
}

/// Emits the body of one function.
pub(crate) struct FunctionEmitter<'shared, 'module, 'ctx> {
    pub(crate) shared: &'shared mut Shared<'module, 'ctx>,
    pub(crate) builder: Builder<'ctx>,
    pub(crate) function: &'module TyirFunction,
    pub(crate) llvm_function: FunctionValue<'ctx>,
    pub(crate) blocks: Vec<BasicBlock<'ctx>>,
    pub(crate) phis: BTreeMap<ValueId, PhiValue<'ctx>>,
    pub(crate) values: Vec<Option<BasicValueEnum<'ctx>>>,
    pub(crate) slots: Vec<Option<PointerValue<'ctx>>>,
    pub(crate) raising_abi: bool,
}

impl<'shared, 'module, 'ctx> FunctionEmitter<'shared, 'module, 'ctx> {
    /// Emits `function` into its declared LLVM function.
    pub(crate) fn emit(
        shared: &'shared mut Shared<'module, 'ctx>,
        function: &'module TyirFunction,
    ) -> Result<(), CodegenError> {
        let index = function.id.0 as usize;
        let llvm_function = shared.functions[index];
        let raising_abi = shared.raising[index];
        let builder = shared.context.create_builder();
        let blocks: Vec<BasicBlock<'ctx>> = function
            .blocks
            .iter()
            .map(|block| {
                shared
                    .context
                    .append_basic_block(llvm_function, &format!("b{}", block.id.0))
            })
            .collect();
        let mut emitter = FunctionEmitter {
            shared,
            builder,
            function,
            llvm_function,
            blocks,
            phis: BTreeMap::new(),
            values: vec![None; function.values.len()],
            slots: Vec::new(),
            raising_abi,
        };
        emitter.prepare()?;
        for block in &function.blocks {
            emitter
                .builder
                .position_at_end(emitter.blocks[block.id.0 as usize]);
            for instruction in &block.instructions {
                let value = emitter.operation(&instruction.operation)?;
                if let (Some(result), Some(value)) = (instruction.result, value) {
                    emitter.values[result.0 as usize] = Some(value);
                }
            }
            emitter.terminator(&block.terminator)?;
        }
        Ok(())
    }

    /// Maps the parameters, adds a phi node for each parameter of every other block, and an alloca for each slot.
    fn prepare(&mut self) -> Result<(), CodegenError> {
        let context = self.shared.context;
        let mut parameters = self.llvm_function.get_param_iter();
        for value in &self.function.parameters {
            if value_type(
                context,
                &self.shared.tyir.tuple_types,
                self.function.value(*value).ty,
            )
            .is_some()
            {
                self.values[value.0 as usize] = parameters.next();
            }
        }
        for block in self.function.blocks.iter().skip(1) {
            self.builder
                .position_at_end(self.blocks[block.id.0 as usize]);
            for parameter in &block.parameters {
                if let Some(ty) = value_type(
                    context,
                    &self.shared.tyir.tuple_types,
                    self.function.value(*parameter).ty,
                ) {
                    let phi = self.builder.build_phi(ty, &format!("v{}", parameter.0))?;
                    self.phis.insert(*parameter, phi);
                    self.values[parameter.0 as usize] = Some(phi.as_basic_value());
                }
            }
        }
        self.builder.position_at_end(self.blocks[0]);
        for (index, slot) in self.function.slots.iter().enumerate() {
            let pointer = match value_type(context, &self.shared.tyir.tuple_types, slot.ty) {
                Some(ty) => Some(self.builder.build_alloca(ty, &format!("s{index}"))?),
                None => None,
            };
            self.slots.push(pointer);
        }
        Ok(())
    }

    /// Returns the LLVM value of a TyIR value, which the verifier has checked is defined before this use.
    pub(crate) fn value(&self, id: ValueId) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        self.values
            .get(id.0 as usize)
            .copied()
            .flatten()
            .ok_or_else(|| {
                CodegenError(format!(
                    "{}: v{} has no LLVM value",
                    self.function.symbol, id.0
                ))
            })
    }

    /// Returns the arguments of a call, leaving out `unit` values, which have no LLVM value.
    pub(crate) fn arguments(
        &self,
        arguments: &[ValueId],
    ) -> Result<Vec<BasicMetadataValueEnum<'ctx>>, CodegenError> {
        arguments
            .iter()
            .filter(|value| {
                value_type(
                    self.shared.context,
                    &self.shared.tyir.tuple_types,
                    self.function.value(**value).ty,
                )
                .is_some()
            })
            .map(|value| self.value(*value).map(Into::into))
            .collect()
    }
}
