//! Lowers calls of the functions of `math` over iterables of numbers (LIB-12, LIB-15): the
//! checker gives each iterable as a list, of numbers or of a union of number types. `prod`
//! and `sumprod` whose result is `int | float` give an `int` when no item is a `float`, and
//! otherwise the `float` the runtime computes; `prod` with a `bool` start gives the start for
//! no items.

use typy_check::{MathFunction, MathShape, Type};
use typy_diagnostics::Span;
use typy_tyir::{
    Comparison, Constant, Operation, RuntimeFunction, ScalarKind, Terminator, TyirType, ValueId,
};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

/// Gives one arm of a choice its value.
type Arm<'arm, 'lower> = Box<dyn FnOnce(&mut FunctionLowerer<'lower>) -> Lowered + 'arm>;

impl<'lower> FunctionLowerer<'lower> {
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
                let pair: Vec<ValueId> = lists(2);
                // Each arm calls the runtime, which checks the lengths.
                let real = self.has_float(&pair, span);
                self.choose(
                    real,
                    result,
                    span,
                    (
                        Type::Float,
                        Box::new(|lowerer: &mut FunctionLowerer<'lower>| {
                            lowerer.call_runtime(
                                RuntimeFunction::MathSumprodFloat,
                                pair.clone(),
                                span,
                            )
                        }),
                    ),
                    (
                        Type::Int,
                        Box::new(|lowerer: &mut FunctionLowerer<'lower>| {
                            lowerer.call_runtime(
                                RuntimeFunction::MathSumprodInt,
                                pair.clone(),
                                span,
                            )
                        }),
                    ),
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
        match start.ty {
            TyirType::Float => self.call_runtime(
                RuntimeFunction::MathProdFloat,
                vec![list.value, start.value],
                span,
            ),
            TyirType::Bool => self.bool_start_product(list, start, result, span),
            _ => self.whole_start_product(list, start, result, span),
        }
    }

    /// Returns `prod` of an `int` start: an `int` when no item is a `float`, and otherwise the
    /// exact product up to the first `float`, then a `float` one.
    fn whole_start_product(
        &mut self,
        list: Lowered,
        start: Lowered,
        result: &Type,
        span: Span,
    ) -> Lowered {
        let whole = |lowerer: &mut FunctionLowerer<'lower>| {
            lowerer.call_runtime(
                RuntimeFunction::MathProdInt,
                vec![list.value, start.value],
                span,
            )
        };
        if *result == Type::Int {
            return whole(self);
        }
        let real = self.has_float(&[list.value], span);
        self.choose(
            real,
            result,
            span,
            (
                Type::Float,
                Box::new(move |lowerer: &mut FunctionLowerer<'lower>| {
                    lowerer.call_runtime(
                        RuntimeFunction::MathProdMixed,
                        vec![list.value, start.value],
                        span,
                    )
                }),
            ),
            (Type::Int, Box::new(whole)),
        )
    }

    /// Returns `prod` of a `bool` start: the start itself for no items, and otherwise the
    /// product of the start as an `int`, as CPython multiplies `True` by the first item.
    fn bool_start_product(
        &mut self,
        list: Lowered,
        start: Lowered,
        result: &Type,
        span: Span,
    ) -> Lowered {
        let rest = Type::union(
            result
                .members()
                .iter()
                .filter(|member| **member != Type::Bool)
                .cloned()
                .collect(),
        );
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
        let product_type = rest.clone();
        self.choose(
            empty,
            result,
            span,
            (
                Type::Bool,
                Box::new(move |_: &mut FunctionLowerer<'lower>| start),
            ),
            (
                rest,
                Box::new(move |lowerer: &mut FunctionLowerer<'lower>| {
                    let whole = lowerer.int_of(start, span);
                    lowerer.whole_start_product(list, whole, &product_type, span)
                }),
            ),
        )
    }

    /// Returns whether any of the lists holds a `float`.
    fn has_float(&mut self, lists: &[ValueId], span: Span) -> ValueId {
        let mut found: Option<ValueId> = None;
        for list in lists {
            let real = self
                .call_runtime(RuntimeFunction::MathHasFloat, vec![*list], span)
                .value;
            found = Some(match found {
                Some(before) => self.builder.push_value(
                    Operation::BoolBinary {
                        operator: typy_tyir::BoolOperator::Or,
                        left: before,
                        right: real,
                    },
                    TyirType::Bool,
                    span,
                ),
                None => real,
            });
        }
        found.unwrap_or_else(|| self.constant(Constant::Bool(false), span))
    }

    /// Returns the union `result`: what `first` gives, a value of its member type, when
    /// `condition` holds, and otherwise what `second` gives.
    fn choose(
        &mut self,
        condition: ValueId,
        result: &Type,
        span: Span,
        first: (Type, Arm<'_, 'lower>),
        second: (Type, Arm<'_, 'lower>),
    ) -> Lowered {
        let ty = self.lowered_type(result);
        let then = self.builder.create_block();
        let otherwise = self.builder.create_block();
        let join = self.builder.create_block();
        let joined = self.builder.add_block_parameter(join, ty);
        self.builder.terminate(Terminator::Branch {
            condition,
            then,
            otherwise,
        });
        for (block, (member, give)) in [(then, first), (otherwise, second)] {
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
