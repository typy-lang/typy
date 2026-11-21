//! A named tuple read as the tuple of its fields where a tuple operation applies (CLS-27,
//! ADR-0088): an index, a slice, `len`, a loop, unpacking, `*` in a call, a sequence pattern,
//! `in`, `+` and a comparison with a tuple. The hidden method written for its class returns
//! the tuple, so the value is evaluated once.

use typy_source::syntax::{BinaryOperator, CompareOperator, Expr};

use super::BodyChecker;
use crate::class_type::NAMED_TUPLE_FIELDS;
use crate::typed::{Argument, Passed, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks `expression` and returns it as `tuple_view` does.
    pub(super) fn tuple_expression(&mut self, expression: &Expr) -> TypedExpr {
        let value = self.expression(expression);
        self.tuple_view(value)
    }

    /// Returns `value` as the tuple of its items when it is `sys.version_info` or a named
    /// tuple; any other value as it is.
    pub(super) fn tuple_view(&self, value: TypedExpr) -> TypedExpr {
        self.fields_tuple(Self::version_tuple(value))
    }

    /// Returns a named tuple as the tuple of its fields; any other value as it is.
    pub(super) fn fields_tuple(&self, value: TypedExpr) -> TypedExpr {
        let Type::Instance(class) = &value.ty else {
            return value;
        };
        if !class.operators.named_tuple() {
            return value;
        }
        let Some(method) = self.program.method(class.id, NAMED_TUPLE_FIELDS) else {
            return value;
        };
        let signature = &self.program.signatures[method.0 as usize];
        let ty = signature.returns.clone();
        let span = value.span;
        let receiver = Self::converted(
            value,
            signature.parameters.first().map(|parameter| &parameter.ty),
        );
        Self::typed(
            TypedExprKind::Call {
                function: method,
                arguments: vec![Argument {
                    passed: Passed::Position,
                    parameter: 0,
                    value: receiver,
                }],
                dispatch: false,
            },
            ty,
            span,
        )
    }

    /// Returns the operands of a comparison that no method of their classes takes: for `in`,
    /// a named tuple on the right read as its fields; for another comparison, each named tuple
    /// read so when both operands are tuples or named tuples.
    pub(super) fn compared_tuples(
        &self,
        operator: CompareOperator,
        left: TypedExpr,
        right: TypedExpr,
    ) -> (TypedExpr, TypedExpr) {
        match operator {
            CompareOperator::In | CompareOperator::NotIn => (left, self.fields_tuple(right)),
            CompareOperator::Is | CompareOperator::IsNot => (left, right),
            _ if [&left.ty, &right.ty].iter().all(|ty| is_tuple(ty))
                && [&left.ty, &right.ty].iter().any(|ty| is_named_tuple(ty)) =>
            {
                (self.fields_tuple(left), self.fields_tuple(right))
            }
            _ => (left, right),
        }
    }

    /// Returns the operands of `+` and `*` that no method of their classes takes, each named
    /// tuple read as its fields, so they follow the rules of a fixed tuple: `+` joins two.
    pub(super) fn arithmetic_tuples(
        &self,
        operator: BinaryOperator,
        left: TypedExpr,
        right: TypedExpr,
    ) -> (TypedExpr, TypedExpr) {
        if matches!(operator, BinaryOperator::Add | BinaryOperator::Mult) {
            (self.fields_tuple(left), self.fields_tuple(right))
        } else {
            (left, right)
        }
    }
}

/// Returns true for a named tuple.
fn is_named_tuple(ty: &Type) -> bool {
    matches!(ty, Type::Instance(class) if class.operators.named_tuple())
}

/// Returns true for a tuple of any form or a named tuple.
fn is_tuple(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(_) | Type::TupleOf(_)) || is_named_tuple(ty)
}
