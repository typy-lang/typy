//! Function and method definitions and their signatures (TYP-01, CLS-01).

use std::collections::BTreeMap;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Constant, Expr, Identifier, Parameter, Stmt};

use super::annotation::{AnnotationScope, annotation_type};
use crate::typed::ClassId;
use crate::typed::{FunctionSignature, SignatureParameter, TypedConstant};
use crate::types::Type;

/// A function definition with its signature: a module-level function, or a method of a class.
#[derive(Clone, Debug)]
pub(crate) struct Definition<'tree> {
    pub(crate) name: String,
    /// The name, or for a method the class's name, a dot and the name.
    pub(crate) qualified_name: String,
    /// The class of a method.
    pub(crate) class: Option<ClassId>,
    pub(crate) signature: FunctionSignature,
    pub(crate) parameters: Vec<(String, Span)>,
    pub(crate) body: &'tree [Stmt],
    pub(crate) span: Span,
}

/// The parts of a `def` statement a definition is built from.
pub(super) struct FunctionParts<'tree> {
    pub(super) name: &'tree Identifier,
    pub(super) parameters: &'tree [Parameter],
    pub(super) returns: Option<&'tree Expr>,
    pub(super) body: &'tree [Stmt],
    pub(super) span: Span,
}

impl<'tree> FunctionParts<'tree> {
    /// Returns the parts of a `def` statement, or `None` for any other statement.
    pub(super) fn of(statement: &'tree Stmt) -> Option<Self> {
        let Stmt::FunctionDef {
            name,
            parameters,
            returns,
            body,
            span,
            ..
        } = statement
        else {
            return None;
        };
        Some(Self {
            name,
            parameters,
            returns: returns.as_ref(),
            body,
            span: *span,
        })
    }
}

/// Reports a second definition of a name; TyPy keeps one definition per name.
pub(super) fn defined_twice(name: &Identifier) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(202),
        name.span,
        format!(
            "`{}` is defined more than once; TyPy keeps one definition per name",
            name.name
        ),
    )
    .with_rule(RuleId::STD_04)
}

/// Collects the module's functions in source order, with their signatures (TYP-01). Each
/// signature may name the classes whose `class` statements come before its `def`.
pub(super) fn collect_functions<'tree>(
    statements: &'tree [Stmt],
    classes: &BTreeMap<String, ClassId>,
    floor_defers: bool,
    diagnostics: &mut Diagnostics,
) -> Vec<Definition<'tree>> {
    let mut functions: Vec<Definition<'tree>> = Vec::new();
    let mut classes_before = 0;
    for statement in statements {
        if matches!(statement, Stmt::ClassDef { .. }) {
            classes_before += 1;
        }
        let Some(parts) = FunctionParts::of(statement) else {
            continue;
        };
        if functions
            .iter()
            .any(|existing| existing.name == parts.name.name)
        {
            diagnostics.push(defined_twice(parts.name));
            continue;
        }
        let scope = AnnotationScope {
            classes,
            bound: if floor_defers {
                u32::MAX
            } else {
                classes_before
            },
        };
        functions.push(definition(&parts, None, &scope, diagnostics));
    }
    functions
}

/// The class a method belongs to, which types its first parameter.
pub(super) struct Owner<'class> {
    pub(super) id: ClassId,
    pub(super) name: &'class str,
}

/// Returns the definition of a function, or of a method of `owner`, whose first parameter is
/// the instance and needs no annotation.
pub(super) fn definition<'tree>(
    parts: &FunctionParts<'tree>,
    owner: Option<&Owner<'_>>,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Definition<'tree> {
    let name = &parts.name.name;
    let mut signature_parameters = Vec::new();
    let mut parameter_names = Vec::new();
    for (index, parameter) in parts.parameters.iter().enumerate() {
        let ty = match (owner, &parameter.annotation) {
            (Some(owner), annotation) if index == 0 => {
                let instance = Type::Instance(owner.id, owner.name.to_owned());
                if let Some(annotation) = annotation {
                    let annotated = annotation_type(annotation, scope, diagnostics);
                    if !annotated.matches(&instance) {
                        diagnostics.push(
                            Diagnostic::new(
                                DiagnosticCode::error(404),
                                annotation.span(),
                                format!(
                                    "the first parameter of a method is the instance, {}",
                                    instance.with_article()
                                ),
                            )
                            .with_rule(RuleId::CLS_01),
                        );
                    }
                }
                instance
            }
            (_, Some(annotation)) => annotation_type(annotation, scope, diagnostics),
            (_, None) => {
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
            }
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
    let initializer = owner.is_some() && name == "__init__";
    let returns = match parts.returns {
        Some(annotation) => annotation_type(annotation, scope, diagnostics),
        // `__init__` returns `None`, so it may leave its return annotation out, as mypy allows.
        None if initializer => Type::NoneType,
        None => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(204),
                    parts.name.span,
                    format!("function `{name}` has no return annotation"),
                )
                .with_rule(RuleId::TYP_01)
                .with_help("write `-> None` if it returns nothing"),
            );
            Type::Error
        }
    };
    if initializer && !returns.matches(&Type::NoneType) {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(405),
                parts.returns.map_or(parts.name.span, Expr::span),
                "`__init__` returns `None`",
            )
            .with_rule(RuleId::CLS_01),
        );
    }
    Definition {
        name: name.clone(),
        qualified_name: owner
            .map_or_else(|| name.clone(), |owner| format!("{}.{name}", owner.name)),
        class: owner.map(|owner| owner.id),
        signature: FunctionSignature {
            parameters: signature_parameters,
            returns,
        },
        parameters: parameter_names,
        body: parts.body,
        span: parts.span,
    }
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
