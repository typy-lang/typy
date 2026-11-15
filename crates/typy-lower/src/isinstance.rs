//! Lowers `isinstance` tests and the narrowed reads they allow (CLS-09, TYP-07).

use typy_check::{IsInstanceTest, Type, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{BoolOperator, Constant, Operation, RuntimeFunction, TyirType};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns `isinstance(value, classes)`: `None` gives what the checker found for it, and
    /// any other value is an instance when every value of its static type is one, or when
    /// the runtime finds its class derives from one of the classes left to check.
    pub(crate) fn isinstance(&mut self, test: &IsInstanceTest, span: Span) -> Lowered {
        let lowered = self.expression(&test.value);
        let result = if matches!(test.value.ty, Type::Optional(_)) {
            let none_matches = test.none_matches;
            self.optional_branch(
                lowered,
                TyirType::Bool,
                span,
                |this| {
                    let value = this.constant(Constant::Bool(none_matches), span);
                    Self::borrowed(value, TyirType::Bool)
                },
                |this, held| this.instance_test(held, test, span),
            )
        } else {
            self.instance_test(Self::borrowed(lowered.value, lowered.ty), test, span)
        };
        self.release(lowered, span);
        result
    }

    /// Returns whether `value`, which is not `None`, is an instance of one of the classes.
    fn instance_test(&mut self, value: Lowered, test: &IsInstanceTest, span: Span) -> Lowered {
        // A tagged union is tested as the member it holds (TYP-14).
        if self.is_tagged(value.ty) {
            return self.tag_switch(value, TyirType::Bool, span, |this, position, slot| {
                let (always, checks) = &test.members[position];
                this.classes_test(slot, *always, checks, span)
            });
        }
        self.classes_test(value, test.always, &test.checks, span)
    }

    /// Returns whether `value`, which is not `None`, is an instance: always, or when the
    /// runtime finds its class derives from one of `checks`.
    fn classes_test(
        &mut self,
        value: Lowered,
        always: bool,
        checks: &[typy_check::TypeObjectOf],
        span: Span,
    ) -> Lowered {
        if always || checks.is_empty() {
            let constant = self.constant(Constant::Bool(always), span);
            return Self::borrowed(constant, TyirType::Bool);
        }
        let mut result = None;
        for class in checks {
            let object = self.bits(Self::borrowed(value.value, value.ty), span);
            let class = self.type_object(class, span).value;
            let found = self
                .call_runtime(RuntimeFunction::IsInstance, vec![object, class], span)
                .value;
            result = Some(match result {
                None => found,
                Some(before) => self.builder.push_value(
                    Operation::BoolBinary {
                        operator: BoolOperator::Or,
                        left: before,
                        right: found,
                    },
                    TyirType::Bool,
                    span,
                ),
            });
        }
        let result = result.unwrap_or_else(|| self.constant(Constant::Bool(false), span));
        Self::borrowed(result, TyirType::Bool)
    }

    /// Returns an instance, which a test showed is of a class derived from its static class,
    /// as one of type `ty`; the result takes over the value's reference.
    pub(crate) fn downcast(&mut self, inner: &TypedExpr, ty: TyirType, span: Span) -> Lowered {
        let value = self.expression(inner);
        if value.ty == ty {
            return value;
        }
        let downcast = self.builder.push_value(
            Operation::Downcast {
                value: value.value,
                ty,
            },
            ty,
            span,
        );
        self.transfer(value, downcast, ty)
    }
}
