//! Checks displays that unpack (EXP-17): `[*a, b]`, `{*a, b}` and `(*a, b)` take each value
//! of an iterable as an item, and `{**d, k: v}` each entry of a dict, in order, as CPython
//! builds them.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::Expr;

use super::BodyChecker;
use crate::typed::{DictPart, SpreadKind, SpreadPart, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks a list, set or tuple display with a `*value` item. The items have one type: the
    /// one the context expects, or else the first item's; an iterable unpacked must give
    /// values of exactly that type.
    pub(super) fn spread_display(
        &mut self,
        kind: SpreadKind,
        elements: &[Expr],
        expected: Option<&Type>,
        span: Span,
    ) -> TypedExpr {
        let expected_item = match (kind, expected) {
            (SpreadKind::List, Some(Type::List(item)))
            | (SpreadKind::Set, Some(Type::Set(item)))
            | (SpreadKind::Tuple, Some(Type::TupleOf(item))) => Some(item.as_ref().clone()),
            _ => None,
        };
        let mut checked = Vec::new();
        for element in elements {
            if let Expr::Starred { value, .. } = element {
                let (source, item) = self.loop_source(value, "`*`");
                checked.push((SpreadPart::Each(Box::new(source)), item, value.span()));
            } else {
                let value = self.expression_expecting(element, expected_item.as_ref());
                let ty = value.ty.clone();
                checked.push((SpreadPart::Item(value), ty, element.span()));
            }
        }
        let item_type = expected_item
            .or_else(|| checked.first().map(|(_, ty, _)| ty.clone()))
            .unwrap_or(Type::Error);
        let name = match kind {
            SpreadKind::List => "list",
            SpreadKind::Set => "set",
            SpreadKind::Tuple => "tuple",
        };
        let mut parts = Vec::new();
        for (part, ty, at) in checked {
            let part = match part {
                SpreadPart::Item(value) => {
                    let value = Self::converted(value, Some(&item_type));
                    if !value.ty.is_assignable_to(&item_type) {
                        self.mixed_items(name, &value.ty, &item_type, at, false);
                    }
                    SpreadPart::Item(value)
                }
                SpreadPart::Each(source) => {
                    if !ty.has_error() && !item_type.has_error() && !held_alike(&ty, &item_type) {
                        self.mixed_items(name, &ty, &item_type, at, true);
                    }
                    SpreadPart::Each(source)
                }
            };
            parts.push(part);
        }
        let item = Box::new(item_type);
        let ty = match kind {
            SpreadKind::List => Type::List(item),
            SpreadKind::Set => Type::Set(item),
            SpreadKind::Tuple => Type::TupleOf(item),
        };
        Self::typed(TypedExprKind::Spread { kind, parts }, ty, span)
    }

    /// Checks a dict display with a `**mapping` item: each key and value has the dict's key and
    /// value types, the ones the context expects or else the first entry's, and each dict
    /// unpacked has exactly those.
    pub(super) fn dict_spread(
        &mut self,
        keys: &[Option<Expr>],
        values: &[Expr],
        expected: Option<&Type>,
        span: Span,
    ) -> TypedExpr {
        let (mut key_type, mut value_type) = match expected {
            Some(Type::Dict(key, value)) => {
                (Some(key.as_ref().clone()), Some(value.as_ref().clone()))
            }
            _ => (None, None),
        };
        let mut parts = Vec::new();
        for (key, value) in keys.iter().zip(values) {
            let part = if let Some(key) = key {
                let key = self.expression_expecting(key, key_type.as_ref());
                let value = self.expression_expecting(value, value_type.as_ref());
                let key_type = key_type.get_or_insert_with(|| key.ty.clone()).clone();
                let value_type = value_type.get_or_insert_with(|| value.ty.clone()).clone();
                let key = Self::converted(key, Some(&key_type));
                let value = Self::converted(value, Some(&value_type));
                if !key.ty.is_assignable_to(&key_type) || !value.ty.is_assignable_to(&value_type) {
                    self.mixed_entries(&key.ty, &value.ty, (&key_type, &value_type), key.span);
                }
                DictPart::Entry(key, value)
            } else {
                // A `frozendict` unpacks as the dict it holds (DAT-18).
                let merged = self.expression(value);
                let merged = self.frozendict_items(merged);
                match &merged.ty {
                    Type::Dict(key, value) => {
                        let key_type = key_type.get_or_insert_with(|| key.as_ref().clone());
                        let value_type = value_type.get_or_insert_with(|| value.as_ref().clone());
                        if **key != *key_type || **value != *value_type {
                            let (key_type, value_type) = (key_type.clone(), value_type.clone());
                            self.mixed_entries(key, value, (&key_type, &value_type), merged.span);
                        }
                    }
                    Type::Error => {}
                    other => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                DiagnosticCode::error(404),
                                merged.span,
                                format!(
                                    "`**` in a dict display takes a dict, and this is {}",
                                    other.with_article()
                                ),
                            )
                            .with_rule(RuleId::EXP_17),
                        );
                    }
                }
                DictPart::Merge(merged)
            };
            parts.push(part);
        }
        let ty = Type::Dict(
            Box::new(key_type.unwrap_or(Type::Error)),
            Box::new(value_type.unwrap_or(Type::Error)),
        );
        Self::typed(TypedExprKind::DictSpread(parts), ty, span)
    }

    /// Reports an item, or the values of an iterable unpacked, of another type than the
    /// collection's items.
    fn mixed_items(&mut self, name: &str, ty: &Type, item: &Type, span: Span, unpacked: bool) {
        let message = if unpacked {
            format!(
                "the items of a {name} have one type; this unpacks items of type {ty}, and the {name} holds {item}"
            )
        } else {
            format!(
                "the items of a {name} have one type; this is {}, and the {name} holds {item}",
                ty.with_article()
            )
        };
        self.diagnostics.push(
            Diagnostic::new(DiagnosticCode::error(409), span, message).with_rule(RuleId::DAT_07),
        );
    }

    /// Reports an entry, or the entries of a dict unpacked, of other types than the dict's.
    fn mixed_entries(
        &mut self,
        key: &Type,
        value: &Type,
        (keys, values): (&Type, &Type),
        span: Span,
    ) {
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(409),
                span,
                format!(
                    "the entries of a dict have one key type and one value type; these are \
                     {key} and {value}, and the dict holds {keys} and {values}"
                ),
            )
            .with_rule(RuleId::DAT_07),
        );
    }
}

/// Returns true when the values of `from` are held as the values of `to` are, so a value of an
/// iterable unpacked goes into the collection unchanged: the same type, the optional of a type
/// other than a tuple, which an optional holds as a pair, or a base class of an instance's.
pub(super) fn held_alike(from: &Type, to: &Type) -> bool {
    match (from, to) {
        _ if from == to => true,
        (_, Type::Optional(base)) if !matches!(from, Type::Tuple(_)) => held_alike(from, base),
        (Type::Instance(class), Type::Instance(base)) => class.derives_from(base),
        _ => false,
    }
}
