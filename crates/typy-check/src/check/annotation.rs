//! The types that annotations name: `int`, `float`, `bool`, `str`, `None`, and from 0.2
//! `tuple[...]`.

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId};
use typy_source::syntax::{Constant, Expr};

use crate::types::Type;

/// Returns the type an annotation names.
pub(crate) fn annotation_type(annotation: &Expr, diagnostics: &mut Diagnostics) -> Type {
    match annotation {
        Expr::Constant {
            value: Constant::None,
            ..
        } => Type::NoneType,
        Expr::Name { id, span } => {
            match id.as_str() {
                "int" => Type::Int,
                "float" => Type::Float,
                "bool" => Type::Bool,
                "str" => Type::Str,
                "Any" | "object" => {
                    diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(403),
                            *span,
                            format!("`{id}` does not give the value one type"),
                        )
                        .with_rule(RuleId::TYP_10),
                    );
                    Type::Error
                }
                "tuple" => {
                    diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(408),
                            *span,
                            "a `tuple` annotation names the type of each item, such as `tuple[int, str]`",
                        )
                        .with_rule(RuleId::TYP_06),
                    );
                    Type::Error
                }
                "list" => {
                    diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(408),
                            *span,
                            "a `list` annotation names the type of its items, such as `list[int]`",
                        )
                        .with_rule(RuleId::TYP_06),
                    );
                    Type::Error
                }
                "dict" | "set" | "bytes" => {
                    diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(201),
                            *span,
                            format!("`{id}` annotations are planned for TyPy 0.2 (TYP-06)"),
                        )
                        .with_rule(RuleId::TYP_06),
                    );
                    Type::Error
                }
                _ => {
                    diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(205),
                        *span,
                        format!("`{id}` is not a type TyPy 0.1 knows; use int, float, bool, str or None"),
                    )
                    .with_rule(RuleId::STD_04),
                );
                    Type::Error
                }
            }
        }
        Expr::Subscript { value, index, .. } if matches!(value.as_ref(), Expr::Name { id, .. } if id == "tuple") => {
            tuple_annotation(index, diagnostics)
        }
        Expr::Subscript { value, index, .. } if matches!(value.as_ref(), Expr::Name { id, .. } if id == "list") =>
        {
            let item = annotation_type(index, diagnostics);
            if !item.is_list_item() {
                diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(201),
                        index.span(),
                        format!("lists of {item} are planned for TyPy 0.2 (DAT-07)"),
                    )
                    .with_rule(RuleId::DAT_07),
                );
                return Type::Error;
            }
            Type::List(Box::new(item))
        }
        other => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(205),
                    other.span(),
                    "this annotation does not name a type",
                )
                .with_rule(RuleId::STD_04),
            );
            Type::Error
        }
    }
}

/// Returns the type `tuple[...]` names: one type per item, `tuple[()]` for the empty tuple. A
/// tuple of any length, `tuple[T, ...]`, is a sequence like a list and arrives with lists.
fn tuple_annotation(index: &Expr, diagnostics: &mut Diagnostics) -> Type {
    let items: Vec<&Expr> = match index {
        Expr::Tuple { elements, .. } => elements.iter().collect(),
        single => vec![single],
    };
    if items.iter().any(|item| {
        matches!(
            item,
            Expr::Constant {
                value: Constant::Ellipsis,
                ..
            }
        )
    }) {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(201),
                index.span(),
                "`tuple[T, ...]` annotations are planned for TyPy 0.2 (TYP-06)",
            )
            .with_rule(RuleId::TYP_06),
        );
        return Type::Error;
    }
    Type::Tuple(
        items
            .into_iter()
            .map(|item| annotation_type(item, diagnostics))
            .collect(),
    )
}
