//! Checks `match` statements (CST-06): the `SyntaxError`s CPython's compiler reports for
//! patterns, and the patterns TyPy takes.

use std::collections::BTreeSet;

use typy_diagnostics::{RuleId, Span};
use typy_source::syntax::{Expr, Identifier, MatchCase, Pattern};

use super::feature::planned;
use super::{Place, Walker};

impl Walker {
    /// Checks a `match` statement: its subject, and each case's pattern, guard and body.
    pub(super) fn match_statement(&mut self, subject: &Expr, cases: &[MatchCase], place: Place) {
        self.expression(subject);
        for (index, case) in cases.iter().enumerate() {
            let last = index + 1 == cases.len();
            // A case that matches anything, without a guard, leaves later cases unreachable.
            if !last
                && case.guard.is_none()
                && let Some(message) = irrefutable(&case.pattern)
            {
                self.syntax_error(case.pattern.span(), &message);
            }
            let mut bound = BTreeSet::new();
            self.pattern(&case.pattern, &mut bound);
            if let Some(guard) = &case.guard {
                self.expression(guard);
            }
            self.block(&case.body, place);
        }
    }

    /// Checks a pattern, adding the names it binds to `bound`.
    fn pattern(&mut self, pattern: &Pattern, bound: &mut BTreeSet<String>) {
        match pattern {
            // An f-string is computed, so CPython's compiler refuses it as a value pattern.
            Pattern::Value {
                value: Expr::FString { .. },
                span,
            } => self.syntax_error(
                *span,
                "patterns may only match literals and attribute lookups",
            ),
            Pattern::Value { value, .. } => self.expression(value),
            Pattern::Singleton { .. } => {}
            Pattern::As {
                pattern: inner,
                name,
                span,
            } => {
                if let Some(inner) = inner {
                    self.pattern(inner, bound);
                }
                if let Some(name) = name {
                    self.bind_pattern_name(name, *span, bound);
                }
            }
            Pattern::Or { patterns, span } => self.or_pattern(patterns, *span, bound),
            Pattern::Sequence { span, .. }
            | Pattern::Mapping { span, .. }
            | Pattern::Class { span, .. }
            | Pattern::Star { span, .. } => {
                self.report(
                    *span,
                    planned(
                        "sequence, mapping and class patterns",
                        "0.5",
                        RuleId::CST_06,
                    ),
                );
            }
        }
    }

    /// Records a name a pattern binds: `__debug__` is CPython's error, and so is a name bound
    /// twice in one pattern.
    fn bind_pattern_name(&mut self, name: &Identifier, at: Span, bound: &mut BTreeSet<String>) {
        if name.name == "__debug__" {
            self.syntax_error(at, "cannot assign to __debug__");
        } else if !bound.insert(name.name.clone()) {
            let message = format!("multiple assignments to name '{}' in pattern", name.name);
            self.syntax_error(at, &message);
        } else {
            self.assigned_name(&name.name);
        }
    }

    /// Checks the alternatives of `p | q`: each binds the same names, and one that matches
    /// anything leaves those after it unreachable.
    fn or_pattern(&mut self, patterns: &[Pattern], span: Span, bound: &mut BTreeSet<String>) {
        let mut first: Option<BTreeSet<String>> = None;
        for (index, alternative) in patterns.iter().enumerate() {
            if index + 1 < patterns.len()
                && let Some(message) = irrefutable(alternative)
            {
                self.syntax_error(alternative.span(), &message);
            }
            let mut names = BTreeSet::new();
            self.pattern(alternative, &mut names);
            match &first {
                None => first = Some(names),
                Some(expected) if *expected != names => {
                    self.syntax_error(span, "alternative patterns bind different names");
                }
                Some(_) => {}
            }
        }
        for name in first.unwrap_or_default() {
            if !bound.insert(name.clone()) {
                let message = format!("multiple assignments to name '{name}' in pattern");
                self.syntax_error(span, &message);
            }
        }
    }
}

/// Returns CPython's message for a pattern that matches any subject: a capture or the
/// wildcard, alone, under `as`, or as an alternative.
fn irrefutable(pattern: &Pattern) -> Option<String> {
    match pattern {
        Pattern::As {
            pattern: None,
            name: Some(name),
            ..
        } => Some(format!(
            "name capture '{}' makes remaining patterns unreachable",
            name.name
        )),
        Pattern::As {
            pattern: None,
            name: None,
            ..
        } => Some("wildcard makes remaining patterns unreachable".to_owned()),
        Pattern::As {
            pattern: Some(inner),
            ..
        } => irrefutable(inner),
        Pattern::Or { patterns, .. } => patterns.iter().find_map(irrefutable),
        _ => None,
    }
}
