//! Checks what a `for` loop goes over: a sequence, or `enumerate`, `zip` or `reversed` of
//! sequences (CST-03, LIB-02).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use crate::builtin::Builtin;
use crate::typed::{LoopSource, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns the builtin a call names, with its arguments, when no local or program function
    /// of that name hides it, as the call would resolve it.
    pub(super) fn called_builtin<'call>(
        &self,
        expression: &'call Expr,
    ) -> Option<(Builtin, &'call [Expr], &'call [Keyword])> {
        let Expr::Call {
            function,
            arguments,
            keywords,
            ..
        } = expression
        else {
            return None;
        };
        let Expr::Name { id, .. } = function.as_ref() else {
            return None;
        };
        if self.locals.contains_key(id) || self.program.function_ids.contains_key(id) {
            return None;
        }
        Some((Builtin::named(id)?, arguments, keywords))
    }

    /// Checks the source of a `for` loop, or an argument of `enumerate` or `zip`, and returns
    /// it with the type of what each step gives.
    pub(super) fn loop_source(&mut self, iter: &Expr, user: &str) -> (LoopSource, Type) {
        let call = self.called_builtin(iter).filter(|(builtin, ..)| {
            matches!(
                builtin,
                Builtin::Enumerate | Builtin::Zip | Builtin::Reversed
            )
        });
        let Some((builtin, arguments, keywords)) = call else {
            let sequence = self.expression(iter);
            let item = self.iterated_item(&sequence, user);
            return (LoopSource::Items(sequence), item);
        };
        let span = iter.span();
        let source = match builtin {
            Builtin::Enumerate => self.enumerate(arguments, keywords, span),
            Builtin::Zip => self.zip(arguments, keywords),
            _ => self.reversed(arguments, keywords, span),
        };
        let ty = source.item_type();
        (source, ty)
    }

    fn enumerate(&mut self, arguments: &[Expr], keywords: &[Keyword], span: Span) -> LoopSource {
        let keyword = self.only_keyword(Builtin::Enumerate, "start", keywords);
        let given = arguments.len() + usize::from(keyword.is_some());
        if arguments.is_empty() || given > 2 {
            return self.source_arity(Builtin::Enumerate, "1 or 2", given, arguments, span);
        }
        let (source, _) = self.loop_source(&arguments[0], "`enumerate`");
        let start = match arguments.get(1).or(keyword) {
            Some(start) => {
                let start = self.expression(start);
                self.expect(&start, Type::Int, "the start of `enumerate`");
                start
            }
            None => TypedExpr {
                kind: TypedExprKind::Constant(TypedConstant::Int("0".to_owned())),
                ty: Type::Int,
                span,
            },
        };
        LoopSource::Enumerate {
            source: Box::new(source),
            start,
        }
    }

    fn zip(&mut self, arguments: &[Expr], keywords: &[Keyword]) -> LoopSource {
        let strict = self.only_keyword(Builtin::Zip, "strict", keywords);
        let sources = arguments
            .iter()
            .map(|argument| self.loop_source(argument, "`zip`").0)
            .collect();
        let strict = strict.map(|strict| {
            let strict = self.expression(strict);
            self.expect(&strict, Type::Bool, "`strict`");
            strict
        });
        LoopSource::Zip { sources, strict }
    }

    fn reversed(&mut self, arguments: &[Expr], keywords: &[Keyword], span: Span) -> LoopSource {
        self.only_keyword(Builtin::Reversed, "", keywords);
        if arguments.len() != 1 {
            return self.source_arity(Builtin::Reversed, "1", arguments.len(), arguments, span);
        }
        let sequence = self.expression(&arguments[0]);
        self.iterated_item(&sequence, "`reversed`");
        LoopSource::Reversed(sequence)
    }

    /// Returns the value of the one keyword argument `accepted`, after reporting any other.
    fn only_keyword<'call>(
        &mut self,
        builtin: Builtin,
        accepted: &str,
        keywords: &'call [Keyword],
    ) -> Option<&'call Expr> {
        let mut value = None;
        for keyword in keywords {
            let name = keyword.name.as_ref().map(|name| name.name.as_str());
            if name == Some(accepted) && value.is_none() {
                value = Some(&keyword.value);
                continue;
            }
            let message = match name {
                Some(name) if name == accepted => format!("`{name}` is given twice"),
                Some(name) => format!("`{}` has no parameter named `{name}`", builtin.name()),
                None => format!("`{}` takes no `**` arguments", builtin.name()),
            };
            self.diagnostics.push(
                Diagnostic::new(DiagnosticCode::error(404), keyword.span, message)
                    .with_rule(RuleId::LIB_02),
            );
        }
        value
    }

    /// Reports a call with the wrong number of arguments, and checks them for their own errors.
    fn source_arity(
        &mut self,
        builtin: Builtin,
        expected: &str,
        given: usize,
        arguments: &[Expr],
        span: Span,
    ) -> LoopSource {
        for argument in arguments {
            self.expression(argument);
        }
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(404),
                span,
                format!(
                    "`{}` takes {expected} arguments, and {given} {} given",
                    builtin.name(),
                    if given == 1 { "is" } else { "are" }
                ),
            )
            .with_rule(RuleId::LIB_02),
        );
        let error = TypedExpr {
            kind: TypedExprKind::Constant(TypedConstant::None),
            ty: Type::Error,
            span,
        };
        LoopSource::Items(error)
    }
}
