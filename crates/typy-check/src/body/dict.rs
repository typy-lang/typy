//! Checks dicts: displays, `dict()` and `dict(source)`, entries read and assigned, and the
//! arguments of the methods that take pairs (DAT-09, TYP-05).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use crate::check::unhashable_key;
use crate::typed::{AssignTarget, LoopSource, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks a dict display: every key has the type of the first, or the expected key type,
    /// with a hash, and every value the type of the first, or the expected value type. An empty
    /// display has the expected type, and needs one (TYP-05).
    pub(super) fn dict_display(
        &mut self,
        keys: &[Option<Expr>],
        values: &[Expr],
        expected: Option<&Type>,
        span: Span,
    ) -> TypedExpr {
        let (expected_key, expected_value) = match expected {
            Some(Type::Dict(key, value)) => {
                (Some(key.as_ref().clone()), Some(value.as_ref().clone()))
            }
            // The expected type already has its error, so the display gives no second one.
            Some(expected) if expected.has_error() => (Some(Type::Error), Some(Type::Error)),
            _ => (None, None),
        };
        // The subset checker refuses a `**` item before types are checked, so every key is
        // present here.
        let keys: Vec<Expr> = keys.iter().flatten().cloned().collect();
        let keys = self.display_items(&keys, expected_key.as_ref());
        let values = self.display_items(values, expected_value.as_ref());
        let types = expected_key.zip(expected_value).or_else(|| {
            Some((
                super::list::joined_type(&keys)?,
                super::list::joined_type(&values)?,
            ))
        });
        let Some((key_type, value_type)) = types else {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(410),
                    span,
                    "an empty dict needs an annotation that names its key and value types",
                )
                .with_rule(RuleId::TYP_05)
                .with_help("annotate the variable, such as `counts: dict[str, int] = {}`"),
                span,
            );
        };
        let keys = super::list::converted_all(keys, &key_type);
        let values = super::list::converted_all(values, &value_type);
        for (items, ty, what) in [(&keys, &key_type, "keys"), (&values, &value_type, "values")] {
            for item in items {
                if !item.ty.is_assignable_to(ty) {
                    self.diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(409),
                            item.span,
                            format!(
                                "the {what} of a dict have one type; this is {}, and the dict holds {ty}",
                                item.ty.with_article()
                            ),
                        )
                        .with_rule(RuleId::DAT_09),
                    );
                }
            }
        }
        if !key_type.is_hashable() {
            return self.error(unhashable_key(&key_type, span), span);
        }
        Self::typed(
            TypedExprKind::Dict { keys, values },
            Type::Dict(Box::new(key_type), Box::new(value_type)),
            span,
        )
    }

    /// Checks `dict()`, an empty dict of the expected type, or `dict(source)`: a copy of a dict,
    /// or a dict of the `(key, value)` pairs a loop over the source gives.
    pub(super) fn call_dict(
        &mut self,
        arguments: &[Expr],
        keywords: &[Keyword],
        expected: Option<&Type>,
        span: Span,
    ) -> TypedExpr {
        if let Some(keyword) = keywords.first() {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    keyword.span,
                    "keyword arguments of `dict` are planned for TyPy 0.3 (DAT-16)",
                )
                .with_rule(RuleId::DAT_16),
                span,
            );
        }
        match arguments {
            [] => self.dict_display(&[], &[], expected, span),
            [source] => {
                let Some((source, ty)) = self.pairs_type(source, "`dict`") else {
                    return Self::invalid_dict(span);
                };
                Self::typed(TypedExprKind::DictOf(Box::new(source)), ty, span)
            }
            _ => self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!(
                        "`dict` takes at most 1 argument, and {} are given",
                        arguments.len()
                    ),
                )
                .with_rule(RuleId::LIB_02),
                span,
            ),
        }
    }

    /// Checks a source of `(key, value)` pairs, as `dict(source)` and `dict.update` take: a
    /// dict, or anything a loop reads two-item tuples from, whose key has a hash. Returns the
    /// source and the dict type it makes, or `None` after reporting why it cannot.
    fn pairs_type(&mut self, source: &Expr, user: &str) -> Option<(LoopSource, Type)> {
        let span = source.span();
        let (source, item) = self.loop_source(source, user);
        if item.has_error() {
            return None;
        }
        if let LoopSource::Items(sequence) = &source
            && let Type::Dict(..) = &sequence.ty
        {
            let ty = sequence.ty.clone();
            return Some((source, ty));
        }
        let Type::Tuple(parts) = &item else {
            self.diagnostics.push(not_pairs(user, &item, span));
            return None;
        };
        let [key, value] = parts.as_slice() else {
            self.diagnostics.push(not_pairs(user, &item, span));
            return None;
        };
        if !key.is_hashable() {
            self.diagnostics.push(unhashable_key(key, span));
            return None;
        }
        let ty = Type::Dict(Box::new(key.clone()), Box::new(value.clone()));
        Some((source, ty))
    }

    /// Checks the argument of `dict.update`, a dict or any source of `(key, value)` pairs of
    /// the receiver's types: a dict, a list or a `tuple[T, ...]` is passed as it is, and any
    /// other source becomes a list of its pairs.
    pub(super) fn pairs_argument(
        &mut self,
        argument: &Expr,
        key: &Type,
        value: &Type,
        what: &str,
    ) -> TypedExpr {
        let span = argument.span();
        let wanted = Type::Dict(Box::new(key.clone()), Box::new(value.clone()));
        if matches!(argument, Expr::Dict { .. }) || self.is_empty_dict_call(argument) {
            let typed = self.expression_expecting(argument, Some(&wanted));
            self.expect(&typed, wanted, what);
            return typed;
        }
        if let Expr::List { .. } = argument {
            let pairs = Type::List(Box::new(Type::Tuple(vec![key.clone(), value.clone()])));
            let typed = self.expression_expecting(argument, Some(&pairs));
            self.expect(&typed, pairs, what);
            return typed;
        }
        let Some((source, ty)) = self.pairs_type(argument, what) else {
            return Self::invalid_dict(span);
        };
        if !ty.is_assignable_to(&wanted) {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!(
                        "{what} must give the items of a {wanted}, and this gives those of a {ty}"
                    ),
                )
                .with_rule(RuleId::DAT_09),
                span,
            );
        }
        let pair = Type::Tuple(vec![key.clone(), value.clone()]);
        match source {
            LoopSource::Items(sequence)
                if matches!(sequence.ty, Type::Dict(..))
                    || sequence.ty.sequence_item().is_some() =>
            {
                sequence
            }
            source => Self::typed(
                TypedExprKind::ListOf(Box::new(source)),
                Type::List(Box::new(pair)),
                span,
            ),
        }
    }

    /// Checks `dict[key]`: the key has the dict's key type, and the entry its value type.
    pub(super) fn dict_item(
        &mut self,
        dict: TypedExpr,
        key_type: &Type,
        value_type: Type,
        key: &Expr,
        span: Span,
    ) -> TypedExpr {
        let key = self.expression_expecting(key, Some(key_type));
        self.expect(&key, key_type.clone(), "a key");
        Self::typed(
            TypedExprKind::DictItem {
                dict: Box::new(dict),
                key: Box::new(key),
            },
            value_type,
            span,
        )
    }

    /// Checks the target `dict[key] = value`, whose value has type `ty`.
    pub(super) fn entry_target(
        &mut self,
        dict: TypedExpr,
        key: TypedExpr,
        ty: &Type,
        span: Span,
    ) -> Option<AssignTarget> {
        let Type::Dict(key_type, value_type) = &dict.ty else {
            return None;
        };
        self.expect(&key, key_type.as_ref().clone(), "a key");
        if !ty.is_assignable_to(value_type) {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(402),
                    span,
                    format!("a value of {} is assigned {}", dict.ty, ty.with_article()),
                )
                .with_rule(RuleId::TYP_03),
            );
        }
        Some(AssignTarget::Entry {
            dict: Box::new(dict),
            key: Box::new(key),
        })
    }

    fn invalid_dict(span: Span) -> TypedExpr {
        Self::typed(
            TypedExprKind::Constant(TypedConstant::None),
            Type::Error,
            span,
        )
    }
}

/// Returns the error for a source of `dict` that does not give `(key, value)` pairs.
fn not_pairs(user: &str, item: &Type, span: Span) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(404),
        span,
        format!(
            "{user} needs `(key, value)` pairs, and this gives {}",
            item.with_article()
        ),
    )
    .with_rule(RuleId::DAT_09)
}
