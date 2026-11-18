//! Lowers function values and the calls through them (FUN-05, ADR-0060).

use typy_check::{FunctionId, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{Callee, Constant, TyirType};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns the function object of `function`, which captures nothing: one immortal
    /// object, so it is borrowed and using it allocates nothing.
    pub(crate) fn function_value(&mut self, function: FunctionId, span: Span) -> Lowered {
        let value = self.constant(Constant::Function(typy_tyir::FunctionId(function.0)), span);
        Self::borrowed(value, TyirType::FUNCTION)
    }

    /// Calls the code of the function object `callee` gives with `arguments`, and returns
    /// the result as an owned value. A `Callable` type does not say whether its function
    /// raises, so the call always takes the raising form. The function object and each
    /// argument are held for the call as those of a call by name are, so the call cannot
    /// free the object it runs, nor an argument it reads, by assigning where it was read.
    pub(crate) fn call_value(
        &mut self,
        callee: &TypedExpr,
        arguments: &[TypedExpr],
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let function = self.expression(callee);
        let function = self.kept_for_call(callee, function, span);
        let lowered: Vec<Lowered> = arguments
            .iter()
            .map(|argument| {
                let value = self.expression(argument);
                self.kept_for_call(argument, value, span)
            })
            .collect();
        let values = std::iter::once(function.value)
            .chain(lowered.iter().map(|argument| argument.value))
            .collect();
        let value = self.invoke(Callee::Value, values, ty, span);
        let result = self.produced(value, ty);
        for argument in lowered.into_iter().rev() {
            self.release(argument, span);
        }
        self.release(function, span);
        result
    }
}
