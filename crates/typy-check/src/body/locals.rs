//! Finds the local variables of a body and binds values to them (LR 4.2.2, TYP-02, TYP-03).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId};
use typy_source::syntax::{Expr, Stmt};

use super::BodyChecker;
use crate::check::annotation_type;
use crate::typed::{SymbolId, SymbolKind, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Every name assigned anywhere in the body is a local of the body (LR 4.2.2).
    pub(super) fn collect_locals(&mut self, statements: &[&Stmt]) {
        for statement in statements {
            match statement {
                Stmt::Assign { targets, .. } => {
                    for target in targets {
                        self.bind_local(target);
                    }
                }
                Stmt::AugAssign { target, .. } => self.bind_local(target),
                Stmt::AnnAssign {
                    target, annotation, ..
                } => {
                    self.bind_local(target);
                    self.declare(target, annotation);
                }
                Stmt::For {
                    target,
                    body,
                    orelse,
                    ..
                } => {
                    self.bind_local(target);
                    self.collect_locals(&body.iter().chain(orelse).collect::<Vec<_>>());
                }
                Stmt::While { body, orelse, .. } | Stmt::If { body, orelse, .. } => {
                    self.collect_locals(&body.iter().chain(orelse).collect::<Vec<_>>());
                }
                _ => {}
            }
        }
    }

    pub(super) fn bind_local(&mut self, target: &Expr) {
        let Expr::Name { id, span } = target else {
            return;
        };
        if !self.locals.contains_key(id) {
            let symbol = self
                .program
                .add_symbol(id, SymbolKind::Local, Type::Error, *span);
            self.locals.insert(id.clone(), symbol);
            self.local_order.push(symbol);
        }
    }

    pub(super) fn declare(&mut self, target: &Expr, annotation: &Expr) {
        let Expr::Name { id, span } = target else {
            return;
        };
        let ty = annotation_type(annotation, self.diagnostics);
        let symbol = self.locals[id];
        match self.declared.get(&symbol) {
            Some(existing) if *existing != ty => self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(402),
                    *span,
                    format!("`{id}` was declared as {existing} and is declared again as {ty}"),
                )
                .with_rule(RuleId::TYP_03),
            ),
            Some(_) => {}
            None => {
                self.declared.insert(symbol, ty);
                self.types.insert(symbol, ty);
            }
        }
    }

    pub(super) fn finish_locals(&mut self) -> Vec<SymbolId> {
        for symbol in &self.local_order {
            let ty = self.types.get(symbol).copied().unwrap_or(Type::Error);
            self.program.symbols[symbol.0 as usize].ty = ty;
        }
        std::mem::take(&mut self.local_order)
    }

    /// Binds a name target to a value of type `ty`; the first binding fixes the variable's type (TYP-02, TYP-03).
    pub(super) fn assign(
        &mut self,
        target: &Expr,
        ty: Type,
        value: &TypedExpr,
    ) -> Option<SymbolId> {
        let Expr::Name { id, span } = target else {
            return None;
        };
        let symbol = *self.locals.get(id)?;
        match self.types.get(&symbol).copied() {
            None => {
                self.types.insert(symbol, ty);
            }
            Some(existing) if !ty.is_assignable_to(existing) => {
                let mut diagnostic = Diagnostic::new(
                    DiagnosticCode::error(402),
                    value.span,
                    format!(
                        "`{id}` was declared as {existing} and is assigned {}",
                        ty.with_article()
                    ),
                )
                .with_label(*span, "the variable")
                .with_rule(if existing == Type::Float && ty == Type::Int {
                    RuleId::TYP_09
                } else {
                    RuleId::TYP_03
                });
                if let (TypedExprKind::Constant(TypedConstant::Int(digits)), Type::Float) =
                    (&value.kind, existing)
                {
                    diagnostic = diagnostic.with_fix(value.span, format!("{digits}.0"));
                } else {
                    diagnostic = diagnostic.with_help(format!(
                        "assign {}, or use a new variable",
                        existing.with_article()
                    ));
                }
                self.diagnostics.push(diagnostic);
            }
            Some(_) => {}
        }
        if let Some(assigned) = &mut self.assigned {
            assigned.insert(symbol);
        }
        Some(symbol)
    }
}
