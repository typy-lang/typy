//! Module-level function definitions and their signatures (TYP-01).

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Constant, Expr, Stmt};

use super::annotation::annotation_type;
use crate::typed::{FunctionSignature, SignatureParameter, TypedConstant};
use crate::types::Type;

/// A module-level function definition with its signature.
#[derive(Clone, Debug)]
pub(crate) struct Definition<'tree> {
    pub(crate) name: String,
    pub(crate) signature: FunctionSignature,
    pub(crate) parameters: Vec<(String, Span)>,
    pub(crate) body: &'tree [Stmt],
    pub(crate) span: Span,
}

/// Collects the module's functions in source order, with their signatures (TYP-01).
pub(super) fn collect_functions<'tree>(
    statements: &'tree [Stmt],
    diagnostics: &mut Diagnostics,
) -> Vec<Definition<'tree>> {
    let mut functions: Vec<Definition<'tree>> = Vec::new();
    for statement in statements {
        let Stmt::FunctionDef {
            name,
            parameters,
            returns,
            body,
            span,
            ..
        } = statement
        else {
            continue;
        };
        if functions.iter().any(|existing| existing.name == name.name) {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(202),
                    name.span,
                    format!(
                        "`{}` is defined more than once; TyPy keeps one definition per name",
                        name.name
                    ),
                )
                .with_rule(RuleId::STD_04),
            );
            continue;
        }
        let mut signature_parameters = Vec::new();
        let mut parameter_names = Vec::new();
        for parameter in parameters {
            let ty = if let Some(annotation) = &parameter.annotation {
                annotation_type(annotation, diagnostics)
            } else {
                diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(203),
                        parameter.span,
                        format!("parameter `{}` has no type annotation", parameter.name.name),
                    )
                    .with_rule(RuleId::TYP_01)
                    .with_help(format!(
                        "write `{}: int`, or the type it holds",
                        parameter.name.name
                    )),
                );
                Type::Error
            };
            let default = parameter
                .default
                .as_ref()
                .and_then(|default| default_value(default, &ty, diagnostics));
            signature_parameters.push(SignatureParameter {
                name: parameter.name.name.clone(),
                ty,
                default,
            });
            parameter_names.push((parameter.name.name.clone(), parameter.name.span));
        }
        let returns = if let Some(annotation) = returns {
            annotation_type(annotation, diagnostics)
        } else {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(204),
                    name.span,
                    format!("function `{}` has no return annotation", name.name),
                )
                .with_rule(RuleId::TYP_01)
                .with_help("write `-> None` if it returns nothing"),
            );
            Type::Error
        };
        functions.push(Definition {
            name: name.name.clone(),
            signature: FunctionSignature {
                parameters: signature_parameters,
                returns,
            },
            parameters: parameter_names,
            body,
            span: *span,
        });
    }
    functions
}

/// Returns the constant of a default value, checked against the parameter's type (TYP-09).
fn default_value(
    default: &Expr,
    ty: &Type,
    diagnostics: &mut Diagnostics,
) -> Option<TypedConstant> {
    let Expr::Constant { value, span } = default else {
        return None;
    };
    let constant = match value {
        Constant::None => TypedConstant::None,
        Constant::Bool(value) => TypedConstant::Bool(*value),
        Constant::Int(digits) => TypedConstant::Int(digits.clone()),
        Constant::Float(value) => TypedConstant::Float(*value),
        Constant::Str(value) => TypedConstant::Str(value.clone()),
        Constant::Complex { .. } | Constant::Bytes(_) | Constant::Ellipsis => return None,
    };
    if !constant.ty().is_assignable_to(ty) {
        let mut diagnostic = Diagnostic::new(
            DiagnosticCode::error(404),
            *span,
            format!(
                "the default value is {}, but the parameter is {}",
                constant.ty().with_article(),
                ty.with_article()
            ),
        )
        .with_rule(if *ty == Type::Float {
            RuleId::TYP_09
        } else {
            RuleId::TYP_03
        });
        if let (TypedConstant::Int(digits), Type::Float) = (&constant, ty) {
            diagnostic = diagnostic.with_fix(*span, format!("{digits}.0"));
        }
        diagnostics.push(diagnostic);
    }
    Some(constant)
}
