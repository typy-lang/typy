//! Calls of a function whose result is `TypeGuard[T]` or `TypeIs[T]` (TYP-31, ADR-0114): the
//! call is the `isinstance` test of its first argument, a variable, against `T`, whose result
//! is the call's, so a test of it narrows the variable as `isinstance` does. The test runs
//! too, and a result it does not bear out raises `TypeError`, so a narrowed read never takes
//! a value of another type.

use super::BodyChecker;
use crate::guard_type::GuardKind;
use crate::typed::{GuardCall, TypedExpr, TypedExprKind};
use crate::types::{Type, TypeObjectOf};

impl BodyChecker<'_> {
    /// Returns `call`, a call of the function `name`, as the guarded test of its first
    /// argument when the function's result is a guard and that argument is a variable, and
    /// as it is otherwise.
    pub(super) fn guarded_call(
        &mut self,
        call: TypedExpr,
        name: &str,
        returns: &Type,
    ) -> TypedExpr {
        let Type::Guard(guard) = returns else {
            return call;
        };
        let TypedExprKind::Call { arguments, .. } = &call.kind else {
            return call;
        };
        let Some(argument) = arguments.iter().find(|argument| argument.parameter == 0) else {
            return call;
        };
        // The argument is held as the parameter holds it, such as an `object`; the test reads
        // the variable itself.
        let mut value = &argument.value;
        while let TypedExprKind::Upcast(inner) | TypedExprKind::Wrap(inner) = &value.kind {
            value = inner;
        }
        if super::flow::narrowable_variable(value).is_none() {
            return call;
        }
        let value = value.clone();
        let classes: Vec<TypeObjectOf> = match &guard.target {
            Type::Optional(inner) => std::iter::once(TypeObjectOf::NoneType)
                .chain(inner.members().iter().filter_map(TypeObjectOf::of))
                .collect(),
            other => other
                .members()
                .iter()
                .filter_map(TypeObjectOf::of)
                .collect(),
        };
        let span = call.span;
        let mut tested = self.isinstance_test(value, &classes, span);
        let exact = guard.kind == GuardKind::TypeIs;
        if let TypedExprKind::IsInstance(test) = &mut tested.kind {
            // Only `TypeIs` narrows where the call is false.
            if !exact {
                test.otherwise = None;
            }
            test.guard = Some(Box::new(GuardCall {
                call,
                exact,
                shown: format!("the {guard} function {name}"),
            }));
        }
        tested
    }
}
