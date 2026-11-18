//! Checks the `key` argument of `sorted`, `list.sort`, `min` and `max` (FUN-05): a function
//! value of one parameter that takes each item as it is, whose results order.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Constant, Expr, Keyword};

use super::BodyChecker;
use crate::typed::{KeyedOperation, TypedExpr, TypedExprKind};
use crate::types::Type;

/// What a call's `key` argument gives.
pub(super) enum Key {
    /// `key=None`, which CPython reads as no key.
    Absent,
    /// A function value of the items.
    Function(TypedExpr),
    /// A key that was refused, with its diagnostic.
    Refused,
}

impl BodyChecker<'_> {
    /// Returns the checked `key` of `name` over items of type `item`: `None`, or a function
    /// value that takes an item as it is held, whose results `<` orders. Another value is
    /// E0404, and results that do not order are E0407.
    pub(super) fn key_argument(&mut self, key: &Expr, name: &str, item: &Type) -> Key {
        if let Expr::Constant {
            value: Constant::None,
            ..
        } = key
        {
            return Key::Absent;
        }
        let value = self.expression(key);
        if value.ty.has_error() {
            return Key::Refused;
        }
        let Some((parameters, returns)) = value.ty.callable_parts() else {
            self.key_error(&value, name, item);
            return Key::Refused;
        };
        let returns = returns.clone();
        let wanted = Type::Callable {
            parameters: vec![item.clone()],
            returns: Box::new(returns.clone()),
        };
        if parameters.len() != 1 || !value.ty.is_assignable_to(&wanted) {
            self.key_error(&value, name, item);
            return Key::Refused;
        }
        if !returns.can_order(&returns) {
            self.comparison_error(&returns, &returns, value.span);
            return Key::Refused;
        }
        Key::Function(Self::converted(value, Some(&wanted)))
    }

    /// Reports a `key` that is not a function of one item.
    fn key_error(&mut self, value: &TypedExpr, name: &str, item: &Type) {
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(404),
                value.span,
                format!(
                    "the `key` of `{name}` is called with each item, of type {item}, and this is \
                     {}",
                    value.ty.with_article()
                ),
            )
            .with_rule(RuleId::FUN_05)
            .with_help(format!("give a function of one parameter of type {item}")),
        );
    }

    /// Returns the call `operation` of `list`, a list of items of type `item`, by `key`, with
    /// its option, `reverse` or `default`, of the type `result`.
    pub(super) fn keyed(
        operation: KeyedOperation,
        list: TypedExpr,
        key: TypedExpr,
        (option, option_first): (Option<TypedExpr>, bool),
        result: Type,
        span: Span,
    ) -> TypedExpr {
        Self::typed(
            TypedExprKind::Keyed {
                operation,
                list: Box::new(list),
                key: Box::new(key),
                option: option.map(Box::new),
                option_first,
            },
            result,
            span,
        )
    }

    /// Returns `list.sort(key=..., reverse=...)` of `list`, the receiver: no positional
    /// argument, the key, and `reverse`, a `bool`. Anything else is E0404, as CPython raises
    /// `TypeError` for it (LIB-03).
    pub(super) fn keyed_sort(
        &mut self,
        list: TypedExpr,
        arguments: &[Expr],
        keywords: &[Keyword],
        key_position: usize,
        span: Span,
    ) -> TypedExpr {
        let mut reverse = None;
        for (position, keyword) in keywords.iter().enumerate() {
            let name = keyword.name.as_ref().map(|name| name.name.as_str());
            let message = match name {
                _ if position == key_position => continue,
                // A keyword written twice is a syntax error the subset check reports.
                Some("reverse") => {
                    reverse = Some((&keyword.value, position < key_position));
                    continue;
                }
                Some(name) => format!("`sort` has no parameter named `{name}`"),
                None => "`sort` takes no `**` arguments".to_owned(),
            };
            return self.error(
                Diagnostic::new(DiagnosticCode::error(404), keyword.span, message)
                    .with_rule(RuleId::LIB_03),
                span,
            );
        }
        if let Some(argument) = arguments.first() {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    argument.span(),
                    "`sort` takes no positional arguments",
                )
                .with_rule(RuleId::LIB_03),
                span,
            );
        }
        let item = list.ty.sequence_item().cloned().unwrap_or(Type::Error);
        let key = self.key_argument(&keywords[key_position].value, "sort", &item);
        let reverse = reverse.map(|(value, first)| {
            let value = self.expression_expecting(value, Some(&Type::Bool));
            self.expect(&value, Type::Bool, "`reverse`");
            (value, first)
        });
        let Key::Function(key) = key else {
            return Self::typed(
                TypedExprKind::Constant(crate::typed::TypedConstant::None),
                Type::Error,
                span,
            );
        };
        let (reverse, first) = reverse.map_or((None, false), |(value, first)| (Some(value), first));
        Self::keyed(
            KeyedOperation::Sort,
            list,
            key,
            (reverse, first),
            Type::NoneType,
            span,
        )
    }

    /// Returns `min(a, b, ..., key=f)` or `max`, the values of one type, as the call over
    /// the list of them (FUN-05).
    pub(super) fn keyed_values(
        &mut self,
        largest: bool,
        values: Vec<TypedExpr>,
        key: &Expr,
        span: Span,
    ) -> TypedExpr {
        let name = if largest { "max" } else { "min" };
        let item = values[0].ty.clone();
        if let Some(other) = values.iter().find(|value| value.ty != item) {
            let message = format!(
                "the arguments of `{name}` must have one type, because CPython returns one of \
                 them unchanged; they are {item} and {}",
                other.ty
            );
            return self.error(
                Diagnostic::new(DiagnosticCode::error(404), other.span, message)
                    .with_rule(RuleId::LIB_02),
                span,
            );
        }
        let Key::Function(key) = self.key_argument(key, name, &item) else {
            return Self::typed(
                TypedExprKind::Constant(crate::typed::TypedConstant::None),
                Type::Error,
                span,
            );
        };
        let list = Self::typed(
            TypedExprKind::List(values),
            Type::List(Box::new(item.clone())),
            span,
        );
        let operation = if largest {
            KeyedOperation::Max
        } else {
            KeyedOperation::Min
        };
        Self::keyed(operation, list, key, (None, false), item, span)
    }

    /// Returns the position of the keyword `name` among `keywords`, if it is given.
    pub(super) fn keyword_position(keywords: &[Keyword], name: &str) -> Option<usize> {
        keywords.iter().position(|keyword| {
            keyword
                .name
                .as_ref()
                .is_some_and(|keyword| keyword.name == name)
        })
    }
}
