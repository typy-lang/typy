//! Checks `match` statements (CST-06): the `SyntaxError`s CPython's compiler reports for
//! patterns, and the patterns TyPy takes.

use std::collections::BTreeSet;

use typy_diagnostics::Span;
use typy_source::syntax::{Expr, Identifier, MatchCase, Pattern};

use super::pattern_key::{key_repr, literal_key, same_key};

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
            Pattern::Sequence { patterns, span } => {
                if patterns
                    .iter()
                    .filter(|each| matches!(each, Pattern::Star { .. }))
                    .count()
                    > 1
                {
                    self.syntax_error(*span, "multiple starred names in sequence pattern");
                }
                for each in patterns {
                    self.pattern(each, bound);
                }
            }
            Pattern::Star { name, span } => {
                if let Some(name) = name {
                    self.bind_pattern_name(name, *span, bound);
                }
            }
            Pattern::Mapping {
                keys,
                patterns,
                rest,
                span,
            } => self.mapping_pattern(keys, patterns, rest.as_ref(), *span, bound),
            Pattern::Class {
                class,
                patterns,
                keywords,
                ..
            } => {
                self.expression(class);
                for each in patterns {
                    self.pattern(each, bound);
                }
                let mut attributes = BTreeSet::new();
                for (name, each) in keywords {
                    if !attributes.insert(name.name.as_str()) {
                        let message =
                            format!("attribute name repeated in class pattern: {}", name.name);
                        self.syntax_error(name.span, &message);
                    }
                    self.pattern(each, bound);
                }
            }
        }
    }

    /// Checks `{key: pattern, **rest}`: no literal key repeats, and the names bound are
    /// bound once. The parser refuses a key that is not a literal or a dotted name.
    fn mapping_pattern(
        &mut self,
        keys: &[Expr],
        patterns: &[Pattern],
        rest: Option<&Identifier>,
        span: Span,
        bound: &mut BTreeSet<String>,
    ) {
        let mut seen = Vec::new();
        for key in keys {
            self.expression(key);
            if let Some(literal) = literal_key(key) {
                if seen.iter().any(|earlier| same_key(earlier, &literal)) {
                    let message = format!(
                        "mapping pattern checks duplicate key ({})",
                        key_repr(&literal)
                    );
                    self.syntax_error(span, &message);
                }
                seen.push(literal);
            }
        }
        for each in patterns {
            self.pattern(each, bound);
        }
        if let Some(rest) = rest {
            self.bind_pattern_name(rest, span, bound);
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
