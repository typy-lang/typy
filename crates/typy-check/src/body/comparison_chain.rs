//! Chains of comparisons of instances whose classes define comparison methods, such as
//! `a < b < c` (CLS-22): each comparison is checked alone, so it may call a method, and the
//! comparisons are joined with `and`. An operand between two comparisons is evaluated once,
//! held in a hidden variable that the next comparison reads, as CPython evaluates it once.

use typy_diagnostics::Span;
use typy_source::syntax::{BoolOperator, CompareOperator};

use super::BodyChecker;
use super::operator_method::Comparison;
use crate::typed::{TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns `left op1 c1 op2 c2 ...` as `left op1 c1 and c1 op2 c2 and ...`, each middle
    /// operand evaluated once.
    pub(super) fn instance_chain(
        &mut self,
        left: TypedExpr,
        operators: &[CompareOperator],
        comparators: Vec<TypedExpr>,
        span: Span,
    ) -> TypedExpr {
        let last = comparators.len().saturating_sub(1);
        let mut values = Vec::new();
        let mut previous = left;
        for (index, (operator, right)) in operators.iter().zip(comparators).enumerate() {
            let (right, next) = if index < last {
                self.held_operand(right)
            } else {
                (right, None)
            };
            values.push(self.single_comparison(*operator, previous, right, span));
            match next {
                Some(next) => previous = next,
                None => break,
            }
        }
        Self::typed(
            TypedExprKind::BoolOp {
                operator: BoolOperator::And,
                values,
            },
            Type::Bool,
            span,
        )
    }

    /// Returns `operand` stored in a new hidden variable as it is given, and the read of the
    /// variable.
    fn held_operand(&mut self, operand: TypedExpr) -> (TypedExpr, Option<TypedExpr>) {
        let (ty, span) = (operand.ty.clone(), operand.span);
        let symbol = self.hidden_variable("chained", ty.clone(), span);
        let read = Self::typed(TypedExprKind::Local(symbol), ty.clone(), span);
        let stored = Self::typed(
            TypedExprKind::Named {
                target: symbol,
                value: Box::new(operand),
            },
            ty,
            span,
        );
        (stored, Some(read))
    }

    /// Returns one comparison: a call of a comparison method, or the built-in comparison.
    pub(super) fn single_comparison(
        &mut self,
        operator: CompareOperator,
        left: TypedExpr,
        right: TypedExpr,
        span: Span,
    ) -> TypedExpr {
        match self.operator_comparison(operator, left, right, span) {
            Comparison::Called(call) => call,
            Comparison::Builtin(left, right) => {
                self.compare_types(operator, &left, &right);
                Self::typed(
                    TypedExprKind::Compare {
                        left: Box::new(left),
                        operators: Box::new([operator]),
                        comparators: vec![right],
                        narrowing: None,
                    },
                    Type::Bool,
                    span,
                )
            }
        }
    }
}
