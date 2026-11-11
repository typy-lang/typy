//! Lowers comparisons and comparison chains, which evaluate each operand once (EXP-06, EXP-08).

use typy_check::TypedExpr;
use typy_diagnostics::Span;
use typy_source::syntax::CompareOperator;
use typy_tyir::{
    Comparison, Constant, Operation, OptionalBase, RuntimeFunction, ScalarKind, Terminator,
    TyirType, ValueId,
};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// `a < b <= c`: each operand is evaluated once, and evaluation stops at the first false link.
    pub(crate) fn comparison_chain(
        &mut self,
        first: &TypedExpr,
        operators: &[CompareOperator],
        comparators: &[TypedExpr],
        span: Span,
    ) -> Lowered {
        let join = self.builder.create_block();
        let result = self.builder.add_block_parameter(join, TyirType::Bool);
        let mut left = self.expression(first);
        for (position, (operator, comparator)) in operators.iter().zip(comparators).enumerate() {
            let right = self.expression(comparator);
            let outcome = self.compare(*operator, left, right, span);
            self.release(left, span);
            if position + 1 == operators.len() {
                self.release(right, span);
                self.builder.terminate(Terminator::Jump {
                    target: join,
                    arguments: vec![outcome],
                });
                break;
            }
            let next = self.builder.create_block();
            let failed = self.builder.create_block();
            self.builder.terminate(Terminator::Branch {
                condition: outcome,
                then: next,
                otherwise: failed,
            });
            self.builder.switch_to(failed);
            if right.owned {
                self.builder.push(Operation::Decrement(right.value), span);
            }
            let falsity = self.constant(Constant::Bool(false), span);
            self.builder.terminate(Terminator::Jump {
                target: join,
                arguments: vec![falsity],
            });
            self.builder.switch_to(next);
            left = right;
        }
        self.builder.switch_to(join);
        Self::borrowed(result, TyirType::Bool)
    }

    /// Returns `item in container` for a list, a set, a `str` or a tuple; the operands are not
    /// released (EXP-07).
    fn contains(&mut self, item: Lowered, container: Lowered, span: Span) -> ValueId {
        match container.ty {
            TyirType::Tuple(_) => self.tuple_contains(item, container, span),
            TyirType::LIST => self.list_contains(item, container, span),
            TyirType::DICT | TyirType::VIEW => self.dict_contains(item, container, span),
            TyirType::RANGE => {
                self.call_runtime(
                    RuntimeFunction::RangeContains,
                    vec![container.value, item.value],
                    span,
                )
                .value
            }
            TyirType::SET => {
                let bits = self.bits(item, span);
                self.call_runtime(
                    RuntimeFunction::SetContains,
                    vec![container.value, bits],
                    span,
                )
                .value
            }
            TyirType::BYTES => {
                let function = if item.ty == TyirType::Int {
                    RuntimeFunction::BytesContainsByte
                } else {
                    RuntimeFunction::BytesContains
                };
                self.call_runtime(function, vec![container.value, item.value], span)
                    .value
            }
            _ => {
                self.call_runtime(
                    RuntimeFunction::StrContains,
                    vec![container.value, item.value],
                    span,
                )
                .value
            }
        }
    }

    /// Returns whether a tuple has an item equal to `item`. Each item is compared in order, as
    /// CPython compares them, and the first equal one ends the search.
    fn tuple_contains(&mut self, item: Lowered, tuple: Lowered, span: Span) -> ValueId {
        let TyirType::Tuple(id) = tuple.ty else {
            unreachable!("the container is a tuple")
        };
        let items = self.tuple_types.items(id).unwrap_or_default().to_vec();
        let join = self.builder.create_block();
        let found = self.builder.add_block_parameter(join, TyirType::Bool);
        for (index, ty) in items.into_iter().enumerate() {
            let candidate = self.tuple_item(Self::borrowed(tuple.value, tuple.ty), index, ty, span);
            let equal = self.compare(CompareOperator::Eq, item, candidate, span);
            let next = self.builder.create_block();
            let yes = self.constant(Constant::Bool(true), span);
            let hit = self.builder.create_block();
            self.builder.terminate(Terminator::Branch {
                condition: equal,
                then: hit,
                otherwise: next,
            });
            self.builder.switch_to(hit);
            self.builder.terminate(Terminator::Jump {
                target: join,
                arguments: vec![yes],
            });
            self.builder.switch_to(next);
        }
        let no = self.constant(Constant::Bool(false), span);
        self.builder.terminate(Terminator::Jump {
            target: join,
            arguments: vec![no],
        });
        self.builder.switch_to(join);
        found
    }

    /// Returns `left is right`, or `left is not right` when `is` is false.
    fn identity(&mut self, is: bool, left: Lowered, right: Lowered, span: Span) -> ValueId {
        // Two objects are compared by address, and `None` of an optional object is
        // the null address. Otherwise an optional value is `None` when its spare
        // value says so; any other value is `None` only when its type is (ADR-0011).
        let holds = match (left.ty, right.ty) {
            (
                TyirType::Object(_) | TyirType::Optional(OptionalBase::Object(_)),
                TyirType::Object(_) | TyirType::Optional(OptionalBase::Object(_)),
            ) => self.identical(left, right, span),
            (TyirType::Optional(_), _) => self.is_none(left, span),
            (_, TyirType::Optional(_)) => self.is_none(right, span),
            (left, right) => self.constant(
                Constant::Bool(left == TyirType::Unit && right == TyirType::Unit),
                span,
            ),
        };
        if is {
            return holds;
        }
        self.builder
            .push_value(Operation::BoolNot(holds), TyirType::Bool, span)
    }

    /// Compares two values whose types the checker accepted; the operands are not released.
    pub(crate) fn compare(
        &mut self,
        operator: CompareOperator,
        left: Lowered,
        right: Lowered,
        span: Span,
    ) -> ValueId {
        let comparison = match operator {
            CompareOperator::Eq => Comparison::Equal,
            CompareOperator::NotEq => Comparison::NotEqual,
            CompareOperator::Lt => Comparison::Less,
            CompareOperator::LtE => Comparison::LessEqual,
            CompareOperator::Gt => Comparison::Greater,
            CompareOperator::GtE => Comparison::GreaterEqual,
            CompareOperator::Is | CompareOperator::IsNot => {
                return self.identity(operator == CompareOperator::Is, left, right, span);
            }
            CompareOperator::In | CompareOperator::NotIn => {
                let found = self.contains(left, right, span);
                if operator == CompareOperator::In {
                    return found;
                }
                return self
                    .builder
                    .push_value(Operation::BoolNot(found), TyirType::Bool, span);
            }
        };
        let kind = match (left.ty, right.ty) {
            (TyirType::Tuple(_), _) => return self.compare_tuples(operator, left, right, span),
            // An instance of a class without `__eq__` is equal only to itself (CLS-07).
            (TyirType::Object(id), _)
                if id.class_index().is_some() || id == typy_tyir::TypeId::TYPE =>
            {
                return self.instance_equality(comparison == Comparison::Equal, left, right, span);
            }
            (TyirType::LIST, _) => {
                if matches!(comparison, Comparison::Equal | Comparison::NotEqual) {
                    return self.list_equality(comparison, left, right, span);
                }
                let code = self.constant(Constant::Int(comparison.code().to_string()), span);
                return self
                    .call_runtime(
                        RuntimeFunction::ListCompare,
                        vec![left.value, right.value, code],
                        span,
                    )
                    .value;
            }
            (TyirType::DICT | TyirType::SET, _) => {
                return self.compare_collections(comparison, left, right, span);
            }
            (TyirType::RANGE, _) => return self.compare_ranges(comparison, left, right, span),
            (TyirType::Bits, _) => {
                unreachable!("list items are converted before they are compared")
            }
            (TyirType::Optional(_), _) => {
                unreachable!("the checker compares an optional value only with `is`")
            }
            (TyirType::Unit, TyirType::Unit) => {
                return self.constant(Constant::Bool(comparison == Comparison::Equal), span);
            }
            (TyirType::Int, TyirType::Float) => {
                return self.compare_int_float(left.value, right.value, comparison, span);
            }
            (TyirType::Float, TyirType::Int) => {
                return self.compare_int_float(right.value, left.value, mirrored(comparison), span);
            }
            (TyirType::Int, _) => ScalarKind::Int,
            (TyirType::Float, _) => ScalarKind::Float,
            (TyirType::Bool, _) => ScalarKind::Bool,
            (TyirType::BYTES, _) => {
                let code = self.constant(Constant::Int(comparison.code().to_string()), span);
                return self
                    .call_runtime(
                        RuntimeFunction::BytesCompare,
                        vec![left.value, right.value, code],
                        span,
                    )
                    .value;
            }
            (TyirType::Object(_), _) => ScalarKind::Str,
            (TyirType::Unit, _) => {
                unreachable!("the checker refuses comparing None with other types")
            }
        };
        self.builder.push_value(
            Operation::Compare {
                kind,
                comparison,
                left: left.value,
                right: right.value,
            },
            TyirType::Bool,
            span,
        )
    }

    /// Compares two ranges with `==` or `!=`, by the values they hold (DAT-17).
    fn compare_ranges(
        &mut self,
        comparison: Comparison,
        left: Lowered,
        right: Lowered,
        span: Span,
    ) -> ValueId {
        let equal = self
            .call_runtime(
                RuntimeFunction::RangeEqual,
                vec![left.value, right.value],
                span,
            )
            .value;
        if comparison == Comparison::Equal {
            equal
        } else {
            self.builder
                .push_value(Operation::BoolNot(equal), TyirType::Bool, span)
        }
    }

    /// Compares two dicts for equality, or two sets, which order as subsets and supersets;
    /// the runtime tests both.
    fn compare_collections(
        &mut self,
        comparison: Comparison,
        left: Lowered,
        right: Lowered,
        span: Span,
    ) -> ValueId {
        if left.ty == TyirType::SET {
            let code = self.constant(Constant::Int(comparison.code().to_string()), span);
            return self
                .call_runtime(
                    RuntimeFunction::SetCompare,
                    vec![left.value, right.value, code],
                    span,
                )
                .value;
        }
        let equal = self
            .call_runtime(
                RuntimeFunction::DictEqual,
                vec![left.value, right.value],
                span,
            )
            .value;
        if comparison == Comparison::Equal {
            return equal;
        }
        self.builder
            .push_value(Operation::BoolNot(equal), TyirType::Bool, span)
    }

    /// CPython compares an `int` with a `float` exactly, without converting the `int` first.
    fn compare_int_float(
        &mut self,
        int: ValueId,
        float: ValueId,
        comparison: Comparison,
        span: Span,
    ) -> ValueId {
        let code = self.constant(Constant::Int(comparison.code().to_string()), span);
        self.builder.push_value(
            Operation::CallRuntime {
                function: RuntimeFunction::CompareIntFloat,
                arguments: vec![int, float, code],
            },
            TyirType::Bool,
            span,
        )
    }
}

/// Returns the comparison with its operands swapped: `a < b` is `b > a`.
fn mirrored(comparison: Comparison) -> Comparison {
    match comparison {
        Comparison::Less => Comparison::Greater,
        Comparison::LessEqual => Comparison::GreaterEqual,
        Comparison::Greater => Comparison::Less,
        Comparison::GreaterEqual => Comparison::LessEqual,
        same => same,
    }
}

#[cfg(test)]
mod tests {
    use super::mirrored;
    use typy_tyir::Comparison;

    #[test]
    fn mirroring_swaps_the_direction_and_keeps_equality() {
        assert_eq!(mirrored(Comparison::Less), Comparison::Greater);
        assert_eq!(mirrored(Comparison::GreaterEqual), Comparison::LessEqual);
        assert_eq!(mirrored(Comparison::Equal), Comparison::Equal);
        assert_eq!(mirrored(Comparison::NotEqual), Comparison::NotEqual);
    }
}
