//! Checks statements, and follows definite assignment through branches and loops.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Constant, Expr, Stmt};

use super::BodyChecker;
use super::flow::meet;
use crate::typed::{TypedConstant, TypedExpr, TypedExprKind, TypedStmt};
use crate::types::Type;

impl BodyChecker<'_> {
    pub(super) fn statements(&mut self, statements: &[&Stmt]) -> Vec<TypedStmt> {
        statements
            .iter()
            .filter_map(|statement| self.statement(statement))
            .collect()
    }

    pub(super) fn block(&mut self, statements: &[Stmt]) -> Vec<TypedStmt> {
        let statements: Vec<&Stmt> = statements.iter().collect();
        self.statements(&statements)
    }

    #[allow(clippy::too_many_lines, reason = "one arm per statement kind")]
    pub(super) fn statement(&mut self, statement: &Stmt) -> Option<TypedStmt> {
        Some(match statement {
            Stmt::Assign {
                targets,
                value,
                span,
            } => {
                let value = self.expression(value);
                let targets = targets
                    .iter()
                    .filter_map(|target| self.assign(target, value.ty, &value))
                    .collect();
                TypedStmt::Assign {
                    targets,
                    value,
                    span: *span,
                }
            }
            Stmt::AnnAssign {
                target,
                value: Some(value),
                span,
                ..
            } => {
                let value = self.expression(value);
                let target = self.assign(target, value.ty, &value)?;
                TypedStmt::Assign {
                    targets: vec![target],
                    value,
                    span: *span,
                }
            }
            Stmt::AugAssign {
                target,
                operator,
                value,
                span,
            } => {
                let current = self.expression(target);
                let value = self.expression(value);
                let result = self.binary_type(*operator, &current, &value, *span);
                let symbol = self.assign(target, result, &value)?;
                TypedStmt::AugAssign {
                    target: symbol,
                    operator: *operator,
                    value,
                    span: *span,
                }
            }
            Stmt::Expr { value, span } => TypedStmt::Expr {
                value: self.expression(value),
                span: *span,
            },
            Stmt::Return { value, span } => self.return_statement(value.as_ref(), *span),
            Stmt::If {
                test,
                body,
                orelse,
                span,
            } => {
                let test = self.truth_test(test);
                let before = self.assigned.clone();
                let body = self.block(body);
                let after_body = std::mem::replace(&mut self.assigned, before);
                let orelse = self.block(orelse);
                self.assigned = meet(after_body, self.assigned.take());
                TypedStmt::If {
                    test,
                    body,
                    orelse,
                    span: *span,
                }
            }
            Stmt::While {
                test,
                body,
                orelse,
                span,
            } => {
                let always = matches!(
                    test,
                    Expr::Constant {
                        value: Constant::Bool(true),
                        ..
                    }
                );
                let test = self.truth_test(test);
                let (body, orelse) = self.loop_statement(body, orelse, always);
                TypedStmt::While {
                    test,
                    body,
                    orelse,
                    span: *span,
                }
            }
            Stmt::For {
                target,
                iter,
                body,
                orelse,
                span,
            } => self.for_range(target, iter, body, orelse, *span)?,
            Stmt::Pass { span } => TypedStmt::Pass { span: *span },
            Stmt::Break { span } => {
                let state = self.assigned.take();
                if let Some(breaks) = self.breaks.last_mut() {
                    breaks.push(state);
                }
                TypedStmt::Break { span: *span }
            }
            Stmt::Continue { span } => {
                self.assigned = None;
                TypedStmt::Continue { span: *span }
            }
            Stmt::AnnAssign { value: None, .. }
            | Stmt::FunctionDef { .. }
            | Stmt::Import { .. }
            | Stmt::ImportFrom { .. }
            | Stmt::Other { .. } => return None,
        })
    }

    /// Checks a loop body and its `else` clause; the body may run zero times, and `else` runs only when no `break` does.
    pub(super) fn loop_statement(
        &mut self,
        body: &[Stmt],
        orelse: &[Stmt],
        always: bool,
    ) -> (Vec<TypedStmt>, Vec<TypedStmt>) {
        let entry = self.assigned.clone();
        self.breaks.push(Vec::new());
        let body = self.block(body);
        let breaks = self.breaks.pop().unwrap_or_default();
        self.assigned = if always { None } else { entry };
        let orelse = self.block(orelse);
        let mut exit = self.assigned.take();
        for state in breaks {
            exit = meet(exit, state);
        }
        self.assigned = exit;
        (body, orelse)
    }

    pub(super) fn for_range(
        &mut self,
        target: &Expr,
        iter: &Expr,
        body: &[Stmt],
        orelse: &[Stmt],
        span: Span,
    ) -> Option<TypedStmt> {
        let Expr::Call {
            arguments,
            span: call_span,
            ..
        } = iter
        else {
            return None;
        };
        let values: Vec<TypedExpr> = arguments
            .iter()
            .map(|argument| self.expression(argument))
            .collect();
        for value in &values {
            self.expect(value, Type::Int, "an argument of `range`");
        }
        let constant = |digits: &str| TypedExpr {
            kind: TypedExprKind::Constant(TypedConstant::Int(digits.to_owned())),
            ty: Type::Int,
            span: *call_span,
        };
        let mut given = values.into_iter();
        let (first, limit, stride) = match (given.next(), given.next(), given.next()) {
            (Some(only), None, _) => (constant("0"), only, constant("1")),
            (Some(begin), Some(end), None) => (begin, end, constant("1")),
            (Some(begin), Some(end), Some(increment)) => (begin, end, increment),
            (None, _, _) => return None,
        };
        let entry = self.assigned.clone();
        let variable = first.clone();
        let target = self.assign(target, Type::Int, &variable)?;
        self.assigned = entry;
        let mut inside = self.assigned.clone().unwrap_or_default();
        inside.insert(target);
        let before_loop = self.assigned.replace(inside);
        self.breaks.push(Vec::new());
        let body = self.block(body);
        let breaks = self.breaks.pop().unwrap_or_default();
        self.assigned = before_loop;
        let orelse = self.block(orelse);
        let mut exit = self.assigned.take();
        for state in breaks {
            exit = meet(exit, state);
        }
        self.assigned = exit;
        Some(TypedStmt::ForRange {
            target,
            start: first,
            stop: limit,
            step: stride,
            body,
            orelse,
            span,
        })
    }

    pub(super) fn return_statement(&mut self, value: Option<&Expr>, span: Span) -> TypedStmt {
        let expected = self.returns.unwrap_or(Type::NoneType);
        let value = value.map(|value| self.expression(value));
        let actual = value.as_ref().map_or(Type::NoneType, |value| value.ty);
        if !actual.is_assignable_to(expected) {
            let mut diagnostic = Diagnostic::new(
                DiagnosticCode::error(405),
                value.as_ref().map_or(span, |value| value.span),
                format!(
                    "this returns {}, but the function returns {expected}",
                    actual.with_article()
                ),
            )
            .with_rule(if expected == Type::Float && actual == Type::Int {
                RuleId::TYP_09
            } else {
                RuleId::TYP_01
            });
            if let Some(TypedExpr {
                kind: TypedExprKind::Constant(TypedConstant::Int(digits)),
                span,
                ..
            }) = &value
                && expected == Type::Float
            {
                diagnostic = diagnostic.with_fix(*span, format!("{digits}.0"));
            }
            self.diagnostics.push(diagnostic);
        }
        self.assigned = None;
        TypedStmt::Return { value, span }
    }
}
