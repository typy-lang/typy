//! Converts the cases and patterns of a `match` statement (CST-06).

use ruff_python_ast as ast;
use ruff_text_size::Ranged;

use super::Converter;
use crate::syntax::{Constant, MatchCase, Pattern};

impl Converter<'_> {
    pub(super) fn match_case(&self, case: &ast::MatchCase) -> MatchCase {
        MatchCase {
            pattern: self.pattern(&case.pattern),
            guard: case.guard.as_deref().map(|guard| self.expression(guard)),
            body: self.statements(&case.body),
            span: self.span(case.range),
        }
    }

    fn patterns(&self, patterns: &[ast::Pattern]) -> Vec<Pattern> {
        patterns
            .iter()
            .map(|pattern| self.pattern(pattern))
            .collect()
    }

    pub(super) fn pattern(&self, pattern: &ast::Pattern) -> Pattern {
        let span = self.span(pattern.range());
        match pattern {
            ast::Pattern::MatchValue(node) => Pattern::Value {
                value: self.expression(&node.value),
                span,
            },
            ast::Pattern::MatchSingleton(node) => Pattern::Singleton {
                value: match node.value {
                    ast::Singleton::None => Constant::None,
                    ast::Singleton::True => Constant::Bool(true),
                    ast::Singleton::False => Constant::Bool(false),
                },
                span,
            },
            ast::Pattern::MatchSequence(node) => Pattern::Sequence {
                patterns: self.patterns(&node.patterns),
                span,
            },
            ast::Pattern::MatchMapping(node) => Pattern::Mapping {
                keys: node.keys.iter().map(|key| self.expression(key)).collect(),
                patterns: self.patterns(&node.patterns),
                rest: node.rest.as_ref().map(|rest| self.identifier(rest)),
                span,
            },
            ast::Pattern::MatchClass(node) => Pattern::Class {
                class: self.expression(&node.cls),
                patterns: self.patterns(&node.arguments.patterns),
                keywords: node
                    .arguments
                    .keywords
                    .iter()
                    .map(|keyword| {
                        (
                            self.identifier(&keyword.attr),
                            self.pattern(&keyword.pattern),
                        )
                    })
                    .collect(),
                span,
            },
            ast::Pattern::MatchStar(node) => Pattern::Star {
                name: node.name.as_ref().map(|name| self.identifier(name)),
                span,
            },
            ast::Pattern::MatchAs(node) => Pattern::As {
                pattern: node
                    .pattern
                    .as_deref()
                    .map(|inner| Box::new(self.pattern(inner))),
                name: node.name.as_ref().map(|name| self.identifier(name)),
                span,
            },
            ast::Pattern::MatchOr(node) => Pattern::Or {
                patterns: self.patterns(&node.patterns),
                span,
            },
        }
    }
}
