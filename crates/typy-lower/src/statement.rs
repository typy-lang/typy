//! Lowers statements, including loops with `break`, `continue` and `else`.

use typy_check::{SymbolId, TypedConstant, TypedExpr, TypedExprKind, TypedStmt};
use typy_diagnostics::Span;
use typy_tyir::{
    BoolOperator, Comparison, Constant, IntOperator, Operation, RuntimeFunction, ScalarKind,
    SlotId, Terminator, TyirType, ValueId,
};

use crate::function::{FunctionLowerer, LoopTargets};

impl FunctionLowerer<'_> {
    pub(crate) fn statements(&mut self, statements: &[TypedStmt]) {
        for statement in statements {
            self.statement(statement);
        }
    }

    fn slot(&self, symbol: SymbolId) -> SlotId {
        self.slots[&symbol]
    }

    #[allow(clippy::too_many_lines, reason = "one arm per statement kind")]
    fn statement(&mut self, statement: &TypedStmt) {
        match statement {
            TypedStmt::Assign {
                targets,
                value,
                span,
            } => {
                let lowered = self.expression(value);
                for (position, target) in targets.iter().enumerate() {
                    if position + 1 == targets.len() {
                        self.assign_target(target, lowered, *span);
                    } else {
                        self.assign_target(
                            target,
                            Self::borrowed(lowered.value, lowered.ty),
                            *span,
                        );
                    }
                }
            }
            TypedStmt::AugAssign {
                target,
                operator,
                value,
                span,
            } => {
                let result = match target {
                    typy_check::AssignTarget::Name(symbol) => {
                        self.program.symbol(*symbol).ty.clone()
                    }
                    typy_check::AssignTarget::Item { list, .. } => match &list.ty {
                        typy_check::Type::List(item) => item.as_ref().clone(),
                        _ => unreachable!("the checker allows an item target only on a list"),
                    },
                    typy_check::AssignTarget::Entry { dict, .. } => match &dict.ty {
                        typy_check::Type::Dict(_, value) => value.as_ref().clone(),
                        _ => unreachable!("the checker allows an entry target only on a dict"),
                    },
                    typy_check::AssignTarget::Attribute { object, index } => match &object.ty {
                        ty if let Some(class) = ty.class_id() => {
                            self.program.class(class).attributes[*index].ty.clone()
                        }
                        _ => unreachable!(
                            "the checker allows an attribute target only on an instance"
                        ),
                    },
                    typy_check::AssignTarget::Tuple(_) | typy_check::AssignTarget::Slice { .. } => {
                        typy_check::Type::Error
                    }
                };
                self.augmented_assignment(target, *operator, value, &result, *span);
            }
            TypedStmt::Expr { value, span } => {
                let lowered = self.expression(value);
                self.release(lowered, *span);
            }
            TypedStmt::Return { value, span } => {
                // A returned variable hands its slot's reference to the caller (MEM-04), unless
                // an `except` clause is open, whose cleanup may release the variable's value.
                let moved = match value.as_ref().map(|value| &value.kind) {
                    Some(typy_check::TypedExprKind::Local(symbol)) if self.cleanups.is_empty() => {
                        let slot = self.slots[symbol];
                        (!self.borrowed.contains(&slot)).then_some(slot)
                    }
                    _ => None,
                };
                let value = match (value, moved) {
                    (Some(_), Some(slot)) => self.load(slot, *span).value,
                    (Some(value), None) => {
                        let lowered = self.expression(value);
                        self.take_reference(lowered, *span)
                    }
                    (None, _) => self.constant(Constant::Unit, *span),
                };
                // A `finally` block runs after the value is computed, and an error in it
                // releases the value with the other temporaries.
                let returned = self.builder.return_type();
                let held = !self.cleanups.is_empty() && self.counted(returned);
                if held {
                    self.temporaries.push(value);
                }
                self.run_cleanups(0, *span);
                if held {
                    self.untrack(value);
                }
                self.release_slots_but(moved, *span);
                self.builder.terminate(Terminator::Return(value));
            }
            TypedStmt::Raise {
                exception,
                cause,
                span,
            } => self.raise_statement(exception.as_ref(), cause, *span),
            TypedStmt::With {
                manager,
                enter,
                exit,
                suppresses,
                target,
                body,
                span,
            } => self.with_statement(
                manager,
                *enter,
                *exit,
                *suppresses,
                target.as_ref(),
                body,
                *span,
            ),
            TypedStmt::Delete { targets, span } => {
                for target in targets {
                    self.delete(target, *span);
                }
            }
            TypedStmt::Assert {
                test,
                exception,
                span,
            } => {
                // The message is evaluated only when the test is false, as in CPython.
                let condition = self.condition(test);
                let passed = self.builder.create_block();
                let failed = self.builder.create_block();
                self.builder.terminate(Terminator::Branch {
                    condition,
                    then: passed,
                    otherwise: failed,
                });
                self.builder.switch_to(failed);
                self.raise_statement(Some(exception), &typy_check::RaiseCause::Absent, *span);
                self.builder.switch_to(passed);
            }
            TypedStmt::If {
                test,
                body,
                orelse,
                span,
            } => {
                let condition = self.condition(test);
                let then = self.builder.create_block();
                let otherwise = self.builder.create_block();
                let join = self.builder.create_block();
                self.builder.terminate(Terminator::Branch {
                    condition,
                    then,
                    otherwise,
                });
                for (block, statements) in [(then, body), (otherwise, orelse)] {
                    self.builder.switch_to(block);
                    self.statements(statements);
                    self.jump_if_open(join, *span);
                }
                self.builder.switch_to(join);
            }
            TypedStmt::While {
                test,
                body,
                orelse,
                span,
            } => self.while_loop(test, body, orelse, *span),
            TypedStmt::ForEach {
                target,
                source,
                body,
                orelse,
                span,
            } => self.for_each(target, source, body, orelse, *span),
            TypedStmt::ForRange {
                target,
                start,
                stop,
                step,
                body,
                orelse,
                span,
            } => {
                self.for_range(*target, [start, stop, step], body, orelse, *span);
            }
            TypedStmt::Pass { .. } => {}
            TypedStmt::Break { span } => {
                if let Some(targets) = self.loops.last().copied() {
                    self.run_cleanups(targets.cleanups, *span);
                    self.jump_if_open(targets.break_target, *span);
                }
            }
            TypedStmt::Continue { span } => {
                if let Some(targets) = self.loops.last().copied() {
                    self.run_cleanups(targets.cleanups, *span);
                    self.jump_if_open(targets.continue_target, *span);
                }
            }
            TypedStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
                span,
            } => self.try_statement(body, handlers, orelse, finalbody, *span),
        }
    }

    fn jump_if_open(&mut self, target: typy_tyir::BlockId, _span: Span) {
        if !self.builder.is_terminated() {
            self.builder.terminate(Terminator::Jump {
                target,
                arguments: Vec::new(),
            });
        }
    }

    /// Evaluates an expression as a condition and returns its truth value.
    fn condition(&mut self, test: &TypedExpr) -> typy_tyir::ValueId {
        let lowered = self.expression(test);
        let truth = self.truth(lowered, test.span);
        self.release(lowered, test.span);
        truth
    }

    fn while_loop(
        &mut self,
        test: &TypedExpr,
        body: &[TypedStmt],
        orelse: &[TypedStmt],
        span: Span,
    ) {
        let header = self.builder.create_block();
        let inside = self.builder.create_block();
        let otherwise = self.builder.create_block();
        let exit = self.builder.create_block();
        self.jump_if_open(header, span);
        self.builder.switch_to(header);
        let condition = self.condition(test);
        self.builder.terminate(Terminator::Branch {
            condition,
            then: inside,
            otherwise,
        });
        self.builder.switch_to(inside);
        self.loops.push(LoopTargets {
            break_target: exit,
            continue_target: header,
            cleanups: self.cleanups.len(),
        });
        self.statements(body);
        self.loops.pop();
        self.jump_if_open(header, span);
        self.builder.switch_to(otherwise);
        self.statements(orelse);
        self.jump_if_open(exit, span);
        self.builder.switch_to(exit);
    }

    /// `for target in range(start, stop, step)`: the arguments are evaluated once, and a zero step raises `ValueError`.
    fn for_range(
        &mut self,
        target: SymbolId,
        arguments: [&TypedExpr; 3],
        body: &[TypedStmt],
        orelse: &[TypedStmt],
        span: Span,
    ) {
        let [first, last, increment] = arguments;
        let counter = self.hidden_slot("counter", TyirType::Int);
        let limit = self.hidden_slot("limit", TyirType::Int);
        let stride = self.hidden_slot("stride", TyirType::Int);
        for (slot, value) in [(counter, first), (limit, last), (stride, increment)] {
            let lowered = self.expression(value);
            self.store(slot, lowered, span);
        }
        let constant_step = match &increment.kind {
            TypedExprKind::Constant(TypedConstant::Int(digits)) => Some(!digits.starts_with('-')),
            _ => None,
        };
        let constant_nonzero = matches!(&increment.kind, TypedExprKind::Constant(TypedConstant::Int(digits)) if digits != "0");
        if !constant_nonzero {
            let step_value = self.load(stride, span);
            self.call_runtime(
                RuntimeFunction::RangeCheckStep,
                vec![step_value.value],
                span,
            );
        }
        let header = self.builder.create_block();
        let inside = self.builder.create_block();
        let advance = self.builder.create_block();
        let otherwise = self.builder.create_block();
        let exit = self.builder.create_block();
        self.jump_if_open(header, span);
        self.builder.switch_to(header);
        let condition = self.range_condition(
            [counter, limit, stride],
            constant_step.filter(|_| constant_nonzero),
            span,
        );
        self.builder.terminate(Terminator::Branch {
            condition,
            then: inside,
            otherwise,
        });
        self.builder.switch_to(inside);
        let value = self.load(counter, span);
        let target_slot = self.slot(target);
        self.store(target_slot, value, span);
        self.loops.push(LoopTargets {
            break_target: exit,
            continue_target: advance,
            cleanups: self.cleanups.len(),
        });
        self.statements(body);
        self.loops.pop();
        self.jump_if_open(advance, span);
        self.builder.switch_to(advance);
        let current = self.load(counter, span).value;
        let step_value = self.load(stride, span).value;
        let next = self.builder.push_value(
            Operation::IntBinary {
                operator: IntOperator::Add,
                left: current,
                right: step_value,
            },
            TyirType::Int,
            span,
        );
        let next = self.produced(next, TyirType::Int);
        self.store(counter, next, span);
        self.jump_if_open(header, span);
        self.builder.switch_to(otherwise);
        self.statements(orelse);
        self.jump_if_open(exit, span);
        self.builder.switch_to(exit);
    }

    /// Returns whether a `range` loop continues: `counter < limit` for a positive step, `counter > limit` for a negative one; `rising_step` is known when the step is a constant.
    pub(crate) fn range_condition(
        &mut self,
        [counter, limit, stride]: [SlotId; 3],
        rising_step: Option<bool>,
        span: Span,
    ) -> ValueId {
        let current = self.load(counter, span).value;
        let end = self.load(limit, span).value;
        let compare = |lowerer: &mut Self, comparison| {
            lowerer.builder.push_value(
                Operation::Compare {
                    kind: ScalarKind::Int,
                    comparison,
                    left: current,
                    right: end,
                },
                TyirType::Bool,
                span,
            )
        };
        match rising_step {
            Some(true) => compare(self, Comparison::Less),
            Some(false) => compare(self, Comparison::Greater),
            None => {
                let rising = compare(self, Comparison::Less);
                let falling = compare(self, Comparison::Greater);
                let step_value = self.load(stride, span).value;
                let zero = self.constant(Constant::Int("0".to_owned()), span);
                let positive = self.builder.push_value(
                    Operation::Compare {
                        kind: ScalarKind::Int,
                        comparison: Comparison::Greater,
                        left: step_value,
                        right: zero,
                    },
                    TyirType::Bool,
                    span,
                );
                let negative =
                    self.builder
                        .push_value(Operation::BoolNot(positive), TyirType::Bool, span);
                let up = self.builder.push_value(
                    Operation::BoolBinary {
                        operator: BoolOperator::And,
                        left: positive,
                        right: rising,
                    },
                    TyirType::Bool,
                    span,
                );
                let down = self.builder.push_value(
                    Operation::BoolBinary {
                        operator: BoolOperator::And,
                        left: negative,
                        right: falling,
                    },
                    TyirType::Bool,
                    span,
                );
                self.builder.push_value(
                    Operation::BoolBinary {
                        operator: BoolOperator::Or,
                        left: up,
                        right: down,
                    },
                    TyirType::Bool,
                    span,
                )
            }
        }
    }
}
