//! Lowers sets: displays, `set(source)`, operators and methods (DAT-10, LIB-03). Each runtime
//! set function borrows its arguments; one that stores an item takes a reference of its own.

use typy_check::{LoopSource, Method, TypedConstant, TypedExpr, TypedExprKind};
use typy_diagnostics::Span;
use typy_source::syntax::{BinaryOperator, UnaryOperator};
use typy_tyir::{Constant, Operation, RuntimeFunction, TyirType, ValueId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

/// The fewest and the most items of a set display CPython builds from a frozenset constant.
const CONSTANT_DISPLAY: std::ops::RangeInclusive<usize> = 3..=30;

/// Returns whether CPython folds an item of a display into a constant: a literal, a number
/// with signs, or a tuple of such items.
fn is_constant(item: &TypedExpr) -> bool {
    match &item.kind {
        TypedExprKind::Constant(_) => true,
        TypedExprKind::Unary {
            operator: UnaryOperator::USub | UnaryOperator::UAdd,
            operand,
        } => is_signed_number(operand),
        TypedExprKind::Tuple(items) => items.iter().all(is_constant),
        _ => false,
    }
}

fn is_signed_number(item: &TypedExpr) -> bool {
    match &item.kind {
        TypedExprKind::Constant(TypedConstant::Int(_) | TypedConstant::Float(_)) => true,
        TypedExprKind::Unary {
            operator: UnaryOperator::USub | UnaryOperator::UAdd,
            operand,
        } => is_signed_number(operand),
        _ => false,
    }
}

/// Returns the code `typy_rt_set_combine` and `typy_rt_set_update` take for an operator.
fn operation_code(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::BitOr => "0",
        BinaryOperator::BitAnd => "1",
        BinaryOperator::Sub => "2",
        _ => "3",
    }
}

impl FunctionLowerer<'_> {
    /// Returns a new empty set of items of type `item`.
    pub(crate) fn new_set(&mut self, item: TyirType, span: Span) -> Lowered {
        let (kind, descriptor) = self.item_layout(item, span);
        self.call_runtime(RuntimeFunction::SetNew, vec![kind, descriptor], span)
    }

    /// Lowers a set display: each item added in order. A display of three to thirty constant
    /// items is then copied, as CPython 3.15 merges the frozenset it builds into a new set,
    /// which can place the items in other slots (OI-19).
    pub(crate) fn set_display(
        &mut self,
        items: &[TypedExpr],
        item: TyirType,
        span: Span,
    ) -> Lowered {
        let set = self.new_set(item, span);
        for element in items {
            let value = self.expression(element);
            let bits = self.bits(value, span);
            self.call_runtime(RuntimeFunction::SetAdd, vec![set.value, bits], span);
            self.release(value, span);
        }
        if !(CONSTANT_DISPLAY.contains(&items.len()) && items.iter().all(is_constant)) {
            return set;
        }
        let copy = self.call_runtime(RuntimeFunction::SetCopy, vec![set.value], span);
        self.release(set, span);
        copy
    }

    /// Returns `set(source)`. A set is copied, which merges its table as CPython does; any
    /// other source is read as a `for` loop reads it, adding each value in order.
    pub(crate) fn set_of(&mut self, source: &LoopSource, item: TyirType, span: Span) -> Lowered {
        if let LoopSource::Items(sequence) = source
            && matches!(sequence.ty, typy_check::Type::Set(_))
        {
            let set = self.expression(sequence);
            let copy = self.call_runtime(RuntimeFunction::SetCopy, vec![set.value], span);
            self.release(set, span);
            return copy;
        }
        let set = self.new_set(item, span);
        self.each_value(source, span, |lowerer, value| {
            let bits = lowerer.bits(value, span);
            lowerer.call_runtime(RuntimeFunction::SetAdd, vec![set.value, bits], span);
        });
        set
    }

    /// Returns `left op right` for two sets, a new set; the operands are not released.
    pub(crate) fn set_combine(
        &mut self,
        operator: BinaryOperator,
        left: ValueId,
        right: ValueId,
        span: Span,
    ) -> Lowered {
        let code = self.constant(Constant::Int(operation_code(operator).to_owned()), span);
        self.call_runtime(RuntimeFunction::SetCombine, vec![left, right, code], span)
    }

    /// Changes `set` in place for `|=`, `&=`, `-=` and `^=`, as CPython's in-place operators of
    /// a set do; the operands are not released.
    pub(crate) fn set_update(
        &mut self,
        operator: BinaryOperator,
        set: ValueId,
        other: ValueId,
        span: Span,
    ) {
        let code = self.constant(Constant::Int(operation_code(operator).to_owned()), span);
        self.call_runtime(RuntimeFunction::SetUpdate, vec![set, other, code], span);
    }

    /// Calls a method of a set with its arguments in parameter order.
    pub(crate) fn set_method(
        &mut self,
        set: ValueId,
        method: Method,
        arguments: &[Lowered],
        result: TyirType,
        span: Span,
    ) -> Lowered {
        match (method, arguments) {
            (Method::SetAdd | Method::SetRemove | Method::SetDiscard, [item]) => {
                let function = match method {
                    Method::SetAdd => RuntimeFunction::SetAdd,
                    Method::SetRemove => RuntimeFunction::SetRemove,
                    _ => RuntimeFunction::SetDiscard,
                };
                let bits = self.bits(*item, span);
                self.call_runtime(function, vec![set, bits], span)
            }
            (Method::SetPop, []) => {
                // The set gives up its reference to the item, so the result is owned.
                let bits = self.call_runtime(RuntimeFunction::SetPop, vec![set], span);
                let value = self.builder.push_value(
                    Operation::FromBits {
                        bits: bits.value,
                        ty: result,
                    },
                    result,
                    span,
                );
                self.produced(value, result)
            }
            (Method::SetClear, []) => self.call_runtime(RuntimeFunction::SetClear, vec![set], span),
            (Method::SetCopy, []) => self.call_runtime(RuntimeFunction::SetCopy, vec![set], span),
            (Method::SetIsdisjoint, [other]) => {
                self.call_runtime(RuntimeFunction::SetIsdisjoint, vec![set, other.value], span)
            }
            (Method::SetIssubset | Method::SetIssuperset, [other]) => {
                let code = if method == Method::SetIssubset {
                    "3"
                } else {
                    "5"
                };
                let code = self.constant(Constant::Int(code.to_owned()), span);
                self.call_runtime(
                    RuntimeFunction::SetCompare,
                    vec![set, other.value, code],
                    span,
                )
            }
            (_, [other]) => {
                let (operator, in_place) = match method {
                    Method::SetUnion => (BinaryOperator::BitOr, false),
                    Method::SetIntersection => (BinaryOperator::BitAnd, false),
                    Method::SetDifference => (BinaryOperator::Sub, false),
                    Method::SetSymmetricDifference => (BinaryOperator::BitXor, false),
                    Method::SetUpdate => (BinaryOperator::BitOr, true),
                    Method::SetIntersectionUpdate => (BinaryOperator::BitAnd, true),
                    Method::SetDifferenceUpdate => (BinaryOperator::Sub, true),
                    Method::SetSymmetricDifferenceUpdate => (BinaryOperator::BitXor, true),
                    method => unreachable!("`{}` is not a set method", method.name()),
                };
                if !in_place {
                    return self.set_combine(operator, set, other.value, span);
                }
                self.set_update(operator, set, other.value, span);
                Self::borrowed(self.constant(Constant::Unit, span), TyirType::Unit)
            }
            (method, _) => unreachable!("the checker gives `{}` its arguments", method.name()),
        }
    }
}
