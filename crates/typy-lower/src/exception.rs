//! Lowers exceptions: a call of an exception class, `raise`, `try` with its `except` clauses,
//! `with`, and `str` and `repr` of an exception (SMT-06, CST-04, CST-05, EXC-01, EXC-04,
//! EXC-08, ADR-0013).

use std::rc::Rc;

use typy_check::{
    AssignTarget, ExceptionClass, InitCall, RaiseCause, Type, TypedExpr, TypedHandler, TypedStmt,
    WithProtocol,
};
use typy_diagnostics::Span;
use typy_tyir::{
    BlockId, Constant, Operation, RuntimeFunction, SlotId, Terminator, TyirType, TypeId,
    TypeObjectRef, ValueId,
};

use crate::function::{Cleanup, FunctionLowerer, Handled};
use crate::ownership::Lowered;

/// The arguments of an exception evaluated into a tuple.
#[derive(Clone, Copy)]
pub(crate) struct ExceptionTuple {
    /// The layout of the tuple.
    id: typy_tyir::TupleTypeId,
    /// The number of arguments.
    count: usize,
    /// The tuple, or none for no arguments.
    tuple: Option<Lowered>,
}

/// Returns the type object of an exception class.
pub(crate) fn exception_class(class: &ExceptionClass) -> TypeObjectRef {
    match class {
        ExceptionClass::Builtin(builtin) => TypeObjectRef::Exception(builtin.name()),
        ExceptionClass::Class(class) => TypeObjectRef::Class(class.id.0),
    }
}

