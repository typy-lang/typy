//! Lowers arithmetic and bitwise operators with Python's results (EXP-02 to EXP-05).

use typy_check::Type;
use typy_diagnostics::Span;
use typy_source::syntax::{BinaryOperator, UnaryOperator};
use typy_tyir::{
    BoolOperator, FloatOperator, IntOperator, IntUnaryOperator, Operation, RuntimeFunction,
    TyirType, ValueId,
};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Applies a binary operator; the operands are released afterwards. The checker has refused every other combination.
    pub(crate) fn binary(
        &mut self,
        operator: BinaryOperator,
        left: Lowered,
        right: Lowered,
        result: &Type,
        span: Span,
    ) -> Lowered {
        use BinaryOperator as Op;
        let lowered = match (left.ty, right.ty) {
            (TyirType::Tuple(_), TyirType::Tuple(_)) => {
                let ty = self.lowered_type(result);
                return self.tuple_concat(left, right, ty, span);
            }
            (TyirType::Int, TyirType::Int) if *result == Type::Int => {
                self.int_binary(operator, left.value, right.value, span)
            }
            (TyirType::Int, TyirType::Int) if operator == Op::Div => self.call_runtime(
                RuntimeFunction::IntTrueDivide,
                vec![left.value, right.value],
                span,
            ),
            (TyirType::Bool, TyirType::Bool) => {
                let operator = match operator {
                    Op::BitAnd => BoolOperator::And,
                    Op::BitOr => BoolOperator::Or,
                    _ => BoolOperator::Xor,
                };
                let value = self.builder.push_value(
                    Operation::BoolBinary {
                        operator,
                        left: left.value,
                        right: right.value,
                    },
                    TyirType::Bool,
                    span,
                );
                Self::borrowed(value, TyirType::Bool)
            }
            (TyirType::Object(_), TyirType::Object(_)) => self.call_runtime(
                RuntimeFunction::StrConcat,
                vec![left.value, right.value],
                span,
            ),
            (TyirType::Object(_), TyirType::Int) => self.call_runtime(
                RuntimeFunction::StrRepeat,
                vec![left.value, right.value],
                span,
            ),
            (TyirType::Int, TyirType::Object(_)) => self.call_runtime(
                RuntimeFunction::StrRepeat,
                vec![right.value, left.value],
                span,
            ),
            _ => {
                let left_float = self.float_operand(left, span);
                let right_float = self.float_operand(right, span);
                self.float_binary(operator, left_float, right_float, span)
            }
        };
        self.release(right, span);
        self.release(left, span);
        lowered
    }

    fn int_binary(
        &mut self,
        operator: BinaryOperator,
        left: ValueId,
        right: ValueId,
        span: Span,
    ) -> Lowered {
        use BinaryOperator as Op;
        let inline = match operator {
            Op::Add => IntOperator::Add,
            Op::Sub => IntOperator::Subtract,
            Op::Mult => IntOperator::Multiply,
            Op::BitAnd => IntOperator::BitAnd,
            Op::BitOr => IntOperator::BitOr,
            Op::BitXor => IntOperator::BitXor,
            Op::FloorDiv | Op::Mod | Op::Pow | Op::LShift | Op::RShift => {
                let function = match operator {
                    Op::FloorDiv => RuntimeFunction::IntFloorDivide,
                    Op::Mod => RuntimeFunction::IntModulo,
                    Op::Pow => RuntimeFunction::IntPower,
                    Op::LShift => RuntimeFunction::IntLeftShift,
                    _ => RuntimeFunction::IntRightShift,
                };
                return self.call_runtime(function, vec![left, right], span);
            }
            Op::Div | Op::MatMult => {
                unreachable!("the checker gives `/` a float result and refuses `@`")
            }
        };
        let value = self.builder.push_value(
            Operation::IntBinary {
                operator: inline,
                left,
                right,
            },
            TyirType::Int,
            span,
        );
        self.produced(value, TyirType::Int)
    }

    fn float_binary(
        &mut self,
        operator: BinaryOperator,
        left: ValueId,
        right: ValueId,
        span: Span,
    ) -> Lowered {
        use BinaryOperator as Op;
        let inline = match operator {
            Op::Add => FloatOperator::Add,
            Op::Sub => FloatOperator::Subtract,
            Op::Mult => FloatOperator::Multiply,
            _ => {
                let function = match operator {
                    Op::Div => RuntimeFunction::FloatDivide,
                    Op::FloorDiv => RuntimeFunction::FloatFloorDivide,
                    Op::Mod => RuntimeFunction::FloatModulo,
                    _ => RuntimeFunction::FloatPower,
                };
                return self.call_runtime(function, vec![left, right], span);
            }
        };
        let value = self.builder.push_value(
            Operation::FloatBinary {
                operator: inline,
                left,
                right,
            },
            TyirType::Float,
            span,
        );
        Self::borrowed(value, TyirType::Float)
    }

    /// Returns a float operand: an `int` is converted, which raises `OverflowError` when it is too large.
    pub(crate) fn float_operand(&mut self, lowered: Lowered, span: Span) -> ValueId {
        if lowered.ty == TyirType::Int {
            self.call_runtime(RuntimeFunction::IntToFloat, vec![lowered.value], span)
                .value
        } else {
            lowered.value
        }
    }

    /// Applies a unary operator; the operand is released afterwards, unless it is the result.
    pub(crate) fn unary(
        &mut self,
        operator: UnaryOperator,
        operand: Lowered,
        span: Span,
    ) -> Lowered {
        let result = match (operator, operand.ty) {
            (UnaryOperator::UAdd, _) => return operand,
            (UnaryOperator::Not, _) => {
                let truth = self.truth(operand, span);
                let value =
                    self.builder
                        .push_value(Operation::BoolNot(truth), TyirType::Bool, span);
                Self::borrowed(value, TyirType::Bool)
            }
            (UnaryOperator::USub, TyirType::Float) => {
                let value = self.builder.push_value(
                    Operation::FloatNegate(operand.value),
                    TyirType::Float,
                    span,
                );
                Self::borrowed(value, TyirType::Float)
            }
            (UnaryOperator::USub | UnaryOperator::Invert, _) => {
                let operator = if operator == UnaryOperator::USub {
                    IntUnaryOperator::Negate
                } else {
                    IntUnaryOperator::Invert
                };
                let value = self.builder.push_value(
                    Operation::IntUnary {
                        operator,
                        operand: operand.value,
                    },
                    TyirType::Int,
                    span,
                );
                self.produced(value, TyirType::Int)
            }
        };
        self.release(operand, span);
        result
    }
}
