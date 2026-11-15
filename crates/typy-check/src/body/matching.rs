//! Checks `match` statements (CST-06). Each pattern becomes tests and assignments written in
//! terms of the subject, checked as the expressions they stand for, so `case 1:` is checked
//! as `subject == 1` and `case None:` narrows as `subject is None` does.

use typy_diagnostics::Span;
use typy_source::syntax::{CompareOperator, Expr, MatchCase, Pattern};

use super::BodyChecker;
use super::flow::{meet, test_facts};
use crate::typed::{SymbolKind, TypedCase, TypedExprKind, TypedPattern, TypedStmt};

impl BodyChecker<'_> {
    /// Checks a `match` statement. A local subject is matched as it is, so what a case shows
    /// of it narrows it; any other subject is stored first in a variable of its own, whose
    /// name no program can write.
    pub(super) fn match_statement(
        &mut self,
        subject: &Expr,
        cases: &[MatchCase],
        span: Span,
    ) -> TypedStmt {
        let checked = self.expression(subject);
        let (name, stored) =
            if let (TypedExprKind::Local(_), Expr::Name { id, .. }) = (&checked.kind, subject) {
                (id.clone(), None)
            } else {
                let name = format!("<match {}>", self.local_order.len());
                let symbol =
                    self.program
                        .add_symbol(&name, SymbolKind::Local, checked.ty.clone(), span);
                self.locals.insert(name.clone(), symbol);
                self.local_order.push(symbol);
                let target = Expr::Name {
                    id: name.clone(),
                    span: subject.span(),
                };
                self.assign(&target, checked.ty.clone(), &checked);
                (name, Some((symbol, checked)))
            };
        let subject = Expr::Name {
            id: name,
            span: subject.span(),
        };
        let mut ends = None;
        let mut typed = Vec::with_capacity(cases.len());
        let mut matched_all = false;
        for case in cases {
            let entry = self.assigned.clone();
            let pattern = self.pattern(&case.pattern, &subject);
            let guard = case.guard.as_ref().map(|guard| {
                let guard = self.truth_test(guard);
                self.add_facts(test_facts(&guard).0);
                guard
            });
            let body = self.block(&case.body);
            ends = meet(ends, self.assigned.take());
            self.assigned = entry;
            // A later case knows this one failed when it is one test without a guard.
            if guard.is_none()
                && let TypedPattern::Test(test) = &pattern
            {
                self.add_facts(test_facts(test).1);
            }
            matched_all |= guard.is_none() && pattern.always_matches();
            typed.push(TypedCase {
                pattern,
                guard,
                body,
            });
        }
        // Without a case that matches everything, the subject may match none.
        let fallthrough = if matched_all {
            None
        } else {
            self.assigned.take()
        };
        self.assigned = meet(ends, fallthrough);
        TypedStmt::Match {
            subject: stored,
            cases: typed,
            span,
        }
    }

    /// Returns what a pattern does to `subject`, adding what each test shows to what is known
    /// while the rest of the case is checked.
    fn pattern(&mut self, pattern: &Pattern, subject: &Expr) -> TypedPattern {
        match pattern {
            Pattern::Value { value, span } => {
                self.pattern_test(subject, CompareOperator::Eq, value.clone(), *span)
            }
            Pattern::Singleton { value, span } => {
                let value = Expr::Constant {
                    value: value.clone(),
                    span: *span,
                };
                self.pattern_test(subject, CompareOperator::Is, value, *span)
            }
            Pattern::As {
                pattern: inner,
                name,
                span,
            } => {
                let mut steps = Vec::new();
                if let Some(inner) = inner {
                    steps.push(self.pattern(inner, subject));
                }
                if let Some(name) = name {
                    let value = self.expression(subject);
                    let target = Expr::Name {
                        id: name.name.clone(),
                        span: *span,
                    };
                    if let Some(symbol) = self.assign(&target, value.ty.clone(), &value) {
                        steps.push(TypedPattern::Bind(symbol, value));
                    }
                }
                match steps.len() {
                    1 => steps.remove(0),
                    _ => TypedPattern::All(steps),
                }
            }
            Pattern::Or { patterns, .. } => {
                let entry = self.assigned.clone();
                let mut ends = None;
                let mut alternatives = Vec::with_capacity(patterns.len());
                for alternative in patterns {
                    self.assigned.clone_from(&entry);
                    alternatives.push(self.pattern(alternative, subject));
                    ends = meet(ends, self.assigned.take());
                }
                self.assigned = ends;
                TypedPattern::Any(alternatives)
            }
            // The subset walker refuses these patterns before any body is checked.
            Pattern::Sequence { .. }
            | Pattern::Mapping { .. }
            | Pattern::Class { .. }
            | Pattern::Star { .. } => TypedPattern::All(Vec::new()),
        }
    }

    /// Returns the test `subject <operator> value`, checked as that comparison is.
    fn pattern_test(
        &mut self,
        subject: &Expr,
        operator: CompareOperator,
        value: Expr,
        span: Span,
    ) -> TypedPattern {
        let comparison = Expr::Compare {
            left: Box::new(subject.clone()),
            operators: vec![operator],
            comparators: vec![value],
            span,
        };
        let test = self.expression(&comparison);
        self.add_facts(test_facts(&test).0);
        TypedPattern::Test(test)
    }
}