impl FunctionLowerer<'_> {
    /// Lowers a call of an exception class: the arguments are evaluated in order into a tuple,
    /// which the new exception copies, and the tuple is released.
    pub(crate) fn new_exception(
        &mut self,
        class: &ExceptionClass,
        arguments: &[TypedExpr],
        init: Option<&InitCall>,
        span: Span,
    ) -> Lowered {
        if let Some(init) = init {
            return self.new_exception_with_init(class, init, span);
        }
        let tuple = self.exception_tuple(arguments, span);
        self.exception_of_tuple(class, tuple, span)
    }

    /// Evaluates the arguments of an exception in order into a tuple, none for no arguments.
    pub(crate) fn exception_tuple(
        &mut self,
        arguments: &[TypedExpr],
        span: Span,
    ) -> ExceptionTuple {
        let types: Vec<Type> = arguments.iter().map(|value| value.ty.clone()).collect();
        let id = self.tuple_type_id(&types);
        let tuple = (!arguments.is_empty())
            .then(|| self.tuple_display(arguments, TyirType::Tuple(id), span));
        ExceptionTuple {
            id,
            count: arguments.len(),
            tuple,
        }
    }

    /// Returns a new exception of `class` that copies the arguments `tuple`, which is
    /// released.
    pub(crate) fn exception_of_tuple(
        &mut self,
        class: &ExceptionClass,
        tuple: ExceptionTuple,
        span: Span,
    ) -> Lowered {
        // An `OSError` of two to five arguments reads them as `errno` and the rest, and one of
        // more reads none (EXC-14, EXC-16).
        let constructor = if class.builtin().is_os_error() && (2..=5).contains(&tuple.count) {
            RuntimeFunction::OsErrorNew
        } else {
            RuntimeFunction::ExceptionNew
        };
        let program_class = class.class_id();
        let class = self.constant(Constant::TypeObject(exception_class(class)), span);
        let descriptor = self.constant(Constant::TupleDescriptor(tuple.id), span);
        let address = if let Some(tuple) = tuple.tuple {
            self.bits(Self::borrowed(tuple.value, tuple.ty), span)
        } else {
            let unit = self.constant(Constant::Unit, span);
            self.bits(Self::borrowed(unit, TyirType::Unit), span)
        };
        let exception = self.call_runtime(constructor, vec![class, descriptor, address], span);
        if let Some(tuple) = tuple.tuple {
            self.release(tuple, span);
        }
        if let Some(program_class) = program_class {
            self.store_defaults(exception, program_class, span);
        }
        exception
    }

    /// Lowers a call of an exception class with an `__init__` of the program's: the arguments
    /// are evaluated in order, the exception is made with those written by position as its
    /// arguments, or with none for an `OSError`, then `__init__` runs on it, as CPython's
    /// `BaseException.__new__`, `OSError.__new__` and the class's `__init__` do (CLS-03,
    /// EXC-14).
    fn new_exception_with_init(
        &mut self,
        class: &ExceptionClass,
        init: &InitCall,
        span: Span,
    ) -> Lowered {
        let evaluated = self.evaluated_arguments(init.function, &init.arguments, span);
        let program_class = class.class_id();
        // `OSError.__new__` leaves the arguments of a class with an `__init__` of its own to
        // `OSError.__init__`, so the exception starts with none.
        let kept = if class.builtin().is_os_error() {
            0
        } else {
            init.positional
        };
        let positional = &init.arguments[..kept];
        let types: Vec<Type> = positional
            .iter()
            .map(|argument| argument.value.ty.clone())
            .collect();
        let id = self.tuple_type_id(&types);
        let layout = TyirType::Tuple(id);
        let items: Vec<ValueId> = evaluated[..kept]
            .iter()
            .map(|(_, _, lowered)| {
                if self.counted(lowered.ty) {
                    self.builder.push(Operation::Increment(lowered.value), span);
                }
                lowered.value
            })
            .collect();
        let tuple = self
            .builder
            .push_value(Operation::MakeTuple { ty: id, items }, layout, span);
        let tuple = self.produced(tuple, layout);
        let class = self.constant(Constant::TypeObject(exception_class(class)), span);
        let descriptor = self.constant(Constant::TupleDescriptor(id), span);
        let address = if kept == 0 {
            let unit = self.constant(Constant::Unit, span);
            self.bits(Self::borrowed(unit, TyirType::Unit), span)
        } else {
            self.bits(Self::borrowed(tuple.value, tuple.ty), span)
        };
        let exception = self.call_runtime(
            RuntimeFunction::ExceptionNew,
            vec![class, descriptor, address],
            span,
        );
        self.release(tuple, span);
        if let Some(program_class) = program_class {
            self.store_defaults(exception, program_class, span);
        }
        let mut values = self.packed_arguments(init.function, evaluated, span);
        values.insert(0, (0, Self::borrowed(exception.value, TyirType::EXCEPTION)));
        let done = self.call_function(init.function, values, TyirType::Unit, span);
        self.release(done, span);
        exception
    }

    /// Lowers `super().__init__(arguments)` that sets an exception's arguments (CLS-14).
    pub(crate) fn set_arguments(
        &mut self,
        exception: &TypedExpr,
        arguments: &[TypedExpr],
        span: Span,
    ) -> Lowered {
        let exception = self.expression(exception);
        let types: Vec<Type> = arguments.iter().map(|value| value.ty.clone()).collect();
        let id = self.tuple_type_id(&types);
        let layout = TyirType::Tuple(id);
        let descriptor = self.constant(Constant::TupleDescriptor(id), span);
        let tuple = self.tuple_display(arguments, layout, span);
        let address = if arguments.is_empty() {
            let unit = self.constant(Constant::Unit, span);
            self.bits(Self::borrowed(unit, TyirType::Unit), span)
        } else {
            self.bits(Self::borrowed(tuple.value, tuple.ty), span)
        };
        self.call_runtime(
            RuntimeFunction::ExceptionSetArguments,
            vec![exception.value, descriptor, address],
            span,
        );
        self.release(tuple, span);
        self.release(exception, span);
        let unit = self.constant(Constant::Unit, span);
        Self::borrowed(unit, TyirType::Unit)
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

    /// Raises the exception `exception` makes where a value of `ty` stands. No path goes on
    /// from the throw, so the value given after it is a zero value no code reads.
    pub(crate) fn raise_in_place(
        &mut self,
        exception: &TypedExpr,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let exception = self.expression(exception);
        self.call_runtime(RuntimeFunction::ExceptionThrow, vec![exception.value], span);
        self.untrack(exception.value);
        self.builder.terminate(Terminator::Unreachable);
        let after = self.builder.create_block();
        self.builder.switch_to(after);
        Self::borrowed(self.zero_value(ty, span), ty)
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

    /// Lowers `try` with its `except` clauses, its `else` block and its `finally` block.
    /// While the `try` runs, a `Finally` cleanup makes every way out run a copy of the
    /// `finally` block; falling through runs it after the rest.
    pub(crate) fn try_statement(
        &mut self,
        body: &[TypedStmt],
        handlers: &[TypedHandler],
        orelse: &[TypedStmt],
        finalbody: &[TypedStmt],
        span: Span,
    ) {
        if !finalbody.is_empty() {
            self.cleanups.push(Cleanup::Finally(Rc::from(finalbody)));
        }
        if handlers.is_empty() {
            self.statements(body);
            self.statements(orelse);
        } else {
            self.try_except(body, handlers, orelse, span);
        }
        if !finalbody.is_empty() {
            self.cleanups.pop();
            if !self.builder.is_terminated() {
                self.statements(finalbody);
            }
        }
    }

    /// Lowers the body, the `except` clauses and the `else` block of a `try`. An exception
    /// raised in the body goes to a dispatch block, which tests the clauses in order; one that
    /// no clause matches goes on as if raised after the clauses. `else` runs after the body
    /// completes, outside the body's clauses.
    fn try_except(
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
        let entered = self
            .call_runtime(RuntimeFunction::ExceptionEnter, vec![exception], span)
            .value;
        let previous = self.generator.is_none().then_some(entered);
        let handled = match clause.name {
            // A clause's variable a nested function captures lives in a cell (FUN-04).
            Some(symbol) if self.cells.contains_key(&symbol) => {
                let owned = Lowered {
                    value: exception,
                    ty: TyirType::EXCEPTION,
                    owned: true,
                };
                self.assign_cell(symbol, owned, span);
                Handled::Cell(symbol)
            }
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
            None if self.generator.is_some() => self.kept_exception(exception, span),
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
        for index in (depth..self.cleanups.len()).rev() {
            match self.cleanups[index].clone() {
                Cleanup::Handlers { .. } | Cleanup::Initialising { .. } => {}
                Cleanup::Handling {
                    previous,
                    exception,
                } => self.leave_clause(index, previous, exception, span),
                Cleanup::Finally(block) => {
                    self.run_finally(index, &block, None, span);
                }
                Cleanup::Iterator(slots) => self.release_hidden(&slots, span),
                Cleanup::With {
                    manager, protocol, ..
                } => {
                    let inner = self.cleanups.split_off(index);
                    self.call_exit(manager, protocol, None, span);
                    self.cleanups.extend(inner);
                }
            }
            if self.builder.is_terminated() {
                return;
            }
        }
    }

    /// Ends an error path: releases the live temporaries, then, innermost first, leaves each
    /// `except` clause and runs each `finally` block on the way out, until the clauses of an
    /// enclosing `try` body take the exception or the slots are released and it returns to
    /// the caller.
    pub(crate) fn propagate(&mut self, exception: ValueId, span: Span) {
        let mut exception = exception;
        self.release_live_temporaries(span);
        // The path released them, so code it runs on the way out must not release them again.
        let released = std::mem::take(&mut self.temporaries);
        for index in (0..self.cleanups.len()).rev() {
            match self.cleanups[index].clone() {
                Cleanup::Handlers { dispatch } => {
                    self.builder.terminate(Terminator::Jump {
                        target: dispatch,
                        arguments: vec![exception],
                    });
                }
                Cleanup::Initialising { state } => self.store_state(state, "0", span),
                Cleanup::Handling {
                    previous,
                    exception: handled,
                } => self.leave_clause(index, previous, handled, span),
                Cleanup::Finally(block) => {
                    if let Some(pending) = self.run_finally(index, &block, Some(exception), span) {
                        exception = pending;
                    }
                }
                Cleanup::Iterator(slots) => self.release_hidden(&slots, span),
                Cleanup::With {
                    manager,
                    protocol,
                    suppresses,
                    after,
                } => self.exit_with_exception(
                    index, manager, protocol, suppresses, after, exception, span,
                ),
            }
            if self.builder.is_terminated() {
                self.temporaries = released;
                return;
            }
        }
        self.leave_frame(exception, span);
        self.builder.terminate(Terminator::ReturnError(exception));
        // A `try` whose body cannot raise still has the path that re-raises what no clause
        // matches, so returning an exception makes the function one that raises.
        self.raises = true;
        self.temporaries = released;
    }

    /// Runs a copy of the `finally` block of cleanup `index`, with only the cleanups outside
    /// it open. On an error path, the pending exception is the one being handled while the
    /// block runs, as in CPython, and an exception the block raises releases it. Returns the
    /// pending exception the path goes on with when the block ends, which a resume function
    /// keeps in its frame while the block runs, since the block may `yield`.
    fn run_finally(
        &mut self,
        index: usize,
        block: &[TypedStmt],
        pending: Option<ValueId>,
        span: Span,
    ) -> Option<ValueId> {
        let inner = self.cleanups.split_off(index);
        let handling = pending.map(|exception| {
            let entered = self
                .call_runtime(RuntimeFunction::ExceptionEnter, vec![exception], span)
                .value;
            let (previous, handled) = if self.generator.is_some() {
                (None, self.kept_exception(exception, span))
            } else {
                (Some(entered), Handled::Value(exception))
            };
            self.cleanups.push(Cleanup::Handling {
                previous,
                exception: handled,
            });
            (previous, handled)
        });
        self.statements(block);
        let mut going_on = None;
        if let Some((previous, handled)) = handling {
            self.cleanups.pop();
            if !self.builder.is_terminated() {
                let previous = self.previous_handled(self.cleanups.len(), previous, span);
                self.call_runtime(RuntimeFunction::ExceptionLeave, vec![previous], span);
                going_on = Some(self.take_handled(handled, span));
            }
        }
        self.cleanups.extend(inner);
        going_on
    }

    /// Keeps `exception`, which the function owns, in a hidden slot of its own, where it
    /// lives across a `yield` of a resume function.
    fn kept_exception(&mut self, exception: ValueId, span: Span) -> Handled {
        let slot = self.hidden_slot("handled", TyirType::EXCEPTION);
        self.builder.push(
            Operation::Store {
                slot,
                value: exception,
            },
            span,
        );
        Handled::Slot(slot)
    }

    /// Returns the exception a clause keeps, with its reference, leaving a hidden slot that
    /// kept it null.
    fn take_handled(&mut self, handled: Handled, span: Span) -> ValueId {
        match handled {
            Handled::Slot(slot) => {
                let held =
                    self.builder
                        .push_value(Operation::Load(slot), TyirType::EXCEPTION, span);
                let none = self.constant(Constant::NoInstance(TypeId::EXCEPTION), span);
                self.builder
                    .push(Operation::Store { slot, value: none }, span);
                held
            }
            Handled::Value(value) => value,
            Handled::Cell(_) => unreachable!("a pending exception is never in a cell"),
        }
    }

    /// Returns the exception a clause keeps, borrowed.
    fn handled_exception(&mut self, handled: Handled, span: Span) -> ValueId {
        match handled {
            Handled::Slot(slot) => {
                self.builder
                    .push_value(Operation::Load(slot), TyirType::EXCEPTION, span)
            }
            Handled::Cell(symbol) => {
                let (cell, layout) = self.cell_of(symbol, span);
                let ty = self
                    .tuple_types
                    .items(layout)
                    .map_or(TyirType::EXCEPTION, |items| items[0]);
                let value = self.cell_value(cell, layout, ty, span);
                if ty == TyirType::EXCEPTION {
                    return value.value;
                }
                let bits = self.bits(value, span);
                self.builder.push_value(
                    Operation::FromBits {
                        bits,
                        ty: TyirType::EXCEPTION,
                    },
                    TyirType::EXCEPTION,
                    span,
                )
            }
            Handled::Value(value) => value,
        }
    }

    /// Returns, as bits, the exception handled before the clause of cleanup `index`: the one
    /// it kept, or in a resume function the exception of the clause around it, or the one
    /// the step's caller handles.
    fn previous_handled(&mut self, index: usize, previous: Option<ValueId>, span: Span) -> ValueId {
        if let Some(previous) = previous {
            return previous;
        }
        let around = self.cleanups[..index]
            .iter()
            .rev()
            .find_map(|cleanup| match cleanup {
                Cleanup::Handling { exception, .. } => Some(*exception),
                _ => None,
            });
        if let Some(around) = around {
            let exception = self.handled_exception(around, span);
            return self.bits(Self::borrowed(exception, TyirType::EXCEPTION), span);
        }
        let Some(generator) = self
            .generator
            .as_ref()
            .map(crate::generator::Resumable::generator)
        else {
            unreachable!("only a resume function leaves a clause without what it kept")
        };
        self.builder.push_value(
            Operation::GeneratorCaller { generator },
            TyirType::Bits,
            span,
        )
    }

    /// Makes the exception of the innermost open clause the one being handled again, as a
    /// step that resumes inside it does (FUN-07).
    pub(crate) fn handle_again(&mut self, span: Span) {
        let innermost = self
            .cleanups
            .iter()
            .rev()
            .find_map(|cleanup| match cleanup {
                Cleanup::Handling { exception, .. } => Some(*exception),
                _ => None,
            });
        if let Some(handled) = innermost {
            let exception = self.handled_exception(handled, span);
            self.call_runtime(RuntimeFunction::ExceptionEnter, vec![exception], span);
        }
    }

    /// Leaves an `except` clause, that of cleanup `index`: restores the exception handled
    /// before it and drops the clause's exception, setting its variable to null.
    fn leave_clause(
        &mut self,
        index: usize,
        previous: Option<ValueId>,
        exception: Handled,
        span: Span,
    ) {
        let previous = self.previous_handled(index, previous, span);
        self.call_runtime(RuntimeFunction::ExceptionLeave, vec![previous], span);
        match exception {
            Handled::Value(value) => {
                self.builder.push(Operation::Decrement(value), span);
            }
            Handled::Cell(symbol) => self.clear_cell(symbol, span),
            Handled::Slot(slot) => {
                let held =
                    self.builder
                        .push_value(Operation::Load(slot), TyirType::EXCEPTION, span);
                let none = self.constant(Constant::NoInstance(TypeId::EXCEPTION), span);
                self.builder
                    .push(Operation::Store { slot, value: none }, span);
                self.builder.push(Operation::Decrement(held), span);
            }
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

    /// Calls the `__next__` of an iterator class on `iterator` and returns what it gives. A
    /// `StopIteration` it raises is released and goes to `exhausted`, and any other exception
    /// takes the error path, as CPython's `for` and `next()` treat them (CST-08).
    pub(crate) fn call_next(
        &mut self,
        next: typy_check::FunctionId,
        iterator: ValueId,
        item: TyirType,
        exhausted: BlockId,
        span: Span,
    ) -> Lowered {
        let dispatch = self.methods.dispatch(next);
        let callee = match dispatch {
            Some((class, slot)) => typy_tyir::Callee::Method { class, slot },
            None => typy_tyir::Callee::Function(typy_tyir::FunctionId(next.0)),
        };
        if !self
            .may_raise
            .get(next.0 as usize)
            .copied()
            .unwrap_or(false)
        {
            let operation = match dispatch {
                Some((class, slot)) => Operation::CallMethod {
                    class,
                    slot,
                    arguments: vec![iterator],
                },
                None => Operation::Call {
                    function: typy_tyir::FunctionId(next.0),
                    arguments: vec![iterator],
                },
            };
            let value = self.builder.push_value(operation, item, span);
            return self.produced(value, item);
        }
        let normal = self.builder.create_block();
        let value = self.builder.add_block_parameter(normal, item);
        let error = self.builder.create_block();
        let exception = self.builder.add_block_parameter(error, TyirType::EXCEPTION);
        self.builder.terminate(Terminator::Invoke {
            callee,
            arguments: vec![iterator],
            normal,
            error,
            span,
        });
        self.builder.switch_to(error);
        let stop = self.constant(
            Constant::TypeObject(TypeObjectRef::Exception("StopIteration")),
            span,
        );
        let stopped = self
            .call_runtime(
                RuntimeFunction::ExceptionMatches,
                vec![exception, stop],
                span,
            )
            .value;
        let caught = self.builder.create_block();
        let other = self.builder.create_block();
        self.builder.terminate(Terminator::Branch {
            condition: stopped,
            then: caught,
            otherwise: other,
        });
        self.builder.switch_to(caught);
        self.builder.push(Operation::Decrement(exception), span);
        self.jump_open(exhausted);
        self.builder.switch_to(other);
        self.propagate(exception, span);
        self.builder.switch_to(normal);
        self.raises = true;
        self.produced(value, item)
    }

    /// Lowers `with manager as target: body`: the manager is kept in a hidden slot, its
    /// `__enter__` gives what the target binds, and every way out of the body calls its
    /// `__exit__`, falling through and `return`, `break` and `continue` with three `None`
    /// values (CST-05).
    #[allow(
        clippy::too_many_arguments,
        reason = "the parts of one `with` statement, each used once"
    )]
    pub(crate) fn with_statement(
        &mut self,
        manager: &TypedExpr,
        protocol: WithProtocol,
        suppresses: bool,
        target: Option<&AssignTarget>,
        body: &[TypedStmt],
        span: Span,
    ) {
        let lowered = self.expression(manager);
        let slot = self.hidden_slot("manager", lowered.ty);
        self.store(slot, lowered, span);
        let current = self.load(slot, span);
        let entered = match protocol {
            WithProtocol::Methods { enter, .. } => {
                let entered_type =
                    self.lowered_type(&self.program.function(enter).signature.returns);
                self.call_method(enter, vec![(0, current)], entered_type, span)
            }
            // A file enters as itself, once it is checked to be open.
            WithProtocol::TextFile => {
                self.call_runtime(RuntimeFunction::TextEnter, vec![current.value], span);
                current
            }
        };
        match target {
            Some(target) => self.assign_target(target, entered, span),
            None => self.release(entered, span),
        }
        let after = self.builder.create_block();
        self.cleanups.push(Cleanup::With {
            manager: slot,
            protocol,
            suppresses,
            after,
        });
        self.statements(body);
        let cleanup = self.cleanups.pop();
        if !self.builder.is_terminated()
            && let Some(Cleanup::With {
                manager, protocol, ..
            }) = cleanup
        {
            self.call_exit(manager, protocol, None, span);
        }
        self.jump_open(after);
        self.builder.switch_to(after);
    }

    /// Calls `__exit__` on the manager in `manager`, with the exception's class, the exception
    /// and the traceback, or three `None` values, and returns what it returns.
    fn call_exit(
        &mut self,
        manager: SlotId,
        protocol: WithProtocol,
        exception: Option<ValueId>,
        span: Span,
    ) -> Lowered {
        let exit = match protocol {
            WithProtocol::Methods { exit, .. } => exit,
            // Leaving a file's `with` closes it, which suppresses no exception.
            WithProtocol::TextFile => {
                let file = self.load(manager, span);
                return self.call_runtime(RuntimeFunction::TextClose, vec![file.value], span);
            }
        };
        let signature = self.program.function(exit).signature.clone();
        let types: Vec<TyirType> = signature.parameters[1..]
            .iter()
            .map(|parameter| self.lowered_type(&parameter.ty))
            .collect();
        let values: Vec<ValueId> = match exception {
            None => types.iter().map(|ty| self.none_of(*ty, span)).collect(),
            Some(exception) => {
                let class = self
                    .call_runtime(RuntimeFunction::ExceptionClass, vec![exception], span)
                    .value;
                let traceback = self
                    .call_runtime(RuntimeFunction::Traceback, Vec::new(), span)
                    .value;
                [class, exception, traceback]
                    .into_iter()
                    .zip(&types)
                    .map(|(value, ty)| {
                        self.builder
                            .push_value(Operation::Wrap { value, ty: *ty }, *ty, span)
                    })
                    .collect()
            }
        };
        let current = self.load(manager, span);
        let mut arguments = vec![(0, current)];
        for (index, (value, ty)) in values.into_iter().zip(types).enumerate() {
            arguments.push((index + 1, Self::borrowed(value, ty)));
        }
        let returns = self.lowered_type(&signature.returns);
        let result = self.call_method(exit, arguments, returns, span);
        if exception.is_none() {
            self.release(result, span);
            let unit = self.constant(Constant::Unit, span);
            return Self::borrowed(unit, TyirType::Unit);
        }
        result
    }

    /// Calls `__exit__` of the `with` of cleanup `index` for an exception on its way out, with
    /// the exception being handled while it runs. A true result suppresses the exception,
    /// which is released, and execution goes on after the `with`; otherwise the exception goes
    /// on. An exception `__exit__` raises takes this one as its context and releases it.
    #[allow(
        clippy::too_many_arguments,
        reason = "the parts of one `with` cleanup, each used once"
    )]
    fn exit_with_exception(
        &mut self,
        index: usize,
        manager: SlotId,
        protocol: WithProtocol,
        suppresses: bool,
        after: BlockId,
        exception: ValueId,
        span: Span,
    ) {
        let inner = self.cleanups.split_off(index);
        let previous = self
            .call_runtime(RuntimeFunction::ExceptionEnter, vec![exception], span)
            .value;
        // `__exit__` runs no `yield`, so the value lives across none.
        self.cleanups.push(Cleanup::Handling {
            previous: Some(previous),
            exception: Handled::Value(exception),
        });
        let result = self.call_exit(manager, protocol, Some(exception), span);
        self.cleanups.pop();
        self.call_runtime(RuntimeFunction::ExceptionLeave, vec![previous], span);
        if suppresses {
            let true_result = self.truth(result, span);
            self.release(result, span);
            let swallow = self.builder.create_block();
            let go_on = self.builder.create_block();
            self.builder.terminate(Terminator::Branch {
                condition: true_result,
                then: swallow,
                otherwise: go_on,
            });
            self.builder.switch_to(swallow);
            self.builder.push(Operation::Decrement(exception), span);
            self.jump_open(after);
            self.builder.switch_to(go_on);
        } else {
            self.release(result, span);
        }
        self.cleanups.extend(inner);
    }
}
