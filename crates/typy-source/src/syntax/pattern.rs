//! The patterns of a `match` statement (CST-06), as CPython's `ast` module defines them.

use typy_diagnostics::Span;

use super::expression::{Constant, Expr};
use super::statement::Stmt;
use super::tree::Identifier;

/// One `case` of a `match` statement.
#[derive(Clone, Debug, PartialEq)]
#[allow(
    missing_docs,
    reason = "fields follow the CPython ast node of the same name"
)]
pub struct MatchCase {
    pub pattern: Pattern,
    /// The `if` condition after the pattern.
    pub guard: Option<Expr>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

/// A pattern.
#[derive(Clone, Debug, PartialEq)]
#[allow(
    missing_docs,
    reason = "fields follow the CPython ast node of the same name"
)]
pub enum Pattern {
    /// A literal or a dotted name, which the subject equals: `1`, `"a"`, `-2`, `Color.RED`.
    Value { value: Expr, span: Span },
    /// `None`, `True` or `False`, which the subject is.
    Singleton { value: Constant, span: Span },
    /// `[p, q, *rest]` or `(p, q)`: a sequence whose items match.
    Sequence { patterns: Vec<Pattern>, span: Span },
    /// `{"key": p, **rest}`: a mapping whose values for the keys match.
    Mapping {
        keys: Vec<Expr>,
        patterns: Vec<Pattern>,
        rest: Option<Identifier>,
        span: Span,
    },
    /// `Point(p, y=q)`: an instance of the class whose attributes match.
    Class {
        class: Expr,
        patterns: Vec<Pattern>,
        keywords: Vec<(Identifier, Pattern)>,
        span: Span,
    },
    /// `*rest` inside a sequence pattern; `*_` binds nothing.
    Star {
        name: Option<Identifier>,
        span: Span,
    },
    /// `p as name`, a capture `name` when there is no pattern, and the wildcard `_` when
    /// there is neither.
    As {
        pattern: Option<Box<Pattern>>,
        name: Option<Identifier>,
        span: Span,
    },
    /// `p | q`: the first alternative that matches.
    Or { patterns: Vec<Pattern>, span: Span },
}

impl Pattern {
    /// Returns where the pattern is written.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Value { span, .. }
            | Self::Singleton { span, .. }
            | Self::Sequence { span, .. }
            | Self::Mapping { span, .. }
            | Self::Class { span, .. }
            | Self::Star { span, .. }
            | Self::As { span, .. }
            | Self::Or { span, .. } => *span,
        }
    }

    /// Returns the names the pattern binds, in source order: captures, `as` names, starred
    /// names and a mapping's rest.
    #[must_use]
    pub fn bound_names(&self) -> Vec<&Identifier> {
        let mut names = Vec::new();
        self.add_bound_names(&mut names);
        names
    }

    fn add_bound_names<'pattern>(&'pattern self, names: &mut Vec<&'pattern Identifier>) {
        match self {
            Self::Value { .. } | Self::Singleton { .. } => {}
            Self::Sequence { patterns, .. } | Self::Or { patterns, .. } => {
                for pattern in patterns {
                    pattern.add_bound_names(names);
                }
            }
            Self::Mapping { patterns, rest, .. } => {
                for pattern in patterns {
                    pattern.add_bound_names(names);
                }
                names.extend(rest);
            }
            Self::Class {
                patterns, keywords, ..
            } => {
                for pattern in patterns
                    .iter()
                    .chain(keywords.iter().map(|(_, pattern)| pattern))
                {
                    pattern.add_bound_names(names);
                }
            }
            Self::Star { name, .. } => names.extend(name),
            Self::As { pattern, name, .. } => {
                if let Some(pattern) = pattern {
                    pattern.add_bound_names(names);
                }
                names.extend(name);
            }
        }
    }

    /// Returns the expressions the pattern holds: values, mapping keys and class names.
    #[must_use]
    pub fn expressions(&self) -> Vec<&Expr> {
        let mut found = Vec::new();
        self.add_expressions(&mut found);
        found
    }

    fn add_expressions<'pattern>(&'pattern self, found: &mut Vec<&'pattern Expr>) {
        match self {
            Self::Value { value, .. } => found.push(value),
            Self::Singleton { .. } | Self::Star { .. } => {}
            Self::Sequence { patterns, .. } | Self::Or { patterns, .. } => {
                for pattern in patterns {
                    pattern.add_expressions(found);
                }
            }
            Self::Mapping { keys, patterns, .. } => {
                found.extend(keys);
                for pattern in patterns {
                    pattern.add_expressions(found);
                }
            }
            Self::Class {
                class,
                patterns,
                keywords,
                ..
            } => {
                found.push(class);
                for pattern in patterns
                    .iter()
                    .chain(keywords.iter().map(|(_, pattern)| pattern))
                {
                    pattern.add_expressions(found);
                }
            }
            Self::As { pattern, .. } => {
                if let Some(pattern) = pattern {
                    pattern.add_expressions(found);
                }
            }
        }
    }

    /// Gives the pattern and everything in it the span `span`.
    pub(crate) fn relocate(&mut self, span: Span) {
        match self {
            Self::Value { value, span: at } => {
                *at = span;
                value.relocate(span);
            }
            Self::Singleton { span: at, .. } => *at = span,
            Self::Sequence { patterns, span: at } | Self::Or { patterns, span: at } => {
                *at = span;
                for pattern in patterns {
                    pattern.relocate(span);
                }
            }
            Self::Mapping {
                keys,
                patterns,
                rest,
                span: at,
            } => {
                *at = span;
                for key in keys {
                    key.relocate(span);
                }
                for pattern in patterns {
                    pattern.relocate(span);
                }
                if let Some(rest) = rest {
                    rest.span = span;
                }
            }
            Self::Class {
                class,
                patterns,
                keywords,
                span: at,
            } => {
                *at = span;
                class.relocate(span);
                for pattern in patterns {
                    pattern.relocate(span);
                }
                for (name, pattern) in keywords {
                    name.span = span;
                    pattern.relocate(span);
                }
            }
            Self::Star { name, span: at } => {
                *at = span;
                if let Some(name) = name {
                    name.span = span;
                }
            }
            Self::As {
                pattern,
                name,
                span: at,
            } => {
                *at = span;
                if let Some(pattern) = pattern {
                    pattern.relocate(span);
                }
                if let Some(name) = name {
                    name.span = span;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
