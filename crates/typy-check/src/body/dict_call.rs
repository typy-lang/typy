//! Checks `dict` with keyword arguments and `dict.fromkeys`, with CPython's results (DAT-16).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use crate::check::unhashable_key;
use crate::typed::{TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks `dict(name=value, ...)` and `dict(source, name=value, ...)`: a dict of `str` keys
    /// with the source's pairs first, then each keyword in order, as CPython's `dict_vectorcall`
    /// sets them. Every value has the source's value type, or the expected one, or one type.
    pub(super) fn dict_with_keywords(
        &mut self,
        arguments: &[Expr],
        keywords: &[Keyword],
        expected: Option<&Type>,
        span: Span,
    ) -> TypedExpr {
        let source = match arguments {
            [] => None,
            [source] => match self.pairs_type(source, "`dict`") {
                Some(pairs) => Some(pairs),
                None => return Self::no_dict(span),
            },
            _ => return self.dict_arity(arguments.len(), span),
        };
        let mut names: Vec<String> = Vec::new();
        // The subset checker refuses `**` before types are checked, so each has a name.
        for (keyword, name) in keywords
            .iter()
            .filter_map(|keyword| Some((keyword, keyword.name.as_ref()?)))
        {
            if names.contains(&name.name) {
                return self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        keyword.span,
                        format!("`{}` is given twice", name.name),
                    )
                    .with_rule(RuleId::LIB_02),
                    span,
                );
            }
            names.push(name.name.clone());
        }
        let known = match (&source, expected) {
            (Some((_, Type::Dict(key, value))), _) => {
                if **key != Type::Str {
                    return self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(404),
                            keywords[0].span,
                            format!(
                                "keyword arguments give `dict` str keys, and this dict has {key} keys"
                            ),
                        )
                        .with_rule(RuleId::DAT_16),
                        span,
                    );
                }
                Some(value.as_ref().clone())
            }
            (_, Some(Type::Dict(key, value))) if **key == Type::Str => Some(value.as_ref().clone()),
            _ => None,
        };
        let expressions: Vec<Expr> = keywords
            .iter()
            .map(|keyword| keyword.value.clone())
            .collect();
        let values = self.display_items(&expressions, known.as_ref());
        // A keyword is given, so its value gives the type when nothing else does.
        let value_type = known
            .or_else(|| super::list::joined_type(&values))
            .unwrap_or(Type::Error);
        let values = super::list::converted_all(values, &value_type);
        for value in &values {
            if !value.ty.is_assignable_to(&value_type) {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(409),
                        value.span,
                        format!(
                            "the values of a dict have one type; this is {}, and the dict holds {value_type}",
                            value.ty.with_article()
                        ),
                    )
                    .with_rule(RuleId::DAT_09),
                );
            }
        }
        Self::typed(
            TypedExprKind::DictWith {
                source: source.map(|(source, _)| Box::new(source)),
                entries: names.into_iter().zip(values).collect(),
            },
            Type::Dict(Box::new(Type::Str), Box::new(value_type)),
            span,
        )
    }

    /// Checks `dict.fromkeys(source)` and `dict.fromkeys(source, value)`: a dict of each item
    /// of the source as a key, mapped to `None` or to the one value given, in order. The
    /// parameters are positional only, as in CPython.
    pub(super) fn call_fromkeys(
        &mut self,
        arguments: &[Expr],
        keywords: &[Keyword],
        expected: Option<&Type>,
        span: Span,
    ) -> TypedExpr {
        if let Some(keyword) = keywords.first() {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    keyword.span,
                    "`dict.fromkeys` takes no keyword arguments",
                )
                .with_rule(RuleId::DAT_16),
                span,
            );
        }
        let (source, value) = match arguments {
            [source] => (source, None),
            [source, value] => (source, Some(value)),
            _ => {
                return self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        span,
                        format!(
                            "`dict.fromkeys` takes 1 or 2 arguments, and {} are given",
                            arguments.len()
                        ),
                    )
                    .with_rule(RuleId::DAT_16),
                    span,
                );
            }
        };
        let (source, key) = self.loop_source(source, "`dict.fromkeys`");
        let expected_value = match expected {
            Some(Type::Dict(_, value)) => Some(value.as_ref().clone()),
            _ => None,
        };
        let value = value.map(|value| {
            let typed = self.expression_expecting(value, expected_value.as_ref());
            Self::converted(typed, expected_value.as_ref())
        });
        if key.has_error() || value.as_ref().is_some_and(|value| value.ty.has_error()) {
            return Self::no_dict(span);
        }
        if !key.is_hashable() {
            return self.error(unhashable_key(&key, span), span);
        }
        let value_type = value
            .as_ref()
            .map_or(Type::NoneType, |value| value.ty.clone());
        Self::typed(
            TypedExprKind::DictFromKeys {
                source: Box::new(source),
                value: value.map(Box::new),
            },
            Type::Dict(Box::new(key), Box::new(value_type)),
            span,
        )
    }

    /// Reports `dict` with more than one argument.
    pub(super) fn dict_arity(&mut self, given: usize, span: Span) -> TypedExpr {
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(404),
                span,
                format!("`dict` takes at most 1 argument, and {given} are given"),
            )
            .with_rule(RuleId::LIB_02),
            span,
        )
    }

    /// Returns the expression of a dict call whose error is already reported.
    fn no_dict(span: Span) -> TypedExpr {
        Self::typed(
            TypedExprKind::Constant(TypedConstant::None),
            Type::Error,
            span,
        )
    }
}
