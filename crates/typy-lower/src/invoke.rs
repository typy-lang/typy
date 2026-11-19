//! Calls of functions that may raise, with the error path that releases what the function owns (ADR-0004).

use typy_diagnostics::Span;
use typy_tyir::{Callee, Operation, RuntimeFunction, Terminator, TyirType, ValueId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

/// Whether runtime calls are followed by the test of the exception a slot's method left with
/// the runtime (CLS-22, ADR-0039).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SlotTests {
    /// No method a type object's slot calls may raise.
    Never,
    /// A slot's method may raise, so each runtime call that may run a slot is tested.
    After,
    /// The lowering is writing a test, whose error path runs no test of its own, since it may
    /// call the same function.
    Writing,
}

impl SlotTests {
    /// Returns the tests a function gets when a slot's method may raise, or none.
    pub(crate) const fn of(raising_slots: bool) -> Self {
        if raising_slots {
            Self::After
        } else {
            Self::Never
        }
    }
}

impl FunctionLowerer<'_> {
    /// Calls a function that may raise, and returns its result. The error path releases the live temporaries and goes on as `propagate` says.
    pub(crate) fn invoke(
        &mut self,
        callee: Callee,
        arguments: Vec<ValueId>,
        result: TyirType,
        span: Span,
    ) -> ValueId {
        let normal = self.builder.create_block();
        let value = self.builder.add_block_parameter(normal, result);
        let error = self.builder.create_block();
        let exception = self.builder.add_block_parameter(error, TyirType::EXCEPTION);
        let kept = self.kept_arguments(callee, &arguments);
        self.builder.terminate(Terminator::Invoke {
            callee,
            arguments,
            normal,
            error,
            span,
        });
        self.builder.switch_to(error);
        if kept.is_empty() {
            self.propagate(exception, span);
        } else {
            // The arguments the function owns were the callee's variables, which CPython's
            // traceback keeps, so they go with the exception on this path (ADR-0070).
            for argument in &kept {
                self.keep_with(exception, *argument, span);
            }
            let owned = self.temporaries.clone();
            self.temporaries.retain(|each| !kept.contains(each));
            self.propagate(exception, span);
            self.temporaries = owned;
        }
        self.builder.switch_to(normal);
        self.raises = true;
        value
    }

    /// Returns the arguments of a call of the program's code that the function owns and that
    /// may reach a generator, which the call's error path keeps with the exception, in a
    /// program with generators (ADR-0070).
    fn kept_arguments(&self, callee: Callee, arguments: &[ValueId]) -> Vec<ValueId> {
        if self.unwinding == crate::ownership::Unwinding::Releases
            || matches!(callee, Callee::Runtime(_))
        {
            return Vec::new();
        }
        arguments
            .iter()
            .copied()
            .filter(|argument| {
                self.temporaries.contains(argument)
                    && crate::ownership::kept_by_traceback(self.builder.value_type(*argument))
            })
            .collect()
    }

    /// Calls a runtime function, with `invoke` when it may raise, and returns its result as an owned value.
    pub(crate) fn call_runtime(
        &mut self,
        function: RuntimeFunction,
        arguments: Vec<ValueId>,
        span: Span,
    ) -> Lowered {
        let signature = function.signature();
        let value = if signature.may_raise {
            self.invoke(Callee::Runtime(function), arguments, signature.result, span)
        } else if signature.result == TyirType::Unit {
            self.builder.push(
                Operation::CallRuntime {
                    function,
                    arguments,
                },
                span,
            );
            self.constant(typy_tyir::Constant::Unit, span)
        } else {
            self.builder.push_value(
                Operation::CallRuntime {
                    function,
                    arguments,
                },
                signature.result,
                span,
            )
        };
        let lowered = self.produced(value, signature.result);
        // A method a slot called may have raised inside the runtime (CLS-22, ADR-0039).
        if self.slot_tests == SlotTests::After && function.may_call_slots() {
            self.slot_tests = SlotTests::Writing;
            self.invoke(
                Callee::Runtime(RuntimeFunction::SlotRaised),
                Vec::new(),
                TyirType::Unit,
                span,
            );
            self.slot_tests = SlotTests::After;
        }
        lowered
    }
}
