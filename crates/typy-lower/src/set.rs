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
            self.store_call(
                RuntimeFunction::SetAdd,
                RuntimeFunction::SetAddMoved,
                vec![set.value, bits],
                value,
                span,
            );
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
            && matches!(sequence.ty, typy_check::Type::Dict(..))
        {
            // CPython adds a dict's keys with the hashes it stored, after one resize.
            let dict = self.expression(sequence);
            let set = self.new_set(item, span);
            let address = self.address(dict.value, span);
            let none = self.constant(Constant::Unit, span);
            let skip = self
                .builder
                .push_value(Operation::ToBits(none), TyirType::Bits, span);
            self.call_runtime(
                RuntimeFunction::SetExtend,
                vec![set.value, address, skip],
                span,
            );
            self.release(dict, span);
            return set;
        }
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

    /// Returns the address the runtime takes for another collection, a set or a list.
    pub(crate) fn address(&mut self, collection: ValueId, span: Span) -> ValueId {
        self.builder
            .push_value(Operation::ToBits(collection), TyirType::Bits, span)
    }

    /// Returns `left op right` for two sets, a new set, by CPython's steps for each operator;
    /// the operands are not released.
    pub(crate) fn set_combine(
        &mut self,
        operator: BinaryOperator,
        left: ValueId,
        right: ValueId,
        span: Span,
    ) -> Lowered {
        let other = self.address(right, span);
        match operator {
            BinaryOperator::BitOr => {
                let result = self.call_runtime(RuntimeFunction::SetCopy, vec![left], span);
                let skip = self.address(left, span);
                self.call_runtime(
                    RuntimeFunction::SetExtend,
                    vec![result.value, other, skip],
                    span,
                );
                result
            }
            BinaryOperator::BitAnd => {
                self.call_runtime(RuntimeFunction::SetIntersect, vec![left, other], span)
            }
            BinaryOperator::Sub => {
                self.call_runtime(RuntimeFunction::SetSubtract, vec![left, other], span)
            }
            _ => self.call_runtime(RuntimeFunction::SetSymmetric, vec![left, other], span),
        }
    }

    /// Changes `set` in place for `|=`, `&=`, `-=` and `^=`, as CPython's in-place operators of
    /// a set do; `other` is a set, or a list for the updating methods, and is not released.
    pub(crate) fn set_update(
        &mut self,
        operator: BinaryOperator,
        set: ValueId,
        other: ValueId,
        span: Span,
    ) {
        let address = self.address(other, span);
        match operator {
            BinaryOperator::BitOr => {
                let skip = self.address(set, span);
                self.call_runtime(RuntimeFunction::SetExtend, vec![set, address, skip], span);
            }
            BinaryOperator::BitAnd => {
                let common = self.set_intersection(set, &[other], span);
                self.call_runtime(RuntimeFunction::SetReplace, vec![set, common.value], span);
                self.release(common, span);
            }
            BinaryOperator::Sub => {
                self.call_runtime(RuntimeFunction::SetRemoveAll, vec![set, address], span);
            }
            _ => {
                self.call_runtime(RuntimeFunction::SetToggle, vec![set, address], span);
            }
        }
    }

    /// Returns `set.intersection(*others)`, a new set: a copy without others, and otherwise the
    /// set intersected with each in turn, as CPython does.
    fn set_intersection(&mut self, set: ValueId, others: &[ValueId], span: Span) -> Lowered {
        let Some((first, rest)) = others.split_first() else {
            return self.call_runtime(RuntimeFunction::SetCopy, vec![set], span);
        };
        let address = self.address(*first, span);
        let mut result = self.call_runtime(RuntimeFunction::SetIntersect, vec![set, address], span);
        for other in rest {
            let address = self.address(*other, span);
            let next = self.call_runtime(
                RuntimeFunction::SetIntersect,
                vec![result.value, address],
                span,
            );
            self.release(result, span);
            result = next;
        }
        result
    }

    /// Calls a method of a set with its arguments in source order.
    pub(crate) fn set_method(
        &mut self,
        set: ValueId,
        method: Method,
        arguments: &[Lowered],
        result: TyirType,
        span: Span,
    ) -> Lowered {
        let others: Vec<ValueId> = arguments.iter().map(|argument| argument.value).collect();
        match (method, others.as_slice()) {
            (Method::SetAdd, [_]) => {
                let bits = self.bits(arguments[0], span);
                self.store_call(
                    RuntimeFunction::SetAdd,
                    RuntimeFunction::SetAddMoved,
                    vec![set, bits],
                    arguments[0],
                    span,
                )
            }
            (Method::SetRemove | Method::SetDiscard, [_]) => {
                let function = match method {
                    Method::SetRemove => RuntimeFunction::SetRemove,
                    _ => RuntimeFunction::SetDiscard,
                };
                let bits = self.bits(arguments[0], span);
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
            (Method::SetIssubset | Method::SetIssuperset | Method::SetIsdisjoint, [other]) => {
                let code = match method {
                    Method::SetIssubset => "0",
                    Method::SetIssuperset => "1",
                    _ => "2",
                };
                let code = self.constant(Constant::Int(code.to_owned()), span);
                let address = self.address(*other, span);
                self.call_runtime(RuntimeFunction::SetRelation, vec![set, address, code], span)
            }
            (Method::SetSymmetricDifference, [other]) => {
                self.set_combine(BinaryOperator::BitXor, set, *other, span)
            }
            (Method::SetSymmetricDifferenceUpdate, [other]) => {
                self.set_update(BinaryOperator::BitXor, set, *other, span);
                self.unit(span)
            }
            (Method::SetUnion, others) => {
                let union = self.call_runtime(RuntimeFunction::SetCopy, vec![set], span);
                let skip = self.address(set, span);
                for other in others {
                    let address = self.address(*other, span);
                    self.call_runtime(
                        RuntimeFunction::SetExtend,
                        vec![union.value, address, skip],
                        span,
                    );
                }
                union
            }
            (Method::SetIntersection, others) => self.set_intersection(set, others, span),
            (Method::SetDifference, []) => {
                self.call_runtime(RuntimeFunction::SetCopy, vec![set], span)
            }
            (Method::SetDifference, [first, rest @ ..]) => {
                let address = self.address(*first, span);
                let difference =
                    self.call_runtime(RuntimeFunction::SetSubtract, vec![set, address], span);
                for other in rest {
                    self.set_update(BinaryOperator::Sub, difference.value, *other, span);
                }
                difference
            }
            (Method::SetUpdate, others) => {
                for other in others {
                    self.set_update(BinaryOperator::BitOr, set, *other, span);
                }
                self.unit(span)
            }
            (Method::SetIntersectionUpdate, others) => {
                let common = self.set_intersection(set, others, span);
                self.call_runtime(RuntimeFunction::SetReplace, vec![set, common.value], span);
                self.release(common, span);
                self.unit(span)
            }
            (Method::SetDifferenceUpdate, others) => {
                for other in others {
                    self.set_update(BinaryOperator::Sub, set, *other, span);
                }
                self.unit(span)
            }
            (method, _) => unreachable!("the checker gives `{}` its arguments", method.name()),
        }
    }

    /// Returns `None`, the result of a method that changes the set.
    fn unit(&mut self, span: Span) -> Lowered {
        Self::borrowed(self.constant(Constant::Unit, span), TyirType::Unit)
    }
}
