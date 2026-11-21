//! Checks the builtins that take one iterable: `sorted`, `sum`, `any`, `all`, and `min` and
//! `max` of one argument (LIB-02), with the `key` of `sorted`, `min` and `max` (FUN-05).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use super::key_argument::Key;
use crate::builtin::Builtin;
use crate::typed::{KeyedOperation, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

/// The value given for an optional parameter.
pub(super) enum Given<'call> {
    /// The call does not give it.
    Absent,
    /// The call gives this value.
    Value(&'call Expr),
    /// The call has a keyword the callee refuses, already reported.
    Refused,
}

impl<'call> Given<'call> {
    fn of(value: Option<&'call Expr>) -> Self {
        value.map_or(Self::Absent, Self::Value)
    }

    /// Returns the value, if one is given.
    pub(super) fn value(&self) -> Option<&'call Expr> {
        match self {
            Self::Value(value) => Some(value),
            Self::Absent | Self::Refused => None,
        }
    }
}

/// Returns whether a call of `builtin` with `positional` arguments takes one iterable.
pub(super) fn takes_one_iterable(builtin: Builtin, positional: usize) -> bool {
    match builtin {
        Builtin::Sorted | Builtin::Sum | Builtin::Any | Builtin::All => true,
        Builtin::Min | Builtin::Max => positional == 1,
        _ => false,
    }
}

/// Returns the name of the one optional value `builtin` takes, if any.
fn option_name(builtin: Builtin) -> Option<&'static str> {
    match builtin {
        Builtin::Sorted => Some("reverse"),
        Builtin::Sum => Some("start"),
        Builtin::Min | Builtin::Max => Some("default"),
        _ => None,
    }
}

