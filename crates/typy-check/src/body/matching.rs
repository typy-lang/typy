//! Checks `match` statements (CST-06). Each pattern becomes tests and assignments written in
//! terms of the subject, checked as the expressions they stand for, so `case 1:` is checked
//! as `subject == 1` and `case None:` narrows as `subject is None` does.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Severity, Span};
use typy_source::syntax::{CompareOperator, Expr, MatchCase, Pattern};

use super::BodyChecker;
use super::flow::{meet, test_facts};
use super::pattern_coverage::{Member, members};
use crate::typed::{
    SymbolId, SymbolKind, TypedCase, TypedExpr, TypedExprKind, TypedPattern, TypedStmt,
};
use crate::types::Type;

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
        let errors = self.error_count();
        let checked = self.expression(subject);
        let subject_type = checked.ty.clone();
        // A subject of literal types is covered by a case for each of its values (TYP-28).
        let covered_type = self
            .literal_of(&checked)
            .unwrap_or_else(|| subject_type.clone());
        let (name, stored) =
            if let (TypedExprKind::Local(_), Expr::Name { id, .. }) = (&checked.kind, subject) {
                (id.clone(), None)
            } else {
                let (name, symbol) = self.hidden_local(&checked, subject.span());
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
            // A case whose pattern the types rule out never runs (TYP-34).
            if !super::flow::pattern_reach(&pattern) {
                self.assigned = None;
            }
            let guard = case.guard.as_ref().map(|guard| {
                let guard = self.truth_test(guard);
                self.add_facts(test_facts(&guard).0);
                guard
            });
            let body = self.block(&case.body);
            ends = meet(ends, self.assigned.take());
            self.assigned = entry;
            // A later case knows this one failed when it is one test without a guard, or
            // alternatives that are each one test.
            if guard.is_none() {
                let tests: Vec<&TypedExpr> = match &pattern {
                    TypedPattern::Test(test) => vec![test],
                    TypedPattern::Any(alternatives) => alternatives
                        .iter()
                        .map_while(|alternative| match alternative {
                            TypedPattern::Test(test) => Some(test),
                            _ => None,
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                let every = match &pattern {
                    TypedPattern::Any(alternatives) => tests.len() == alternatives.len(),
                    _ => true,
                };
                if every {
                    for test in tests {
                        self.add_facts(test_facts(test).1);
                    }
                }
            }
            matched_all |= guard.is_none() && pattern.always_matches();
            typed.push(TypedCase {
                pattern,
                guard,
                body,
            });
        }
        let uncovered = self.uncovered(&covered_type, cases);
        if uncovered.is_empty() {
            matched_all = true;
        } else if members(&covered_type).len() > 1 && self.error_count() == errors {
            // A `match` with an error has that one diagnostic, so no warning follows it.
            self.missing_cases(&uncovered, subject.span());
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

    /// Returns how many errors have been reported.
    fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity() == Severity::Error)
            .count()
    }

    /// Warns that the cases leave members of the subject's type unmatched (CST-07,
    /// ADR-0033).
    fn missing_cases(&mut self, uncovered: &[Member], span: Span) {
        let names: Vec<String> = uncovered.iter().map(ToString::to_string).collect();
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::warning(401),
                span,
                format!("this `match` has no case for {}", names.join(", ")),
            )
            .with_rule(RuleId::CST_07)
            .with_help("add a case for each, or end with `case _:` for the rest"),
        );
    }

    /// Stores `value` in a new variable whose name no program can write, and returns the
    /// name and the variable.
    fn hidden_local(&mut self, value: &TypedExpr, span: Span) -> (String, SymbolId) {
        let name = format!("<match {}>", self.local_order.len());
        let symbol = self
            .program
            .add_symbol(&name, SymbolKind::Local, value.ty.clone(), span);
        self.locals.insert(name.clone(), symbol);
        self.local_order.push(symbol);
        let target = Expr::Name {
            id: name.clone(),
            span,
        };
        self.assign(&target, value.ty.clone(), value);
        (name, symbol)
    }

    /// Adds to `steps` what matching `pattern` against the part `value` of the subject does:
    /// a capture binds the part, and any other pattern but `_` tests it, stored first so it
    /// is read once and narrows as a variable does.
    pub(super) fn part_pattern(
        &mut self,
        pattern: &Pattern,
        value: &Expr,
        steps: &mut Vec<TypedPattern>,
    ) {
        match pattern {
            Pattern::As {
                pattern: None,
                name: None,
                ..
            } => {}
            Pattern::As {
                pattern: None,
                name: Some(_),
                ..
            } => steps.push(self.pattern(pattern, value)),
            _ => {
                let checked = self.expression(value);
                let (name, symbol) = self.hidden_local(&checked, value.span());
                steps.push(TypedPattern::Bind(symbol, checked));
                let part = Expr::Name {
                    id: name,
                    span: value.span(),
                };
                steps.push(self.pattern(pattern, &part));
            }
        }
    }

    /// Binds each name `pattern` binds to a value of no type, after an error, so the names
    /// are not reported again as unassigned.
    pub(super) fn bind_after_error(&mut self, pattern: &Pattern, error: &TypedExpr) {
        for name in pattern.bound_names() {
            let target = Expr::Name {
                id: name.name.clone(),
                span: name.span,
            };
            self.assign(&target, Type::Error, error);
        }
    }

    /// Returns what a pattern does to `subject`, adding what each test shows to what is known
    /// while the rest of the case is checked.
    pub(super) fn pattern(&mut self, pattern: &Pattern, subject: &Expr) -> TypedPattern {
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
            Pattern::Sequence { patterns, span } => {
                self.sequence_pattern(pattern, patterns, subject, *span)
            }
            Pattern::Mapping {
                keys,
                patterns,
                rest,
                span,
            } => self.mapping_pattern(pattern, keys, patterns, rest.as_ref(), subject, *span),
            Pattern::Class {
                class,
                patterns,
                keywords,
                span,
            } => self.class_pattern(pattern, class, (patterns, keywords), subject, *span),
            // A starred name only appears inside a sequence pattern, which binds it.
            Pattern::Star { .. } => TypedPattern::All(Vec::new()),
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
