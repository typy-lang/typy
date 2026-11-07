//! The types that annotations name in 0.1: `int`, `float`, `bool`, `str` and `None`.

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId};
use typy_source::syntax::{Constant, Expr};

use crate::types::Type;

/// Returns the type an annotation names; 0.1 knows `int`, `float`, `bool`, `str` and `None`.
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
                "list" | "dict" | "set" | "tuple" | "bytes" => {
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
