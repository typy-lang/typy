//! Writes an operation the runtime may raise from in place where it cannot raise, and calls the
//! runtime elsewhere, joining the two (ADR-0107).

use inkwell::AddressSpace;
use inkwell::basic_block::BasicBlock;
use inkwell::values::{BasicValueEnum, IntValue, PointerValue, ValueKind};
use typy_tyir::{RuntimeFunction, ValueId};

use crate::error::CodegenError;
use crate::function::FunctionEmitter;
use crate::runtime::runtime_function;

/// What a runtime call that may raise gives: its result, if it has one, and its exception,
/// null when it raised none.
pub(crate) type Raised<'ctx> = (Option<BasicValueEnum<'ctx>>, PointerValue<'ctx>);

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    /// Returns the result of `in_place` and no exception where `fast` holds, and the result
    /// and the exception of the runtime function `function` elsewhere.
    pub(crate) fn in_place_or_call(
        &mut self,
        fast: IntValue<'ctx>,
        function: RuntimeFunction,
        arguments: &[ValueId],
        in_place: impl FnOnce(&mut Self) -> Result<BasicValueEnum<'ctx>, CodegenError>,
    ) -> Result<Raised<'ctx>, CodegenError> {
        let context = self.shared.context;
        let fast = self.expect_true(fast)?;
        let fast_block = context.append_basic_block(self.llvm_function, "in_place");
        let call_block = context.append_basic_block(self.llvm_function, "call");
        let join = context.append_basic_block(self.llvm_function, "");
        self.builder
            .build_conditional_branch(fast, fast_block, call_block)?;
        self.builder.position_at_end(fast_block);
        let value = in_place(self)?;
        let fast_end = self.current_block()?;
        self.builder.build_unconditional_branch(join)?;
        self.builder.position_at_end(call_block);
        let (called, raised) = self.call_raising(function, arguments)?;
        let called = called.ok_or_else(|| CodegenError(format!("{function:?} has a result")))?;
        let called = self.runtime_result(function.signature().result, called)?;
        let call_end = self.current_block()?;
        self.builder.build_unconditional_branch(join)?;
        self.builder.position_at_end(join);
        let result = self.builder.build_phi(value.get_type(), "")?;
        result.add_incoming(&[(&value, fast_end), (&called, call_end)]);
        let exception = self.no_exception_or(raised, fast_end, call_end)?;
        Ok((Some(result.as_basic_value()), exception))
    }

    /// Calls the runtime function `function`, which may raise, and returns its result as the
    /// runtime gives it and its exception.
    pub(crate) fn call_raising(
        &mut self,
        function: RuntimeFunction,
        arguments: &[ValueId],
    ) -> Result<Raised<'ctx>, CodegenError> {
        let callee = runtime_function(self.shared.context, self.shared.module, function);
        let arguments = self.arguments(arguments)?;
        let call = self.builder.build_call(callee, &arguments, "")?;
        if crate::operation::drops_objects(function) {
            self.close_dropped()?;
        }
        let ValueKind::Basic(returned) = call.try_as_basic_value() else {
            return Err(CodegenError(
                "a raising call returns its exception".to_owned(),
            ));
        };
        if returned.is_pointer_value() {
            return Ok((None, returned.into_pointer_value()));
        }
        let pair = returned.into_struct_value();
        let result = self.builder.build_extract_value(pair, 0, "")?;
        let exception = self
            .builder
            .build_extract_value(pair, 1, "")?
            .into_pointer_value();
        Ok((Some(result), exception))
    }

    /// Returns, where the in-place path and the call join, the call's exception, or null from
    /// the in-place path.
    pub(crate) fn no_exception_or(
        &self,
        raised: PointerValue<'ctx>,
        in_place_end: BasicBlock<'ctx>,
        call_end: BasicBlock<'ctx>,
    ) -> Result<PointerValue<'ctx>, CodegenError> {
        let pointer = self.shared.context.ptr_type(AddressSpace::default());
        let exception = self.builder.build_phi(pointer, "")?;
        exception.add_incoming(&[(&pointer.const_null(), in_place_end), (&raised, call_end)]);
        Ok(exception.as_basic_value().into_pointer_value())
    }
}
