//! Checks statements, and follows definite assignment through branches and loops.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Constant, Expr, Stmt};

use super::BodyChecker;
use std::collections::BTreeSet;

use super::flow::{Fact, meet, test_facts};
use crate::builtin::Builtin;
use crate::typed::{AssignTarget, TypedConstant, TypedExpr, TypedExprKind, TypedStmt};
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
                let expected = match targets.as_slice() {
                    [single] => self.known_type(single),
                    _ => None,
                };
                let value = self.expression_expecting(value, expected.as_ref());
                let targets = targets
                    .iter()
                    .filter_map(|target| self.assign_target(target, value.ty.clone(), &value))
                    .collect();
                TypedStmt::Assign {
                    targets,
                    value,
                    span: *span,
                }
            }
            Stmt::AnnAssign {
                target:
                    Expr::Attribute {
                        value: object,
                        attribute,
                        ..
                    },
                annotation,
                value: Some(value),
                span,
                ..
            } => {
                let declared = self.annotation(annotation);
                let value = self.expression_expecting(value, Some(&declared));
                let target =
                    self.attribute_target(object, attribute, &value.ty, Some(declared), *span)?;
                TypedStmt::Assign {
                    targets: vec![target],
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
                let expected = self.known_type(target);
                let value = self.expression_expecting(value, expected.as_ref());
                let target = self.assign(target, value.ty.clone(), &value)?;
                TypedStmt::Assign {
                    targets: vec![AssignTarget::Name(target)],
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
                let (current, value) = Self::tuple_operands(*operator, current, value);
                let result = self.binary_type(*operator, &current, &value, *span);
                let target = if let Expr::Subscript { .. } | Expr::Attribute { .. } = target {
                    self.augmented_item(current, &result, *span)?
                } else {
                    AssignTarget::Name(self.assign(target, result, &value)?)
                };
                TypedStmt::AugAssign {
                    target,
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
                let (holds, fails) = test_facts(&test);
                let before = self.assigned.clone();
                self.add_facts(holds);
                let body = self.block(body);
                let after_body = std::mem::replace(&mut self.assigned, before);
                self.add_facts(fails);
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
                // A variable the body assigns may be `None` again when the test is next read.
                self.forget_not_none(body);
                let test = self.truth_test(test);
                let facts = test_facts(&test);
                let (body, orelse) = self.loop_statement(body, orelse, always, facts);
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
            } => self.for_statement(target, iter, body, orelse, *span)?,
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
            | Stmt::ClassDef { .. }
            | Stmt::Import { .. }
            | Stmt::ImportFrom { .. }
            | Stmt::Other { .. } => return None,
        })
    }

    /// Adds what a test shows to what holds here, when this point can be reached.
    pub(super) fn add_facts(&mut self, facts: BTreeSet<Fact>) {
        if let Some(known) = &mut self.assigned {
            known.extend(facts);
        }
    }

    /// Forgets that a variable is not `None` for each variable `body` assigns, since a loop
    /// reads its body again with what the body left (TYP-13).
    pub(super) fn forget_not_none(&mut self, body: &[Stmt]) {
        let mut names = BTreeSet::new();
        assigned_names(body, &mut names);
        if let Some(known) = &mut self.assigned {
            for name in names {
                if let Some(symbol) = self.locals.get(name) {
                    known.remove(&Fact::NotNone(*symbol));
                }
            }
        }
    }

    /// Checks a loop body and its `else` clause; the body may run zero times, and `else` runs only when no `break` does.
    pub(super) fn loop_statement(
        &mut self,
        body: &[Stmt],
        orelse: &[Stmt],
        always: bool,
        (holds, fails): (BTreeSet<Fact>, BTreeSet<Fact>),
    ) -> (Vec<TypedStmt>, Vec<TypedStmt>) {
        let entry = self.assigned.clone();
        self.add_facts(holds);
        self.breaks.push(Vec::new());
        let body = self.block(body);
        let breaks = self.breaks.pop().unwrap_or_default();
        self.assigned = if always { None } else { entry };
        self.add_facts(fails);
        let orelse = self.block(orelse);
        let mut exit = self.assigned.take();
        for state in breaks {
            exit = meet(exit, state);
        }
        self.assigned = exit;
        (body, orelse)
    }

    /// Checks a `for` statement: over a call of `range`, or over the items of a list (CST-02,
    /// CST-03).
    pub(super) fn for_statement(
        &mut self,
        target: &Expr,
        iter: &Expr,
        body: &[Stmt],
        orelse: &[Stmt],
        span: Span,
    ) -> Option<TypedStmt> {
        if self.is_range_call(iter) {
            return self.for_range(target, iter, body, orelse, span);
        }
        let (source, item) = self.loop_source(iter, "`for`");
        // What each step binds has no expression of its own; diagnostics about it point at the
        // loop's source.
        let binding = TypedExpr {
            kind: TypedExprKind::Constant(TypedConstant::None),
            ty: item.clone(),
            span: iter.span(),
        };
        // Binding the target marks its names as assigned: the body runs with them, while the
        // `else` clause and the code after the loop start from the state before it, because the
        // list may be empty.
        self.forget_not_none(body);
        let before_loop = self.assigned.clone();
        let target = self.assign_target(target, item, &binding)?;
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
        Some(TypedStmt::ForEach {
            target,
            source,
            body,
            orelse,
            span,
        })
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
        self.forget_not_none(body);
        let entry = self.assigned.clone();
        let variable = first.clone();
        let target = self.assign(target, Type::Int, &variable)?;
        self.assigned = entry;
        let mut inside = self.assigned.clone().unwrap_or_default();
        inside.insert(Fact::Assigned(target));
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

    /// Returns true for a call of the builtin `range` with one to three positional arguments,
    /// the form `for` lowers as a counting loop. A program function or variable named `range`
    /// hides the builtin, as it does in a call.
    fn is_range_call(&self, iter: &Expr) -> bool {
        matches!(
            self.called_builtin(iter),
            Some((Builtin::Range, arguments, keywords))
                if keywords.is_empty() && (1..=3).contains(&arguments.len())
        )
    }

    pub(super) fn return_statement(&mut self, value: Option<&Expr>, span: Span) -> TypedStmt {
        self.check_instance_complete(span);
        let expected = self.returns.clone().unwrap_or(Type::NoneType);
        let value = value.map(|value| self.expression_expecting(value, Some(&expected)));
        let actual = value
            .as_ref()
            .map_or(Type::NoneType, |value| value.ty.clone());
        if !actual.is_assignable_to(&expected) {
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

/// Collects the names the statements assign, in nested blocks as well.
fn assigned_names<'tree>(statements: &'tree [Stmt], names: &mut BTreeSet<&'tree str>) {
    for statement in statements {
        match statement {
            Stmt::Assign { targets, .. } => {
                for target in targets {
                    target_names(target, names);
                }
            }
            Stmt::AugAssign { target, .. } | Stmt::AnnAssign { target, .. } => {
                target_names(target, names);
            }
            Stmt::For {
                target,
                body,
                orelse,
                ..
            } => {
                target_names(target, names);
                assigned_names(body, names);
                assigned_names(orelse, names);
            }
            Stmt::While { body, orelse, .. } | Stmt::If { body, orelse, .. } => {
                assigned_names(body, names);
                assigned_names(orelse, names);
            }
            _ => {}
        }
    }
}

/// Collects the names an assignment target binds.
fn target_names<'tree>(target: &'tree Expr, names: &mut BTreeSet<&'tree str>) {
    match target {
        Expr::Name { id, .. } => {
            names.insert(id.as_str());
        }
        Expr::Tuple { elements, .. } | Expr::List { elements, .. } => {
            for element in elements {
                target_names(element, names);
            }
        }
        _ => {}
    }
}
