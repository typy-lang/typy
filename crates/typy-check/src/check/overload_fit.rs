//! Whether an `@overload` signature fits its implementation, and how a call of the
//! implementation reads it (TYP-29, ADR-0112).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::ParameterKind;

use crate::typed::{FunctionSignature, SignatureParameter};

/// How a call of an implementation reads one of its overloads: the overload's parameters,
/// each passed as the implementation's of the same name and with its default, and the
/// position of that parameter in the implementation's signature.
pub(crate) struct OverloadView {
    pub(crate) signature: FunctionSignature,
    pub(crate) positions: Vec<usize>,
}

/// Returns how calls read `overload` as a call of `implementation`, or why the
/// implementation does not take what the overload does: each of the overload's parameters
/// is the implementation's of the same name, in the same order and passed alike, whose type
/// holds the overload's; each parameter the overload leaves out, or lets a call leave out,
/// has a default in the implementation; and the implementation's result holds the
/// overload's.
pub(crate) fn fit(
    overload: &FunctionSignature,
    implementation: &FunctionSignature,
) -> Result<OverloadView, String> {
    let mut positions = Vec::with_capacity(overload.parameters.len());
    let mut parameters = Vec::with_capacity(overload.parameters.len());
    let mut next = 0;
    for parameter in &overload.parameters {
        let Some(offset) = implementation.parameters[next..]
            .iter()
            .position(|own| own.name == parameter.name)
        else {
            return Err(format!(
                "the implementation has no parameter `{}` in that place",
                parameter.name
            ));
        };
        let at = next + offset;
        if let Some(skipped) = implementation.parameters[next..at]
            .iter()
            .find(|own| !may_be_left_out(own))
        {
            return Err(format!(
                "it leaves out `{}`, which has no default in the implementation",
                skipped.name
            ));
        }
        let own = &implementation.parameters[at];
        if own.kind != parameter.kind {
            return Err(format!(
                "it passes `{}` otherwise than the implementation does",
                parameter.name
            ));
        }
        let optional = parameter.default.is_some() || parameter.default_call.is_some();
        if optional && !may_be_left_out(own) {
            return Err(format!(
                "a call may leave out `{}`, which has no default in the implementation",
                parameter.name
            ));
        }
        if !parameter.ty.is_assignable_to(&own.ty) {
            return Err(format!(
                "its `{}: {}` is not held by the implementation's `{}: {}`",
                parameter.name, parameter.ty, own.name, own.ty
            ));
        }
        parameters.push(SignatureParameter {
            name: parameter.name.clone(),
            ty: parameter.ty.clone(),
            default: if optional { own.default.clone() } else { None },
            default_call: if optional {
                own.default_call.clone()
            } else {
                None
            },
            kind: own.kind,
        });
        positions.push(at);
        next = at + 1;
    }
    if let Some(left) = implementation.parameters[next..]
        .iter()
        .find(|own| !may_be_left_out(own))
    {
        return Err(format!(
            "it leaves out `{}`, which has no default in the implementation",
            left.name
        ));
    }
    if !overload.returns.is_assignable_to(&implementation.returns) {
        return Err(format!(
            "its result, {}, is not one the implementation's {} holds",
            overload.returns, implementation.returns
        ));
    }
    Ok(OverloadView {
        signature: FunctionSignature {
            parameters,
            returns: overload.returns.clone(),
        },
        positions,
    })
}

/// Returns whether a call may leave out a parameter: one with a default, `*args` and
/// `**kwargs`.
fn may_be_left_out(parameter: &SignatureParameter) -> bool {
    parameter.default.is_some()
        || parameter.default_call.is_some()
        || matches!(
            parameter.kind,
            ParameterKind::VarPositional | ParameterKind::VarKeyword
        )
}

/// Returns the signature an overload's body is checked with, as the implementation's body
/// sees its parameters: each of the overload's type, each other the implementation's, and
/// the overload's result.
pub(crate) fn body_signature(
    view: &OverloadView,
    implementation: &FunctionSignature,
) -> FunctionSignature {
    let mut parameters = implementation.parameters.clone();
    for (parameter, at) in view.signature.parameters.iter().zip(&view.positions) {
        parameters[*at].ty = parameter.ty.clone();
    }
    FunctionSignature {
        parameters,
        returns: view.signature.returns.clone(),
    }
}

/// Returns E0432 for an overload the implementation does not take, and why.
pub(crate) fn unfit(name: &str, position: usize, reason: &str, span: Span) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(432),
        span,
        format!(
            "overload {} of `{name}` does not fit its implementation: {reason}",
            position + 1
        ),
    )
    .with_rule(RuleId::TYP_29)
    .with_help(
        "give the implementation each overload's parameters, by name and in order, with \
         types that hold the overload's and defaults where the overload's may be left out",
    )
}

#[cfg(test)]
mod tests;
