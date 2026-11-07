//! Maps Ruff's operators to TyPy's.

use ruff_python_ast as ast;

use crate::syntax::{BinaryOperator, CompareOperator};

pub(super) fn binary_operator(operator: ast::Operator) -> BinaryOperator {
    match operator {
        ast::Operator::Add => BinaryOperator::Add,
        ast::Operator::Sub => BinaryOperator::Sub,
        ast::Operator::Mult => BinaryOperator::Mult,
        ast::Operator::MatMult => BinaryOperator::MatMult,
        ast::Operator::Div => BinaryOperator::Div,
        ast::Operator::Mod => BinaryOperator::Mod,
        ast::Operator::Pow => BinaryOperator::Pow,
        ast::Operator::LShift => BinaryOperator::LShift,
        ast::Operator::RShift => BinaryOperator::RShift,
        ast::Operator::BitOr => BinaryOperator::BitOr,
        ast::Operator::BitXor => BinaryOperator::BitXor,
        ast::Operator::BitAnd => BinaryOperator::BitAnd,
        ast::Operator::FloorDiv => BinaryOperator::FloorDiv,
    }
}

pub(super) fn compare_operator(operator: ast::CmpOp) -> CompareOperator {
    match operator {
        ast::CmpOp::Eq => CompareOperator::Eq,
        ast::CmpOp::NotEq => CompareOperator::NotEq,
        ast::CmpOp::Lt => CompareOperator::Lt,
        ast::CmpOp::LtE => CompareOperator::LtE,
        ast::CmpOp::Gt => CompareOperator::Gt,
        ast::CmpOp::GtE => CompareOperator::GtE,
        ast::CmpOp::Is => CompareOperator::Is,
        ast::CmpOp::IsNot => CompareOperator::IsNot,
        ast::CmpOp::In => CompareOperator::In,
        ast::CmpOp::NotIn => CompareOperator::NotIn,
    }
}
