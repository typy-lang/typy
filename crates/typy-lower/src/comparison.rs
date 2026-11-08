//! Lowers comparisons and comparison chains, which evaluate each operand once (EXP-06, EXP-08).

use typy_check::TypedExpr;
use typy_diagnostics::Span;
use typy_source::syntax::CompareOperator;
use typy_tyir::{
    Comparison, Constant, Operation, RuntimeFunction, ScalarKind, Terminator, TyirType, ValueId,
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
                let both_none = left.ty == TyirType::Unit && right.ty == TyirType::Unit;
                return self.constant(
                    Constant::Bool(both_none == (operator == CompareOperator::Is)),
                    span,
                );
            }
            CompareOperator::In | CompareOperator::NotIn => {
                let found = self.list_contains(left, right, span);
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
            (TyirType::Object(typy_tyir::TypeId::LIST), _) => {
                return self.list_equality(comparison, left, right, span);
            }
            (TyirType::Bits, _) => {
                unreachable!("list items are converted before they are compared")
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
