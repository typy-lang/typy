//! Lowers `match` statements (CST-06): the subject is stored once when it is not a local,
//! then each case's pattern runs its tests and assignments, its guard is tested, and the
//! first case that matches runs its body.

use typy_check::{SymbolId, TypedCase, TypedExpr, TypedPattern};
use typy_diagnostics::Span;
use typy_tyir::{BlockId, Terminator};

use crate::function::FunctionLowerer;

impl FunctionLowerer<'_> {
    pub(crate) fn match_statement(
        &mut self,
        subject: Option<&(SymbolId, TypedExpr)>,
        cases: &[TypedCase],
        span: Span,
    ) {
        if let Some((symbol, value)) = subject {
            let lowered = self.expression(value);
            self.assign_name(*symbol, lowered, span);
        }
        let join = self.builder.create_block();
        for case in cases {
            let next = self.builder.create_block();
            self.pattern(&case.pattern, next, span);
            if let Some(guard) = &case.guard {
                let condition = self.condition(guard);
                let then = self.builder.create_block();
                self.builder.terminate(Terminator::Branch {
                    condition,
                    then,
                    otherwise: next,
                });
                self.builder.switch_to(then);
            }
            self.statements(&case.body);
            self.jump_if_open(join, span);
            self.builder.switch_to(next);
        }
        self.jump_if_open(join, span);
        self.builder.switch_to(join);
    }

    /// Lowers a pattern: the code continues where it matches and goes to `fail` where it
    /// does not.
    fn pattern(&mut self, pattern: &TypedPattern, fail: BlockId, span: Span) {
        match pattern {
            TypedPattern::Test(test) => {
                let condition = self.condition(test);
                let then = self.builder.create_block();
                self.builder.terminate(Terminator::Branch {
                    condition,
                    then,
                    otherwise: fail,
                });
                self.builder.switch_to(then);
            }
            TypedPattern::Bind(symbol, value) => {
                let lowered = self.expression(value);
                self.assign_name(*symbol, lowered, span);
            }
            TypedPattern::All(patterns) => {
                for inner in patterns {
                    self.pattern(inner, fail, span);
                }
            }
            TypedPattern::Any(alternatives) => {
                let matched = self.builder.create_block();
                for (index, alternative) in alternatives.iter().enumerate() {
                    let otherwise = if index + 1 == alternatives.len() {
                        fail
                    } else {
                        self.builder.create_block()
                    };
                    self.pattern(alternative, otherwise, span);
                    self.jump_if_open(matched, span);
                    if otherwise != fail {
                        self.builder.switch_to(otherwise);
                    }
                }
                self.builder.switch_to(matched);
            }
        }
    }
}
