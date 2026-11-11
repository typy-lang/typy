//! Checks `iter()` and `next()`: iterators of the built-in iterables, and the `__iter__` and
//! `__next__` of a class (CST-08, CST-09, ADR-0014).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use crate::builtin::Builtin;
use crate::typed::{Argument, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks `iter(source)`: an iterator over a built-in iterable, the iterator itself, or
    /// what the class's `__iter__` returns.
    pub(super) fn call_iter(
        &mut self,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let [source] = arguments else {
            return self.wrong_arity(Builtin::Iter, "1", arguments, span);
        };
        self.no_keywords(Builtin::Iter, keywords);
        let source = self.sequence(source);
        match &source.ty {
            Type::Iterator(_) | Type::Error => return source,
            ty if ty.class_id().is_some() => return self.call_special(source, "__iter__", span),
            _ => {}
        }
        let item = self.iterated_item(&source, "`iter`");
        if item.has_error() {
            return Self::invalid_call(span);
        }
        Self::typed(
            TypedExprKind::Iter(Box::new(source)),
            Type::Iterator(Box::new(item)),
            span,
        )
    }

    /// Checks `next(iterator)` and `next(iterator, default)`. With a default, the result has
    /// the item type when the default has it, or is optional when the default is `None`.
    pub(super) fn call_next(
        &mut self,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let (iterator, default) = match arguments {
            [iterator] => (iterator, None),
            [iterator, default] => (iterator, Some(default)),
            _ => return self.wrong_arity(Builtin::Next, "1 or 2", arguments, span),
        };
        self.no_keywords(Builtin::Next, keywords);
        let iterator = self.expression(iterator);
        let (method, item) = match &iterator.ty {
            Type::Iterator(item) => (None, item.as_ref().clone()),
            Type::Error => (None, Type::Error),
            ty if let Some(class) = ty.class_id()
                && let Some(next) = self.program.method(class, "__next__") =>
            {
                (
                    Some(next),
                    self.program.signatures[next.0 as usize].returns.clone(),
                )
            }
            other => {
                let message = format!(
                    "`next` takes an iterator, and this is {}",
                    other.with_article()
                );
                self.diagnostics.push(
                    Diagnostic::new(DiagnosticCode::error(404), iterator.span, message)
                        .with_rule(RuleId::CST_09)
                        .with_help("make an iterator with `iter()` first"),
                );
                return Self::invalid_call(span);
            }
        };
        let (default, ty) = match default {
            None => (None, item),
            Some(default) => {
                let value = self.expression_expecting(default, Some(&item));
                let ty = if value.ty == Type::NoneType && !matches!(item, Type::Optional(_)) {
                    Type::optional(item)
                } else {
                    self.expect(&value, item.clone(), "the default of `next`");
                    item
                };
                let value = Self::converted(value, Some(&ty));
                (Some(Box::new(value)), ty)
            }
        };
        Self::typed(
            TypedExprKind::Next {
                iterator: Box::new(iterator),
                method,
                default,
            },
            ty,
            span,
        )
    }

    /// Calls the special method `name`, which takes only the instance, of the class of
    /// `instance`.
    fn call_special(&mut self, instance: TypedExpr, name: &str, span: Span) -> TypedExpr {
        let class = instance.ty.class_id().unwrap_or(crate::typed::ClassId(0));
        let Some(method) = self.program.method(class, name) else {
            let message = format!(
                "{} has no `{name}` in its class",
                instance.ty.with_article()
            );
            return self.error(
                Diagnostic::new(DiagnosticCode::error(411), instance.span, message)
                    .with_rule(RuleId::CST_08),
                span,
            );
        };
        let returns = self.program.signatures[method.0 as usize].returns.clone();
        Self::typed(
            TypedExprKind::Call {
                function: method,
                arguments: vec![Argument {
                    parameter: 0,
                    value: instance,
                }],
            },
            returns,
            span,
        )
    }

    /// Reports keyword arguments to a builtin that takes none.
    fn no_keywords(&mut self, builtin: Builtin, keywords: &[Keyword]) {
        if let Some(keyword) = keywords.first() {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    keyword.span,
                    format!("`{}` takes no keyword arguments", builtin.name()),
                )
                .with_rule(RuleId::CST_09),
            );
        }
    }

    /// Reports a call of a builtin with the wrong number of arguments.
    fn wrong_arity(
        &mut self,
        builtin: Builtin,
        expected: &str,
        arguments: &[Expr],
        span: Span,
    ) -> TypedExpr {
        for argument in arguments {
            self.expression(argument);
        }
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(404),
                span,
                format!(
                    "`{}` takes {expected} arguments, and {} are given",
                    builtin.name(),
                    arguments.len()
                ),
            )
            .with_rule(RuleId::CST_09),
            span,
        )
    }
}
