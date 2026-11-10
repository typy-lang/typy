//! Calls of functions that may raise, with the error path that releases what the function owns (ADR-0004).

use typy_diagnostics::Span;
use typy_tyir::{Callee, Operation, RuntimeFunction, Terminator, TyirType, ValueId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

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
        self.builder.terminate(Terminator::Invoke {
            callee,
            arguments,
            normal,
            error,
            span,
        });
        self.builder.switch_to(error);
        self.propagate(exception, span);
        self.builder.switch_to(normal);
        self.raises = true;
        value
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
        self.produced(value, signature.result)
    }
}
