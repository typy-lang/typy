//! Lowers the methods of a generator, `send`, `throw` and `close` (FUN-07, FUN-11,
//! ADR-0063). Each steps the generator through the runtime, which raises `StopIteration`
//! when its body returns.

use typy_check::Method;
use typy_diagnostics::Span;
use typy_tyir::{Constant, Operation, RuntimeFunction, TyirType, ValueId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Calls `method` of `generator` with the values of its arguments; `send` and `throw`
    /// return the item it yields next, of type `result`, with a reference of its own.
    pub(crate) fn generator_method(
        &mut self,
        generator: ValueId,
        method: Method,
        arguments: &[Lowered],
        result: TyirType,
        span: Span,
    ) -> Lowered {
        match (method, arguments) {
            (Method::GeneratorSend, [value]) => {
                // The value stays borrowed: the caller holds it while the step runs.
                let bits = self.bits(*value, span);
                let none = self.sends_none(*value, span);
                self.call_runtime(
                    RuntimeFunction::GeneratorSend,
                    vec![generator, bits, none],
                    span,
                );
                self.yielded(generator, result, span)
            }
            (Method::GeneratorThrow, [exception]) => {
                // The generator takes over a reference to the exception it raises.
                if exception.owned {
                    self.untrack(exception.value);
                    self.handed_over.insert(exception.value);
                } else {
                    self.builder
                        .push(Operation::Increment(exception.value), span);
                }
                self.call_runtime(
                    RuntimeFunction::GeneratorThrow,
                    vec![generator, exception.value],
                    span,
                );
                self.yielded(generator, result, span)
            }
            (Method::GeneratorClose, []) => {
                self.call_runtime(RuntimeFunction::GeneratorClose, vec![generator], span)
            }
            _ => unreachable!("the checker gives a generator only its own methods"),
        }
    }

    /// Returns whether `value`, which `send` passes, is `None`, which a generator that has
    /// not started accepts alone.
    fn sends_none(&mut self, value: Lowered, span: Span) -> ValueId {
        match value.ty {
            TyirType::Optional(_) => self.is_none(value, span),
            TyirType::Unit => self.constant(Constant::Bool(true), span),
            _ => self.constant(Constant::Bool(false), span),
        }
    }

    /// Returns the item the generator's last step yielded, with a reference of its own.
    fn yielded(&mut self, generator: ValueId, item: TyirType, span: Span) -> Lowered {
        let iterator = self.builder.push_value(
            Operation::Upcast {
                value: generator,
                ty: TyirType::ITERATOR,
            },
            TyirType::ITERATOR,
            span,
        );
        self.current_item(iterator, item, span)
    }
}
