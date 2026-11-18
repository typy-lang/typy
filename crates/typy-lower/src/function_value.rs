//! Lowers function values and the calls through them (FUN-05, ADR-0060).

use typy_check::{ClassId, FunctionId, MemberTest, TypeObjectOf, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{Callee, Constant, RuntimeFunction, TyirType};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns the function object of `function`, which captures nothing: one immortal
    /// object, so it is borrowed and using it allocates nothing.
    pub(crate) fn function_value(&mut self, function: FunctionId, span: Span) -> Lowered {
        let value = self.constant(Constant::Function(typy_tyir::FunctionId(function.0)), span);
        Self::borrowed(value, TyirType::FUNCTION)
    }

    /// Returns a new bound method of the instance `object` gives, of the method of its
    /// class among `methods`: the first, of its static class, unless the object is an
    /// instance of a class of a later entry, each after the classes it derives from, so the
    /// last test that holds names the object's own class's method.
    pub(crate) fn bound_method(
        &mut self,
        object: &TypedExpr,
        methods: &[(ClassId, FunctionId)],
        span: Span,
    ) -> Lowered {
        let instance = self.expression(object);
        let descriptor = |this: &mut Self, method: FunctionId| {
            this.constant(
                Constant::MethodDescriptor(typy_tyir::FunctionId(method.0)),
                span,
            )
        };
        let mut chosen = descriptor(self, methods[0].1);
        for (class, method) in &methods[1..] {
            let name = self.program.class(*class).name.clone();
            let test = MemberTest::Class(TypeObjectOf::Instance(*class, name));
            let found = self.member_found(instance, &test, span);
            let own = descriptor(self, *method);
            chosen = self.select(found, own, chosen, TyirType::Bits);
        }
        let bits = self.bits(Self::borrowed(instance.value, instance.ty), span);
        let made = self.call_runtime(RuntimeFunction::MethodNew, vec![chosen, bits], span);
        self.release(instance, span);
        made
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
