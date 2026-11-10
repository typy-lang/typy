//! Checks calls of exception classes and `raise` statements (SMT-06, EXC-07, EXC-12,
//! ADR-0013).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Constant, Expr, Keyword};

use super::BodyChecker;
use crate::exception::{BuiltinException, ExceptionClass};
use crate::typed::{RaiseCause, TypedExpr, TypedExprKind, TypedStmt};
use crate::types::{Type, TypeObjectOf};

impl BodyChecker<'_> {
    /// Checks a call of a built-in exception class: positional arguments of any type, which
    /// the exception keeps.
    pub(super) fn new_exception(
        &mut self,
        class: BuiltinException,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let values: Vec<TypedExpr> = arguments
            .iter()
            .map(|value| self.expression(value))
            .collect();
        if !class.can_be_called() {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(417),
                    span,
                    format!(
                        "`{}` cannot be called in TyPy; its five arguments are not checked",
                        class.name()
                    ),
                )
                .with_rule(RuleId::EXC_12)
                .with_help("raise `ValueError` or a class of your own, and catch this one"),
                span,
            );
        }
        if let Some(keyword) = keywords.first() {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    keyword.span,
                    format!("`{}` takes positional arguments only", class.name()),
                )
                .with_rule(RuleId::EXC_01),
                span,
            );
        }
        let class = ExceptionClass::Builtin(class);
        Self::typed(
            TypedExprKind::NewException {
                class: class.clone(),
                arguments: values,
            },
            Type::Exception(class),
            span,
        )
    }

    /// Checks `raise`, `raise exception` and `raise exception from cause`. Nothing after it
    /// runs, so the point after it cannot be reached.
    pub(super) fn raise_statement(
        &mut self,
        exception: Option<&Expr>,
        cause: Option<&Expr>,
        span: Span,
    ) -> TypedStmt {
        let Some(exception) = exception else {
            if self.handling == 0 {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(416),
                        span,
                        "a bare `raise` is accepted only inside an `except` clause",
                    )
                    .with_rule(RuleId::EXC_07)
                    .with_help("name the exception to raise, such as `raise ValueError(...)`"),
                );
            }
            self.assigned = None;
            return TypedStmt::Raise {
                exception: None,
                cause: RaiseCause::Absent,
                span,
            };
        };
        let exception = self.exception_operand(exception, "`raise`");
        let cause = match cause {
            None => RaiseCause::Absent,
            Some(Expr::Constant {
                value: Constant::None,
                ..
            }) => RaiseCause::None,
            Some(cause) => RaiseCause::Exception(self.exception_operand(cause, "`from`")),
        };
        self.assigned = None;
        TypedStmt::Raise {
            exception: Some(exception),
            cause,
            span,
        }
    }

    /// Checks what `raise` or `from` names: an exception, or an exception class, which is
    /// called with no arguments, as CPython does.
    fn exception_operand(&mut self, operand: &Expr, what: &str) -> TypedExpr {
        let value = self.expression(operand);
        match &value.ty {
            Type::Exception(_) | Type::Error => value,
            Type::TypeObject(Some(of)) if matches!(**of, TypeObjectOf::Exception(_)) => {
                let TypeObjectOf::Exception(class) = (**of).clone() else {
                    unreachable!("matched above");
                };
                self.called_class(class, value.span)
            }
            other => {
                let message = format!(
                    "{what} takes an exception or an exception class, and this is {}",
                    other.with_article()
                );
                self.error(
                    Diagnostic::new(DiagnosticCode::error(415), value.span, message)
                        .with_rule(RuleId::SMT_06),
                    value.span,
                )
            }
        }
    }

    /// Returns a new exception of `class`, called with no arguments, in place of the class.
    fn called_class(&mut self, class: ExceptionClass, span: Span) -> TypedExpr {
        let ExceptionClass::Builtin(builtin) = &class;
        if !builtin.can_be_called() {
            return self.new_exception(*builtin, &[], &[], span);
        }
        Self::typed(
            TypedExprKind::NewException {
                class: class.clone(),
                arguments: Vec::new(),
            },
            Type::Exception(class),
            span,
        )
    }
}
