//! Lowers calls of the functions of `math` over iterables of numbers (LIB-12): the checker
//! gives each iterable as a list. `prod` and `sumprod` whose result is `int | float` give the
//! `int` start or 0 when the first list is empty, and otherwise the `float` the runtime
//! computes.

use typy_check::{MathFunction, MathShape, Type};
use typy_diagnostics::Span;
use typy_tyir::{
    Comparison, Constant, Operation, RuntimeFunction, ScalarKind, Terminator, TyirType,
};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns the result of `math.function` of the lists `values`, with the start of `prod`
    /// after its list.
    pub(crate) fn iterable_math_call(
        &mut self,
        function: MathFunction,
        values: &[Lowered],
        result: &Type,
        span: Span,
    ) -> Lowered {
        let lists = |count: usize| values[..count].iter().map(|value| value.value).collect();
        match function.shape() {
            MathShape::Fsum => self.call_runtime(RuntimeFunction::MathFsum, lists(1), span),
            MathShape::Dist => self.call_runtime(RuntimeFunction::MathDist, lists(2), span),
            MathShape::Sumprod if *result == Type::Int => {
                self.call_runtime(RuntimeFunction::MathSumprodInt, lists(2), span)
            }
            MathShape::Sumprod => {
                let pair: Vec<_> = lists(2);
                // Each branch calls the runtime, which checks the lengths; for empty
                // iterables its `float` 0.0 gives way to the `int` 0.
                self.empty_or(
                    values[0],
                    result,
                    span,
                    |lowerer| {
                        let checked = lowerer.call_runtime(
                            RuntimeFunction::MathSumprodFloat,
                            pair.clone(),
                            span,
                        );
                        lowerer.release(checked, span);
                        let zero = lowerer.constant(Constant::Int("0".to_owned()), span);
                        Self::borrowed(zero, TyirType::Int)
                    },
                    |lowerer| {
                        lowerer.call_runtime(RuntimeFunction::MathSumprodFloat, pair.clone(), span)
                    },
                )
            }
            _ => self.product(values[0], values[1], result, span),
        }
    }

    /// Returns `prod(list, start=start)`, where a start not given is a unit value and stands
    /// for 1.
    fn product(&mut self, list: Lowered, start: Lowered, result: &Type, span: Span) -> Lowered {
        let start = if start.ty == TyirType::Unit {
            let one = self.constant(Constant::Int("1".to_owned()), span);
            Self::borrowed(one, TyirType::Int)
        } else {
            start
        };
        match result {
            Type::Int => self.call_runtime(
                RuntimeFunction::MathProdInt,
                vec![list.value, start.value],
                span,
            ),
            Type::Float => {
                let start = self.float_of(start, span);
                let call = self.call_runtime(
                    RuntimeFunction::MathProdFloat,
                    vec![list.value, start.value],
                    span,
                );
                self.release(start, span);
                call
            }
            // An `int` start with `float` items: the start itself for no items, and only
            // otherwise converted, which may raise `OverflowError`.
            _ => self.empty_or(
                list,
                result,
                span,
                |_| start,
                |lowerer| {
                    let real = lowerer.float_of(start, span);
                    let call = lowerer.call_runtime(
                        RuntimeFunction::MathProdFloat,
                        vec![list.value, real.value],
                        span,
                    );
                    lowerer.release(real, span);
                    call
                },
            ),
        }
    }

    /// Returns the `int | float` union `result`: the `int` that `when_empty` gives when
    /// `list` is empty, and otherwise the `float` that `otherwise` gives.
    fn empty_or(
        &mut self,
        list: Lowered,
        result: &Type,
        span: Span,
        when_empty: impl FnOnce(&mut Self) -> Lowered,
        otherwise: impl FnOnce(&mut Self) -> Lowered,
    ) -> Lowered {
        let ty = self.lowered_type(result);
        let length = self.call_runtime(RuntimeFunction::ListLength, vec![list.value], span);
        let zero = self.constant(Constant::Int("0".to_owned()), span);
        let empty = self.builder.push_value(
            Operation::Compare {
                kind: ScalarKind::Int,
                comparison: Comparison::Equal,
                left: length.value,
                right: zero,
            },
            TyirType::Bool,
            span,
        );
        self.release(length, span);
        let none = self.builder.create_block();
        let some = self.builder.create_block();
        let join = self.builder.create_block();
        let joined = self.builder.add_block_parameter(join, ty);
        self.builder.terminate(Terminator::Branch {
            condition: empty,
            then: none,
            otherwise: some,
        });
        for (block, member, give) in [
            (
                none,
                Type::Int,
                Box::new(when_empty) as Box<dyn FnOnce(&mut Self) -> Lowered>,
            ),
            (some, Type::Float, Box::new(otherwise)),
        ] {
            self.builder.switch_to(block);
            let value = give(self);
            // The union takes a reference of its own to the value.
            let widened = self.widened_to_union(value, &member, result, ty, span);
            let given = self.take_reference(widened, span);
            self.builder.terminate(Terminator::Jump {
                target: join,
                arguments: vec![given],
            });
        }
        self.builder.switch_to(join);
        self.produced(joined, ty)
    }
}
