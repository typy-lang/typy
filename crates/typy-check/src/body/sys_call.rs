//! Checks the calls of the functions of built-in modules that are not `math`'s: `sys.exit`
//! raises `SystemExit` with its argument, as CPython's does (LIB-04), and
//! `sys.getrecursionlimit` and `sys.setrecursionlimit` read and set the limit (LIB-13).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use super::builtin_module::ModuleFunction;
use crate::typed::{TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks a call of a function of a built-in module.
    pub(super) fn module_function_call(
        &mut self,
        function: ModuleFunction,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        match function {
            ModuleFunction::Math(function) => self.math_call(function, arguments, keywords, span),
            ModuleFunction::Exit => self.sys_exit(arguments, keywords, span),
            ModuleFunction::GetRecursionLimit => {
                self.recursion_limit(false, arguments, keywords, span)
            }
            ModuleFunction::SetRecursionLimit => {
                self.recursion_limit(true, arguments, keywords, span)
            }
        }
    }

    /// Checks `sys.getrecursionlimit()`, or `sys.setrecursionlimit(limit)` when `set`, whose
    /// limit is an `int` or a `bool`.
    fn recursion_limit(
        &mut self,
        set: bool,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let name = if set {
            "sys.setrecursionlimit"
        } else {
            "sys.getrecursionlimit"
        };
        let wanted = usize::from(set);
        if !keywords.is_empty() || arguments.len() != wanted {
            for argument in arguments {
                self.expression(argument);
            }
            let given = arguments.len() + keywords.len();
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!(
                        "`{name}` takes {wanted} positional argument{}, and {given} {} given",
                        if wanted == 1 { "" } else { "s" },
                        if given == 1 { "is" } else { "are" }
                    ),
                )
                .with_rule(RuleId::LIB_13),
                span,
            );
        }
        let Some(argument) = arguments.first() else {
            return Self::typed(TypedExprKind::RecursionLimit(None), Type::Int, span);
        };
        let limit = self.expression(argument);
        match limit.ty {
            Type::Int | Type::Bool => Self::typed(
                TypedExprKind::RecursionLimit(Some(Box::new(limit))),
                Type::NoneType,
                span,
            ),
            Type::Error => Self::invalid_call(span),
            ref other => {
                let message = format!(
                    "'{}' object cannot be interpreted as an integer",
                    other.class_name()
                );
                self.error(
                    Diagnostic::new(DiagnosticCode::error(404), limit.span, message)
                        .with_rule(RuleId::LIB_13),
                    span,
                )
            }
        }
    }

    /// Checks `sys.exit()` or `sys.exit(code)`, where `code` may be any value: `None` and an
    /// `int` give the exit status, and any other value is written to standard error.
    fn sys_exit(&mut self, arguments: &[Expr], keywords: &[Keyword], span: Span) -> TypedExpr {
        if let Some(keyword) = keywords.first() {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    keyword.span,
                    "`sys.exit` takes no keyword arguments",
                )
                .with_rule(RuleId::LIB_04),
                span,
            );
        }
        if arguments.len() > 1 {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    arguments[1].span(),
                    format!(
                        "`sys.exit` takes at most 1 argument, and {} are given",
                        arguments.len()
                    ),
                )
                .with_rule(RuleId::LIB_04),
                span,
            );
        }
        let code = arguments
            .first()
            .map(|argument| Box::new(self.expression(argument)));
        if code.as_ref().is_some_and(|code| code.ty.has_error()) {
            return Self::invalid_call(span);
        }
        Self::typed(TypedExprKind::SysExit(code), Type::NoneType, span)
    }
}
