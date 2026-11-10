//! Lowers exceptions: a call of an exception class, `raise`, `try` with its `except` clauses,
//! and `str` and `repr` of an exception (SMT-06, CST-04, EXC-01, EXC-04, EXC-08, ADR-0013).

use typy_check::{ExceptionClass, RaiseCause, Type, TypedExpr, TypedHandler, TypedStmt};
use typy_diagnostics::Span;
use typy_tyir::{
    BlockId, Constant, Operation, RuntimeFunction, Terminator, TyirType, TypeId, TypeObjectRef,
    ValueId,
};

use crate::function::{Cleanup, FunctionLowerer, Handled};
use crate::ownership::Lowered;

/// Returns the type object of an exception class.
pub(crate) fn exception_class(class: &ExceptionClass) -> TypeObjectRef {
    let ExceptionClass::Builtin(builtin) = class;
    TypeObjectRef::Exception(builtin.name())
}

impl FunctionLowerer<'_> {
    /// Lowers a call of an exception class: the arguments are evaluated in order into a tuple,
    /// which the new exception copies, and the tuple is released.
    pub(crate) fn new_exception(
        &mut self,
        class: &ExceptionClass,
        arguments: &[TypedExpr],
        span: Span,
    ) -> Lowered {
        let class = self.constant(Constant::TypeObject(exception_class(class)), span);
        let types = arguments.iter().map(|value| value.ty.clone()).collect();
        let layout = self.lowered_type(&Type::Tuple(types));
        let TyirType::Tuple(id) = layout else {
            unreachable!("a tuple type lowers to a tuple");
        };
        let descriptor = self.constant(Constant::TupleDescriptor(id), span);
        if arguments.is_empty() {
            let unit = self.constant(Constant::Unit, span);
            let nothing = self.bits(Self::borrowed(unit, TyirType::Unit), span);
            return self.call_runtime(
                RuntimeFunction::ExceptionNew,
                vec![class, descriptor, nothing],
                span,
            );
        }
        let tuple = self.tuple_display(arguments, layout, span);
        let address = self.bits(Self::borrowed(tuple.value, tuple.ty), span);
        let exception = self.call_runtime(
            RuntimeFunction::ExceptionNew,
            vec![class, descriptor, address],
            span,
        );
        self.release(tuple, span);
        exception
    }

    /// Lowers `raise`: the exception, or the one being handled for a bare `raise`, takes its
    /// cause, then the runtime raises it and the error path of the call takes it on. Nothing
    /// after it runs.
    pub(crate) fn raise_statement(
        &mut self,
        exception: Option<&TypedExpr>,
        cause: &RaiseCause,
        span: Span,
    ) {
        let exception = match exception {
            Some(exception) => self.expression(exception),
            None => self.call_runtime(RuntimeFunction::ExceptionHandled, Vec::new(), span),
        };
        match cause {
            RaiseCause::Absent => {}
            RaiseCause::None => {
                let unit = self.constant(Constant::Unit, span);
                let none = self.bits(Self::borrowed(unit, TyirType::Unit), span);
                self.call_runtime(
                    RuntimeFunction::ExceptionSetCause,
                    vec![exception.value, none],
                    span,
                );
            }
            RaiseCause::Exception(cause) => {
                let cause = self.expression(cause);
                let bits = self.bits(Self::borrowed(cause.value, cause.ty), span);
                self.call_runtime(
                    RuntimeFunction::ExceptionSetCause,
                    vec![exception.value, bits],
                    span,
                );
                self.release(cause, span);
            }
        }
        self.call_runtime(RuntimeFunction::ExceptionThrow, vec![exception.value], span);
        // The error path released the exception with the other temporaries, and no code
        // follows, so no later path may release it again.
        self.untrack(exception.value);
        self.builder.terminate(Terminator::Unreachable);
    }

    /// Returns `str(exception)`, or `repr(exception)` when `repr` is set, through the
    /// exception's class. The exception is not released.
    pub(crate) fn exception_text(&mut self, lowered: Lowered, repr: bool, span: Span) -> Lowered {
        let flag = self.constant(Constant::Int(i64::from(repr).to_string()), span);
        self.call_runtime(
            RuntimeFunction::ExceptionText,
            vec![lowered.value, flag],
            span,
        )
    }

    /// Returns `type(exception)`, the class the exception was made from, which may derive from
    /// its static class. The exception is not released.
    pub(crate) fn exception_type(&mut self, lowered: &Lowered, span: Span) -> Lowered {
        // A class is immortal, so the result is borrowed and never counted.
        let class = self.call_runtime(RuntimeFunction::ExceptionClass, vec![lowered.value], span);
        self.borrowed_result(class)
    }

    /// Lowers `try` with its `except` clauses and its `else` block. An exception raised in the
    /// body goes to a dispatch block, which tests the clauses in order; one that no clause
    /// matches goes on as if raised after the `try`. `else` runs after the body completes,
    /// outside the body's clauses.
    pub(crate) fn try_statement(
        &mut self,
        body: &[TypedStmt],
        handlers: &[TypedHandler],
        orelse: &[TypedStmt],
        span: Span,
    ) {
        let dispatch = self.builder.create_block();
        let exception = self
            .builder
            .add_block_parameter(dispatch, TyirType::EXCEPTION);
        let after = self.builder.create_block();
        self.cleanups.push(Cleanup::Handlers { dispatch });
        self.statements(body);
        self.cleanups.pop();
        self.statements(orelse);
        self.jump_open(after);
        self.builder.switch_to(dispatch);
        for handler in handlers {
            let matched = self.builder.create_block();
            for class in &handler.classes {
                let next = self.builder.create_block();
                let class = self.constant(Constant::TypeObject(exception_class(class)), span);
                let test = self
                    .call_runtime(
                        RuntimeFunction::ExceptionMatches,
                        vec![exception, class],
                        span,
                    )
                    .value;
                self.builder.terminate(Terminator::Branch {
                    condition: test,
                    then: matched,
                    otherwise: next,
                });
                self.builder.switch_to(next);
            }
            let unmatched = self.builder.create_block();
            self.jump_open(unmatched);
            self.builder.switch_to(matched);
            self.handler(handler, exception, after, span);
            self.builder.switch_to(unmatched);
        }
        self.propagate(exception, span);
        self.builder.switch_to(after);
    }

    /// Lowers the body of an `except` clause that matched `exception`, which the clause now
    /// owns: it becomes the exception being handled, and the clause's variable holds it.
    fn handler(&mut self, clause: &TypedHandler, exception: ValueId, after: BlockId, span: Span) {
        let previous = self
            .call_runtime(RuntimeFunction::ExceptionEnter, vec![exception], span)
            .value;
        let handled = match clause.name {
            Some(symbol) => {
                let slot = self.slots[&symbol];
                let owned = Lowered {
                    value: exception,
                    ty: TyirType::EXCEPTION,
                    owned: true,
                };
                self.store(slot, owned, span);
                Handled::Slot(slot)
            }
            None => Handled::Value(exception),
        };
        self.cleanups.push(Cleanup::Handling {
            previous,
            exception: handled,
        });
        self.statements(&clause.body);
        if !self.builder.is_terminated() {
            self.run_cleanups(self.cleanups.len() - 1, span);
        }
        self.cleanups.pop();
        self.jump_open(after);
    }

    /// Runs, innermost first, the cleanups opened after the first `depth`, as a way out of
    /// them does; they stay open for the other ways out.
    pub(crate) fn run_cleanups(&mut self, depth: usize, span: Span) {
        let open: Vec<Cleanup> = self.cleanups[depth..].iter().rev().copied().collect();
        for cleanup in open {
            if let Cleanup::Handling {
                previous,
                exception,
            } = cleanup
            {
                self.call_runtime(RuntimeFunction::ExceptionLeave, vec![previous], span);
                match exception {
                    Handled::Value(value) => {
                        self.builder.push(Operation::Decrement(value), span);
                    }
                    Handled::Slot(slot) => {
                        let held = self.builder.push_value(
                            Operation::Load(slot),
                            TyirType::EXCEPTION,
                            span,
                        );
                        let none = self.constant(Constant::NoInstance(TypeId::EXCEPTION), span);
                        self.builder
                            .push(Operation::Store { slot, value: none }, span);
                        self.builder.push(Operation::Decrement(held), span);
                    }
                }
            }
        }
    }

    /// Ends an error path: releases the live temporaries, leaves every `except` clause on the
    /// way out, then hands the exception to the nearest enclosing `try` body's clauses, or
    /// releases the slots and returns it to the caller.
    pub(crate) fn propagate(&mut self, exception: ValueId, span: Span) {
        self.release_live_temporaries(span);
        let target = self
            .cleanups
            .iter()
            .rposition(|cleanup| matches!(cleanup, Cleanup::Handlers { .. }));
        self.run_cleanups(target.map_or(0, |index| index + 1), span);
        if let Some(Cleanup::Handlers { dispatch }) = target.map(|index| self.cleanups[index]) {
            self.builder.terminate(Terminator::Jump {
                target: dispatch,
                arguments: vec![exception],
            });
        } else {
            self.release_slots(span);
            self.builder.terminate(Terminator::ReturnError(exception));
        }
    }

    /// Jumps to `target` unless the current block has already ended.
    fn jump_open(&mut self, target: BlockId) {
        if !self.builder.is_terminated() {
            self.builder.terminate(Terminator::Jump {
                target,
                arguments: Vec::new(),
            });
        }
    }
}
