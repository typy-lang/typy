//! The types that annotations name: `int`, `float`, `bool`, `str`, `None`, and from 0.2
//! `tuple[...]`.

use std::collections::BTreeMap;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId};
use typy_source::syntax::{Constant, Expr};

use crate::exception::{BuiltinException, ExceptionClass};
use crate::typed::ClassId;
use crate::types::Type;

/// The classes an annotation may name. A class whose ID is below `bound` is bound where the
/// annotation is evaluated, so its bare name may be used; any class may be named in a string.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AnnotationScope<'classes> {
    pub(crate) classes: &'classes BTreeMap<String, ClassId>,
    /// The classes that derive from an exception class, whose instances are exceptions.
    pub(crate) exceptions: &'classes BTreeMap<ClassId, ExceptionClass>,
    /// For each name bound to a module at module level, that module's classes, which an
    /// annotation such as `shapes.Point` names (MOD-10).
    pub(crate) modules: &'classes BTreeMap<String, BTreeMap<String, ClassId>>,
    pub(crate) bound: u32,
}

/// Returns the type an annotation names.
pub(crate) fn annotation_type(
    annotation: &Expr,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Type {
    match annotation {
        Expr::Constant {
            value: Constant::None,
            ..
        } => Type::NoneType,
        Expr::Name { id, span } => named_type(id, *span, scope, diagnostics),
        Expr::Attribute {
            value,
            attribute,
            span,
        } if let Expr::Name { id, .. } = value.as_ref()
            && let Some(classes) = scope.modules.get(id) =>
        {
            let only = BTreeMap::from([(
                attribute.name.clone(),
                classes
                    .get(&attribute.name)
                    .copied()
                    .unwrap_or(ClassId(u32::MAX)),
            )]);
            if classes.contains_key(&attribute.name) {
                let inner = AnnotationScope {
                    classes: &only,
                    bound: u32::MAX,
                    ..*scope
                };
                return named_type(&attribute.name, *span, &inner, diagnostics);
            }
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(425),
                    attribute.span,
                    format!("the module `{id}` has no class `{}`", attribute.name),
                )
                .with_rule(RuleId::SMT_07),
            );
            Type::Error
        }
        // A string is read when the annotation is, so it may name any class (CLS-01).
        Expr::Quoted { annotation, .. } => {
            let deferred = AnnotationScope {
                bound: u32::MAX,
                ..*scope
            };
            annotation_type(annotation, &deferred, diagnostics)
        }
        Expr::Subscript { value, index, .. } if matches!(value.as_ref(), Expr::Name { id, .. } if id == "tuple") => {
            tuple_annotation(index, scope, diagnostics)
        }
        Expr::Subscript { value, index, .. } if matches!(value.as_ref(), Expr::Name { id, .. } if id == "type") =>
        {
            let of = annotation_type(index, scope, diagnostics);
            crate::types::TypeObjectOf::type_of(crate::types::TypeObjectOf::of(&of))
        }
        Expr::Subscript { value, index, .. } if matches!(value.as_ref(), Expr::Name { id, .. } if id == "list") => {
            Type::List(Box::new(annotation_type(index, scope, diagnostics)))
        }
        Expr::BinOp {
            operator: typy_source::syntax::BinaryOperator::BitOr,
            left,
            right,
            ..
        } => {
            // The subset checker lets only `T | None` and `None | T` through.
            let inner = match left.as_ref() {
                Expr::Constant {
                    value: Constant::None,
                    ..
                } => right,
                _ => left,
            };
            Type::optional(annotation_type(inner, scope, diagnostics))
        }
        Expr::Subscript { value, index, span } if matches!(value.as_ref(), Expr::Name { id, .. } if id == "dict") => {
            dict_annotation(index, *span, scope, diagnostics)
        }
        Expr::Subscript { value, index, .. } if matches!(value.as_ref(), Expr::Name { id, .. } if id == "set") =>
        {
            let item = annotation_type(index, scope, diagnostics);
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
fn named_type(
    id: &str,
    span: typy_diagnostics::Span,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Type {
    if let Some(class) = scope.classes.get(id) {
        if class.0 >= scope.bound {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(306),
                    span,
                    format!("this annotation is evaluated before the `class {id}` statement runs"),
                )
                .with_rule(RuleId::TYP_12)
                .with_help(format!("write the annotation as a string, `\"{id}\"`")),
            );
            return Type::Error;
        }
        if let Some(exception) = scope.exceptions.get(class) {
            return Type::Exception(exception.clone());
        }
        return Type::Instance(*class, id.to_owned());
    }
    match id {
        "int" => Type::Int,
        // `type` holds any type object; `type[X]` the one of `X` (CLS-12).
        "type" => Type::TypeObject(None),
        "float" => Type::Float,
        "bool" => Type::Bool,
        "str" => Type::Str,
        "bytes" => Type::Bytes,
        "range" => Type::Range,
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
        _ if let Some(class) = BuiltinException::named(id) => {
            Type::Exception(ExceptionClass::Builtin(class))
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
    scope: &AnnotationScope<'_>,
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
        annotation_type(key, scope, diagnostics),
        annotation_type(value, scope, diagnostics),
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
fn tuple_annotation(
    index: &Expr,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Type {
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
        return Type::TupleOf(Box::new(annotation_type(item, scope, diagnostics)));
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
            .map(|item| annotation_type(item, scope, diagnostics))
            .collect(),
    )
}
