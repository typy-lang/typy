//! Calls of a method whose signature names `typing.Self` (TYP-30, ADR-0113): `Self` is the
//! type of the instance the call is made through, so a call through an instance of a derived
//! class takes and gives instances of that class. The method runs as its own class's, given
//! each argument as its parameter holds it, and its result is read as the derived class's
//! after a test of its class.

use typy_diagnostics::Span;
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use crate::typed::{
    Argument, FunctionId, FunctionSignature, SignatureParameter, TypedExpr, TypedExprKind,
};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns the call of the method `target` through an instance of type `receiver`, the
    /// instance argument in `bound`, read through its overloads or its `Self` type; `None`
    /// when it has neither.
    pub(super) fn method_call_by_signature(
        &mut self,
        (target, signature): (FunctionId, &FunctionSignature),
        (receiver, bound): (&Type, Vec<Argument>),
        name: &str,
        given: (&[Expr], &[Keyword]),
        span: Span,
    ) -> Option<TypedExpr> {
        if let Some(call) =
            self.overloaded_method_call((target, signature), bound.clone(), name, given, span)
        {
            return Some(call);
        }
        let names_self = signature.returns.has_self()
            || signature
                .parameters
                .iter()
                .any(|parameter| parameter.ty.has_self());
        if !names_self {
            return None;
        }
        let read = FunctionSignature {
            parameters: signature
                .parameters
                .iter()
                .map(|parameter| SignatureParameter {
                    ty: parameter.ty.with_self(Some(receiver)),
                    ..parameter.clone()
                })
                .collect(),
            returns: signature.returns.with_self(Some(receiver)),
        };
        let (arguments, keywords) = given;
        let arguments = match self.bind_to(&read, 1, name, arguments, keywords, span) {
            Ok(arguments) => arguments,
            Err(error) => return Some(error),
        };
        let mut bound = bound;
        bound.extend(arguments.into_iter().map(|argument| {
            let held = signature.parameters[argument.parameter].ty.value_type();
            let held = match held {
                Type::TupleOf(item) if argument.passed == crate::typed::Passed::Position => *item,
                other => other,
            };
            Argument {
                value: Self::converted(argument.value, Some(&held)),
                ..argument
            }
        }));
        let call = Self::typed(
            TypedExprKind::Call {
                function: target,
                arguments: bound,
                dispatch: true,
            },
            signature.returns.value_type(),
            span,
        );
        let wanted = read.returns.value_type();
        if call.ty == wanted || call.ty.has_error() {
            return Some(call);
        }
        let kept = call.clone();
        Some(Self::tested_as(call, &wanted, span).unwrap_or(kept))
    }
}

/// Returns a signature whose parameters name `Self` with `Self` read as the method's own
/// class, for a call not made through an instance; `None` when no parameter names it.
pub(super) fn through_own_class(signature: &FunctionSignature) -> Option<FunctionSignature> {
    if !signature
        .parameters
        .iter()
        .any(|parameter| parameter.ty.has_self())
    {
        return None;
    }
    Some(FunctionSignature {
        parameters: signature
            .parameters
            .iter()
            .map(|parameter| SignatureParameter {
                ty: parameter.ty.with_self(None),
                ..parameter.clone()
            })
            .collect(),
        returns: signature.returns.clone(),
    })
}
