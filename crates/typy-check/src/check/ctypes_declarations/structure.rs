//! Reads a `ctypes.Structure` subclass at module level: a docstring, `pass`, and one
//! `_fields_` list of `(name, type)` pairs written in the class body (FFI-03, ADR-0049).

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Constant, Expr, Stmt};

use super::{Types, refused};
use crate::ctypes::CData;

/// The prefix of the hidden name a structure class statement binds its name to.
const HIDDEN: &str = "<structure ";

/// Returns the hidden name of the structure class numbered `id`, which no source can write.
pub(crate) fn hidden_name(id: u32) -> String {
    format!("{HIDDEN}{id}>")
}

/// Returns the number of the structure class a hidden name stands for.
pub(crate) fn hidden_id(name: &str) -> Option<u32> {
    name.strip_prefix(HIDDEN)?.strip_suffix('>')?.parse().ok()
}

/// Returns the error for a form of a structure class planned for 0.13 (FFI-11).
pub(super) fn planned(span: Span, what: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(201),
        span,
        format!("{what} is planned for TyPy 0.13 (FFI-11)"),
    )
    .with_rule(RuleId::FFI_11)
}

/// The parts of a structure class statement its fields are read from.
pub(super) struct ClassParts<'class> {
    /// The class body.
    pub(super) body: &'class [Stmt],
    /// Whether it has decorators, keywords or type parameters.
    pub(super) decorated: bool,
    /// The whole statement.
    pub(super) span: Span,
}

/// Returns the fields a structure class statement declares, each with its class, after
/// reporting each part of it TyPy does not compile.
pub(super) fn fields(
    types: &Types<'_>,
    class: &ClassParts<'_>,
    diagnostics: &mut Diagnostics,
) -> Vec<(String, CData)> {
    let body = class.body;
    if class.decorated {
        diagnostics.push(planned(
            class.span,
            "a `Structure` subclass with decorators, keywords or type parameters",
        ));
    }
    let mut listed = None;
    for (position, each) in body.iter().enumerate() {
        match each {
            Stmt::Expr {
                value:
                    Expr::Constant {
                        value: Constant::Str(_),
                        ..
                    },
                ..
            } if position == 0 => {}
            Stmt::Pass { .. } => {}
            Stmt::Assign { targets, value, .. }
                if listed.is_none()
                    && matches!(targets.as_slice(), [Expr::Name { id, .. }] if id == "_fields_") =>
            {
                listed = Some(value);
            }
            other => diagnostics.push(planned(
                other.span(),
                "a `Structure` subclass with methods or class attributes other than one \
                 `_fields_`",
            )),
        }
    }
    let Some(listed) = listed else {
        return Vec::new();
    };
    let (Expr::List { elements, .. } | Expr::Tuple { elements, .. }) = listed else {
        diagnostics.push(refused(
            listed.span(),
            "`_fields_` is a list of `(name, type)` pairs written in the class".to_owned(),
            RuleId::FFI_03,
            "list each field, as `_fields_ = [(\"x\", ctypes.c_int)]`",
        ));
        return Vec::new();
    };
    let mut fields: Vec<(String, CData)> = Vec::new();
    for element in elements {
        let Some((name, class)) = field(types, element, diagnostics) else {
            continue;
        };
        if fields.iter().any(|(each, _)| *each == name) {
            diagnostics.push(refused(
                element.span(),
                format!("the field `{name}` is listed twice"),
                RuleId::FFI_03,
                "give each field its own name",
            ));
            continue;
        }
        fields.push((name, class));
    }
    fields
}

/// Returns the name and class of one item of `_fields_`, or `None` after reporting why it is
/// not one.
fn field(
    types: &Types<'_>,
    element: &Expr,
    diagnostics: &mut Diagnostics,
) -> Option<(String, CData)> {
    match element {
        Expr::Tuple { elements, .. } if elements.len() == 3 => {
            diagnostics.push(planned(element.span(), "a bit field"));
            None
        }
        Expr::Tuple { elements, .. } if elements.len() == 2 => {
            let Expr::Constant {
                value: Constant::Str(name),
                ..
            } = &elements[0]
            else {
                diagnostics.push(refused(
                    elements[0].span(),
                    "a field is named by a string literal".to_owned(),
                    RuleId::FFI_03,
                    "write the field's name in quotes, as `(\"x\", ctypes.c_int)`",
                ));
                return None;
            };
            match types.declared(&elements[1], true) {
                Ok(class) => Some((name.clone(), class)),
                Err(diagnostic) => {
                    diagnostics.push(*diagnostic);
                    None
                }
            }
        }
        other => {
            diagnostics.push(refused(
                other.span(),
                "each item of `_fields_` is a `(name, type)` pair".to_owned(),
                RuleId::FFI_03,
                "write the field as `(\"x\", ctypes.c_int)`",
            ));
            None
        }
    }
}
