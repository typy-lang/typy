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
        Expr::Name { id, span } => named_type(id, *span, diagnostics),
        Expr::Subscript { value, index, .. } if matches!(value.as_ref(), Expr::Name { id, .. } if id == "tuple") => {
            tuple_annotation(index, diagnostics)
        }
        Expr::Subscript { value, index, .. } if matches!(value.as_ref(), Expr::Name { id, .. } if id == "list") => {
            Type::List(Box::new(annotation_type(index, diagnostics)))
        }
        Expr::Subscript { value, index, span } if matches!(value.as_ref(), Expr::Name { id, .. } if id == "dict") => {
            dict_annotation(index, *span, diagnostics)
        }
        Expr::Subscript { value, index, .. } if matches!(value.as_ref(), Expr::Name { id, .. } if id == "set") =>
        {
            let item = annotation_type(index, diagnostics);
            if !item.is_hashable() {
                diagnostics.push(unhashable(&item, index.span()));
                return Type::Error;
            }
            Type::Set(Box::new(item))
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

/// Returns the type a bare name names: a scalar, or an error for a name that is not a type or
/// needs its item types.
fn named_type(id: &str, span: typy_diagnostics::Span, diagnostics: &mut Diagnostics) -> Type {
    match id {
        "int" => Type::Int,
        "float" => Type::Float,
        "bool" => Type::Bool,
        "str" => Type::Str,
        "bytes" => Type::Bytes,
        "Any" | "object" => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(403),
                    span,
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
                    span,
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
                    span,
                    "a `list` annotation names the type of its items, such as `list[int]`",
                )
                .with_rule(RuleId::TYP_06),
            );
            Type::Error
        }
        "set" => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(408),
                    span,
                    "a `set` annotation names the type of its items, such as `set[int]`",
                )
                .with_rule(RuleId::TYP_06),
            );
            Type::Error
        }
        "dict" => {
            diagnostics.push(dict_arity(span));
            Type::Error
        }
        _ => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(205),
                    span,
                    format!(
                        "`{id}` is not a type TyPy 0.1 knows; use int, float, bool, str or None"
                    ),
                )
                .with_rule(RuleId::STD_04),
            );
            Type::Error
        }
    }
}

/// Returns the type `dict[K, V]` names: two types, and a key type with a hash.
fn dict_annotation(
    index: &Expr,
    span: typy_diagnostics::Span,
    diagnostics: &mut Diagnostics,
) -> Type {
    let Expr::Tuple { elements, .. } = index else {
        diagnostics.push(dict_arity(span));
        return Type::Error;
    };
    let [key, value] = elements.as_slice() else {
        diagnostics.push(dict_arity(span));
        return Type::Error;
    };
    let (key_type, value_type) = (
        annotation_type(key, diagnostics),
        annotation_type(value, diagnostics),
    );
    if !key_type.is_hashable() {
        diagnostics.push(unhashable_key(&key_type, key.span()));
        return Type::Error;
    }
    Type::Dict(Box::new(key_type), Box::new(value_type))
}

fn dict_arity(span: typy_diagnostics::Span) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(408),
        span,
        "a `dict` annotation names the types of its keys and values, such as `dict[str, int]`",
    )
    .with_rule(RuleId::TYP_06)
}

/// Returns the error for an item of a set, or a key of a dict, whose type has no hash.
pub(crate) fn unhashable(item: &Type, span: typy_diagnostics::Span) -> Diagnostic {
    unhashable_as(item, span, "an item of a set", RuleId::DAT_10)
}

/// Returns the error for a key of a dict whose type has no hash.
pub(crate) fn unhashable_key(key: &Type, span: typy_diagnostics::Span) -> Diagnostic {
    unhashable_as(key, span, "a key of a dict", RuleId::DAT_09)
}

fn unhashable_as(
    item: &Type,
    span: typy_diagnostics::Span,
    place: &str,
    rule: RuleId,
) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(413),
        span,
        format!(
            "{} cannot be {place}, because it has no hash",
            item.with_article()
        ),
    )
    .with_rule(rule)
    .with_help("a list, a set or a dict changes, so it has no hash; use a tuple of its items")
}

/// Returns the type `tuple[...]` names: one type per item, `tuple[()]` for the empty tuple. A
/// tuple of any length, `tuple[T, ...]`, is a sequence like a list and arrives with lists.
fn tuple_annotation(index: &Expr, diagnostics: &mut Diagnostics) -> Type {
    let items: Vec<&Expr> = match index {
        Expr::Tuple { elements, .. } => elements.iter().collect(),
        single => vec![single],
    };
    let is_ellipsis = |item: &Expr| {
        matches!(
            item,
            Expr::Constant {
                value: Constant::Ellipsis,
                ..
            }
        )
    };
    if let [item, ellipsis] = items.as_slice()
        && is_ellipsis(ellipsis)
        && !is_ellipsis(item)
    {
        return Type::TupleOf(Box::new(annotation_type(item, diagnostics)));
    }
    if items.iter().any(|item| is_ellipsis(item)) {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(408),
                index.span(),
                "`...` in a tuple annotation follows its only item type, as in `tuple[int, ...]`",
            )
            .with_rule(RuleId::DAT_14),
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
