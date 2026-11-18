//! Reads `Callable[[A, ...], R]` of `typing` or `collections.abc`, the type of a function
//! value (FUN-05, ADR-0060).

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Constant, Expr};

use super::annotation::{AnnotationScope, annotation_type};
use crate::types::Type;

/// Returns the type `Callable[[A, ...], R]` names: the list of parameter types, then the
/// result type. `Callable[..., R]` takes any arguments, which a call through the value could
/// not check, so it is E0408, as is any other form.
pub(super) fn callable_annotation(
    index: &Expr,
    span: Span,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Type {
    let Expr::Tuple { elements, .. } = index else {
        diagnostics.push(callable_form(span));
        return Type::Error;
    };
    let [parameters, returns] = elements.as_slice() else {
        diagnostics.push(callable_form(span));
        return Type::Error;
    };
    let parameters = match parameters {
        Expr::List { elements, .. } => elements
            .iter()
            .map(|parameter| annotation_type(parameter, scope, diagnostics))
            .collect::<Vec<Type>>(),
        Expr::Constant {
            value: Constant::Ellipsis,
            span,
        } => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(408),
                    *span,
                    "`Callable[..., R]` takes any arguments, which TyPy cannot check",
                )
                .with_rule(RuleId::FUN_05)
                .with_help("list the parameter types, as `Callable[[int, str], bool]`"),
            );
            return Type::Error;
        }
        _ => {
            diagnostics.push(callable_form(span));
            return Type::Error;
        }
    };
    let returns = annotation_type(returns, scope, diagnostics);
    if parameters.iter().any(Type::has_error) || returns.has_error() {
        return Type::Error;
    }
    Type::Callable {
        parameters,
        returns: Box::new(returns),
    }
}

/// Returns the error for a `Callable` written in any other form than `Callable[[A, ...], R]`.
pub(super) fn callable_form(span: Span) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(408),
        span,
        "a `Callable` annotation names its parameter types and its result, such as \
         `Callable[[int, str], bool]`",
    )
    .with_rule(RuleId::FUN_05)
}
