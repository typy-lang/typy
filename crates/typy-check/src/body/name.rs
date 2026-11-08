//! Resolves names read as values: locals, `__name__`, functions, builtins and names TyPy does not provide.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};

use super::BodyChecker;
use crate::builtin::Builtin;
use crate::typed::{TypedExpr, TypedExprKind};
use crate::types::Type;
use crate::unprovided::{Unprovided, unprovided};

/// Module-level names that CPython defines and TyPy 0.1 does not provide.
const MODULE_DUNDERS: &[&str] = &[
    "__annotations__",
    "__builtins__",
    "__doc__",
    "__file__",
    "__loader__",
    "__package__",
    "__spec__",
];

impl BodyChecker<'_> {
    /// Resolves a name that is read as a value (LR 4.2.2).
    pub(super) fn name(&mut self, id: &str, span: Span) -> TypedExpr {
        if let Some(symbol) = self.locals.get(id).copied() {
            if let Some(assigned) = &self.assigned
                && !assigned.contains(&symbol)
                && self.reported_unassigned.insert(symbol)
            {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(306),
                        span,
                        format!("`{id}` may be used before it is assigned"),
                    )
                    .with_rule(RuleId::TYP_12)
                    .with_help(format!(
                        "assign `{id}` on every path that reaches this line"
                    )),
                );
            }
            let ty = self.types.get(&symbol).cloned().unwrap_or(Type::Error);
            return Self::typed(TypedExprKind::Local(symbol), ty, span);
        }
        if id == "__name__" {
            return Self::typed(TypedExprKind::ModuleName, Type::Str, span);
        }
        if self.in_function() && self.program.module_variables.contains(id) {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    span,
                    format!("reading the module variable `{id}` from a function is planned for TyPy 0.3 (SMT-08)"),
                )
                .with_rule(RuleId::SMT_08),
                span,
            );
        }
        if self.program.function_ids.contains_key(id) || Builtin::named(id).is_some() {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    span,
                    format!("using `{id}` as a value is planned for TyPy 1.0 (FUN-05)"),
                )
                .with_rule(RuleId::FUN_05),
                span,
            );
        }
        self.unknown_name(id, span)
    }

    pub(super) fn unknown_name(&mut self, id: &str, span: Span) -> TypedExpr {
        let diagnostic = match unprovided(id) {
            Some(Unprovided::Planned { release, rule }) => Diagnostic::new(
                DiagnosticCode::error(201),
                span,
                format!("`{id}` is planned for TyPy {release} ({rule})"),
            )
            .with_rule(rule),
            Some(Unprovided::Excluded) => Diagnostic::new(
                DiagnosticCode::error(202),
                span,
                format!("`{id}` is a Python builtin that TyPy does not provide"),
            )
            .with_rule(RuleId::STD_04),
            None if MODULE_DUNDERS.contains(&id) => Diagnostic::new(
                DiagnosticCode::error(202),
                span,
                format!("`{id}` is a module attribute that TyPy does not provide"),
            )
            .with_rule(RuleId::STD_04),
            None => Diagnostic::new(
                DiagnosticCode::error(205),
                span,
                format!("`{id}` is not defined"),
            )
            .with_rule(RuleId::STD_04),
        };
        self.error(diagnostic, span)
    }
}