impl BodyChecker<'_> {
    /// Checks a call of a builtin that takes one iterable, and the optional value it allows:
    /// `reverse` for `sorted`, `start` for `sum`, also by position, and `default` for `min` and
    /// `max`, and the `key` of `sorted`, `min` and `max` (FUN-05).
    pub(super) fn call_over_iterable(
        &mut self,
        builtin: Builtin,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let positional = if builtin == Builtin::Sum {
            1..=2
        } else {
            1..=1
        };
        if !positional.contains(&arguments.len()) {
            let most = positional.end();
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!(
                        // One argument is always accepted, so a refused call has none or
                        // several.
                        "`{}` takes 1{} positional argument{}, and {} are given",
                        builtin.name(),
                        if *most == 1 {
                            String::new()
                        } else {
                            format!(" to {most}")
                        },
                        if *most == 1 { "" } else { "s" },
                        arguments.len(),
                    ),
                )
                .with_rule(RuleId::LIB_02),
                span,
            );
        }
        // The key is checked on its own; every other keyword is the option's or refused.
        let key_position = matches!(builtin, Builtin::Sorted | Builtin::Min | Builtin::Max)
            .then(|| Self::keyword_position(keywords, "key"))
            .flatten();
        let others: Vec<Keyword> = keywords
            .iter()
            .enumerate()
            .filter(|(position, _)| Some(*position) != key_position)
            .map(|(_, keyword)| keyword.clone())
            .collect();
        let option_first = option_name(builtin)
            .and_then(|name| Self::keyword_position(keywords, name))
            .zip(key_position)
            .is_some_and(|(option, key)| option < key);
        let iterable = self.iterable_argument(&arguments[0], builtin);
        let item = iterable.ty.sequence_item().cloned().unwrap_or(Type::Error);
        let option = self.option_argument(builtin, arguments.get(1), &others, span);
        if let Given::Refused = option {
            return Self::invalid(span);
        }
        let option = option.value();
        let option_type = match builtin {
            Builtin::Sorted => Type::Bool,
            // The bools `sum` adds count as ints, so it starts from an `int` (DAT-19).
            Builtin::Sum if item == Type::Bool => Type::Int,
            _ => item.clone(),
        };
        let option = option.map(|value| {
            let value = self.expression_expecting(value, Some(&option_type));
            self.expect(
                &value,
                option_type.clone(),
                &format!("`{}`", option_name(builtin).unwrap_or_default()),
            );
            Box::new(value)
        });
        if item.has_error() {
            return Self::invalid(span);
        }
        let key = key_position
            .map(|position| self.key_argument(&keywords[position].value, builtin.name(), &item));
        match key {
            Some(Key::Refused) => return Self::invalid(span),
            Some(Key::Function(key)) => {
                let (operation, result) = match builtin {
                    Builtin::Sorted => (KeyedOperation::Sorted, Type::List(Box::new(item))),
                    Builtin::Min => (KeyedOperation::Min, item),
                    _ => (KeyedOperation::Max, item),
                };
                let option = (option.map(|option| *option), option_first);
                return Self::keyed(operation, iterable, key, option, result, span);
            }
            Some(Key::Absent) | None => {}
        }
        let Some(result) = self.iterable_result(builtin, &iterable, &item, option.is_some(), span)
        else {
            return Self::invalid(span);
        };
        Self::typed(
            TypedExprKind::IterableBuiltin {
                builtin,
                iterable: Box::new(iterable),
                option,
            },
            result,
            span,
        )
    }

    /// Returns the optional value the call gives, after reporting any keyword it refuses.
    fn option_argument<'call>(
        &mut self,
        builtin: Builtin,
        positional: Option<&'call Expr>,
        keywords: &'call [Keyword],
        span: Span,
    ) -> Given<'call> {
        let mut option = positional;
        for keyword in keywords {
            let (name, name_span) = match &keyword.name {
                Some(name) => (name.name.as_str(), name.span),
                None => ("**", keyword.span),
            };
            let accepted = option_name(builtin) == Some(name) && option.is_none();
            if accepted {
                option = Some(&keyword.value);
                continue;
            }
            let diagnostic = if option_name(builtin) == Some(name) {
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    name_span,
                    format!("`{name}` is given twice"),
                )
                .with_rule(RuleId::LIB_02)
            } else {
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    name_span,
                    format!("`{}` has no parameter named `{name}`", builtin.name()),
                )
                .with_rule(RuleId::LIB_02)
            };
            self.error(diagnostic, span);
            return Given::Refused;
        }
        Given::of(option)
    }

    /// Returns the type of the call, or `None` after reporting why the items do not suit it.
    fn iterable_result(
        &mut self,
        builtin: Builtin,
        iterable: &TypedExpr,
        item: &Type,
        has_option: bool,
        span: Span,
    ) -> Option<Type> {
        match builtin {
            Builtin::Any | Builtin::All => {
                // The runtime tests each item with its type object's slot, which calls
                // `__bool__`; a length cannot raise from there (CLS-07).
                Some(Type::Bool)
            }
            Builtin::Sorted | Builtin::Min | Builtin::Max => {
                if !item.can_order(item) {
                    self.comparison_error(item, item, span);
                    return None;
                }
                Some(if builtin == Builtin::Sorted {
                    Type::List(Box::new(item.clone()))
                } else {
                    item.clone()
                })
            }
            _ => self.sum_result(iterable, item, has_option, span),
        }
    }

    /// `sum` adds `int`, `bool` or `float` items. A `float` list needs its start, because
    /// CPython returns the `int` 0 for an empty list without one.
    fn sum_result(
        &mut self,
        iterable: &TypedExpr,
        item: &Type,
        has_start: bool,
        span: Span,
    ) -> Option<Type> {
        match item {
            // `sum` of bools counts the true ones, as CPython adds them as ints (DAT-19).
            Type::Int | Type::Bool => Some(Type::Int),
            Type::Float if has_start => Some(Type::Float),
            Type::Float => {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        span,
                        "`sum` of a list[float] needs a float start, because CPython returns the int 0 for an empty list",
                    )
                    .with_rule(RuleId::LIB_02)
                    .with_help("give the start, such as `sum(values, 0.0)`"),
                );
                None
            }
            other => {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(401),
                        iterable.span,
                        format!(
                            "`sum` adds numbers, and the items of {} are {other}",
                            iterable.ty.with_article()
                        ),
                    )
                    .with_rule(RuleId::LIB_02),
                );
                None
            }
        }
    }

    /// Returns the type of the items `user` gets from `iterable`, a list or a `str`; reports an
    /// iterable it cannot go over yet and returns the error type.
    pub(super) fn iterated_item(&mut self, iterable: &TypedExpr, user: &str) -> Type {
        match &iterable.ty {
            Type::List(item) | Type::TupleOf(item) | Type::Set(item) | Type::Dict(item, _) => {
                item.as_ref().clone()
            }
            Type::DictView(view, key, value) => view.item(key, value),
            Type::Str => Type::Str,
            file if file.is_file() => file.file_line().unwrap_or(Type::Error),
            Type::Bytes | Type::Range => Type::Int,
            Type::Iterator(item) | Type::Iterable(item) => item.as_ref().clone(),
            Type::Generator { yields, .. } => yields.as_ref().clone(),
            // A `ctypes` array gives the value of each item (FFI-03).
            Type::CData(crate::ctypes::CData::Array(item, _)) => item.result_type(),
            Type::Error => Type::Error,
            // A tuple whose items have one type is read as a `tuple[T, ...]` before it gets
            // here; the items of this one differ, or there are none.
            Type::Tuple(_) => {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(411),
                        iterable.span,
                        format!(
                            "{user} over {} would give values of no one type; its items need one type",
                            iterable.ty.with_article()
                        ),
                    )
                    .with_rule(RuleId::CST_03),
                );
                Type::Error
            }
            other => {
                let mut diagnostic = Diagnostic::new(
                    DiagnosticCode::error(411),
                    iterable.span,
                    format!("{} cannot be iterated over", other.with_article()),
                )
                .with_rule(RuleId::CST_03);
                if *other == Type::Object {
                    diagnostic = diagnostic.with_help(super::operator::OBJECT_HELP);
                }
                self.diagnostics.push(diagnostic);
                Type::Error
            }
        }
    }

    fn invalid(span: Span) -> TypedExpr {
        Self::typed(
            TypedExprKind::Constant(TypedConstant::None),
            Type::Error,
            span,
        )
    }
}
