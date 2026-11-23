//! Reads `typing.Literal[...]` annotations (TYP-28, ADR-0110): each value is an `int`, `str`,
//! `bytes` or `bool` constant, `None`, or another `Literal[...]`, and the type is their union.

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId};
use typy_source::syntax::{Constant, Expr};

use super::annotation::{AnnotationName, AnnotationScope, standard_name};
use crate::literal_type::LiteralValue;
use crate::types::Type;

/// Returns the type `Literal[index]` names: the union of the values `index` writes.
pub(super) fn literal_annotation(
    index: &Expr,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Type {
    let written: Vec<&Expr> = match index {
        Expr::Tuple { elements, .. } => elements.iter().collect(),
        other => vec![other],
    };
    let mut members = Vec::with_capacity(written.len());
    for each in written {
        let member = literal_member(each, scope, diagnostics);
        if member.has_error() {
            return Type::Error;
        }
        members.push(member);
    }
    Type::union(members)
}

/// Returns the type one value of `Literal[...]` names, or the error type, reported, for one
/// that is no constant `Literal` takes.
fn literal_member(
    expression: &Expr,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Type {
    let value = match expression {
        Expr::Constant {
            value: Constant::None,
            ..
        } => return Type::NoneType,
        Expr::Subscript { value, index, .. }
            if standard_name(value, scope) == Some(AnnotationName::Literal) =>
        {
            return literal_annotation(index, scope, diagnostics);
        }
        other => LiteralValue::of_written(other),
    };
    let Some(value) = value else {
        diagnostics.push(not_a_literal(expression));
        return Type::Error;
    };
    Type::literal(value)
}

/// Returns E0408 for a value of `Literal[...]` that is no constant it takes.
fn not_a_literal(expression: &Expr) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(408),
        expression.span(),
        "`Literal` takes `int`, `str`, `bytes` and `bool` constants, `None` and other \
         `Literal` types",
    )
    .with_rule(RuleId::TYP_28)
    .with_help("write the constants themselves, such as `Literal['r', 'w']` or `Literal[-1, 0]`")
}

/// Returns E0408 for `Literal` written without its values.
pub(super) fn literal_form(span: typy_diagnostics::Span) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(408),
        span,
        "a `Literal` annotation names its values, such as `Literal['r', 'w']`",
    )
    .with_rule(RuleId::TYP_28)
}
