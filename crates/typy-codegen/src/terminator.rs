//! Emits block terminators, including `invoke` with its traceback entry (ADR-0004).

use inkwell::basic_block::BasicBlock;
use inkwell::values::{BasicValue, BasicValueEnum, ValueKind};
use typy_tyir::{Callee, Terminator, TyirType, ValueId};

use crate::error::CodegenError;
use crate::function::FunctionEmitter;
use crate::runtime::{Internal, internal_function, runtime_function};

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    pub(crate) fn terminator(&mut self, terminator: &Terminator) -> Result<(), CodegenError> {
        let current = self
            .builder
            .get_insert_block()
            .ok_or_else(|| CodegenError("no current block".to_owned()))?;
        match terminator {
            Terminator::Jump { target, arguments } => {
                for (parameter, argument) in self
                    .function
                    .block(*target)
                    .parameters
                    .clone()
                    .iter()
                    .zip(arguments)
                {
                    self.add_incoming(*parameter, *argument, current)?;
                }
                self.builder
                    .build_unconditional_branch(self.blocks[target.0 as usize])?;
            }
            Terminator::Branch {
                condition,
                then,
                otherwise,
            } => {
                let condition = self.value(*condition)?.into_int_value();
                self.builder.build_conditional_branch(
                    condition,
                    self.blocks[then.0 as usize],
                    self.blocks[otherwise.0 as usize],
                )?;
            }
            Terminator::Invoke {
                callee,
                arguments,
                normal,
                error,
                span,
            } => {
                self.invoke(*callee, arguments, *normal, *error, *span, current)?;
            }
            Terminator::Return(value) => self.return_value(Some(*value), None)?,
            Terminator::ReturnError(exception) => self.return_value(None, Some(*exception))?,
            Terminator::Unreachable => {
                self.builder.build_unreachable()?;
            }
        }
        Ok(())
    }

    fn add_incoming(
        &mut self,
        parameter: ValueId,
        argument: ValueId,
        from: BasicBlock<'ctx>,
    ) -> Result<(), CodegenError> {
        if let Some(phi) = self.phis.get(&parameter) {
            let value = self.value(argument)?;
            phi.add_incoming(&[(&value, from)]);
        }
        Ok(())
    }

    /// Calls a function that may raise; when the exception is not null, a traceback entry is added before the error block runs.
    fn invoke(
        &mut self,
        callee: Callee,
        arguments: &[ValueId],
        normal: typy_tyir::BlockId,
        error: typy_tyir::BlockId,
        span: typy_diagnostics::Span,
        _current: BasicBlock<'ctx>,
    ) -> Result<(), CodegenError> {
        let context = self.shared.context;
        let (function, result_type) = match callee {
            Callee::Runtime(function) => (
                runtime_function(context, self.shared.module, function),
                function.signature().result,
            ),
            Callee::Function(id) => (
                self.shared.functions[id.0 as usize],
                self.shared.tyir.function(id).return_type,
            ),
        };
        let arguments = self.arguments(arguments)?;
        let returned = match self
            .builder
            .build_call(function, &arguments, "")?
            .try_as_basic_value()
        {
            ValueKind::Basic(value) => value,
            ValueKind::Instruction(_) => {
                return Err(CodegenError(
                    "a raising call returns its exception".to_owned(),
                ));
            }
        };
        let (result, exception) = if crate::abi::value_type(
            context,
            &self.shared.tyir.tuple_types,
            result_type,
        )
        .is_some()
        {
            let pair = returned.into_struct_value();
            let mut result: BasicValueEnum<'ctx> = self.builder.build_extract_value(pair, 0, "")?;
            // A runtime function returns a `bool` as one byte, as C's `_Bool` is.
            if matches!(callee, Callee::Runtime(_)) && result_type == TyirType::Bool {
                result = self.byte_to_bool(result.into_int_value())?.into();
            }
            let exception = self
                .builder
                .build_extract_value(pair, 1, "")?
                .into_pointer_value();
            (Some(result), exception)
        } else {
            (None, returned.into_pointer_value())
        };
        let raised = self.builder.build_is_not_null(exception, "")?;
        let traceback = context.append_basic_block(self.llvm_function, "traceback");
        let after_call = self
            .builder
            .get_insert_block()
            .ok_or_else(|| CodegenError("no current block".to_owned()))?;
        self.builder
            .build_conditional_branch(raised, traceback, self.blocks[normal.0 as usize])?;
        if let (Some(result), Some(parameter)) =
            (result, self.function.block(normal).parameters.first())
            && let Some(phi) = self.phis.get(parameter)
        {
            phi.add_incoming(&[(&result, after_call)]);
        }
        self.builder.position_at_end(traceback);
        let position = self.shared.lookup.position(span.file(), span.start());
        let file = self.shared.lookup.path(span.file()).to_owned();
        let function_name = self
            .shared
            .statics
            .c_string(self.shared.module, &self.function.name);
        let file_name = self.shared.statics.c_string(self.shared.module, &file);
        let line = context
            .i32_type()
            .const_int(u64::from(position.line), false);
        let add = internal_function(context, self.shared.module, Internal::TracebackAdd);
        self.builder.build_call(
            add,
            &[
                exception.into(),
                function_name.into(),
                file_name.into(),
                line.into(),
            ],
            "",
        )?;
        self.builder
            .build_unconditional_branch(self.blocks[error.0 as usize])?;
        if let Some(parameter) = self.function.block(error).parameters.first()
            && let Some(phi) = self.phis.get(parameter)
        {
            phi.add_incoming(&[(&exception, traceback)]);
        }
        Ok(())
    }

    /// Returns normally or with an exception, in the function's convention: a raising function returns the exception beside its result.
    fn return_value(
        &mut self,
        value: Option<ValueId>,
        exception: Option<ValueId>,
    ) -> Result<(), CodegenError> {
        let context = self.shared.context;
        let result_type = crate::abi::value_type(
            context,
            &self.shared.tyir.tuple_types,
            self.function.return_type,
        );
        let result = match value {
            Some(value) if self.function.value(value).ty != TyirType::Unit => {
                Some(self.value(value)?)
            }
            _ => None,
        };
        if !self.raising_abi {
            match result {
                Some(result) => self.builder.build_return(Some(&result))?,
                None => self.builder.build_return(None)?,
            };
            return Ok(());
        }
        let pointer = context.ptr_type(inkwell::AddressSpace::default());
        let exception_value: BasicValueEnum<'ctx> = match exception {
            Some(exception) => self.value(exception)?,
            None => pointer.const_null().into(),
        };
        match result_type {
            None => {
                self.builder.build_return(Some(&exception_value))?;
            }
            Some(ty) => {
                let pair_type = context.struct_type(&[ty, pointer.into()], false);
                let first = result.unwrap_or_else(|| zero(ty));
                let pair = self
                    .builder
                    .build_insert_value(pair_type.get_undef(), first, 0, "")?;
                let pair = self
                    .builder
                    .build_insert_value(pair, exception_value, 1, "")?;
                self.builder
                    .build_return(Some(&pair.as_basic_value_enum()))?;
            }
        }
        Ok(())
    }
}

/// Returns the zero value of a type, which a raising function returns beside an exception.
fn zero(ty: inkwell::types::BasicTypeEnum<'_>) -> BasicValueEnum<'_> {
    match ty {
        inkwell::types::BasicTypeEnum::IntType(int) => int.const_zero().into(),
        inkwell::types::BasicTypeEnum::FloatType(float) => float.const_zero().into(),
        inkwell::types::BasicTypeEnum::PointerType(pointer) => pointer.const_null().into(),
        other => other.const_zero(),
    }
}
