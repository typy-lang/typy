//! Lowers augmented assignment, `target op= value` (SMT-01, DAT-07, DAT-10). A list target
//! with `+=` or `*=` changes the list in place, as CPython's `list.__iadd__` and
//! `list.__imul__` do, and a set target with `|=`, `&=`, `-=` or `^=` changes the set, so every
//! name that shares the collection sees the change.

use typy_check::{AssignTarget, Type, TypedExpr};
use typy_diagnostics::Span;
use typy_source::syntax::BinaryOperator;
use typy_tyir::{Operation, RuntimeFunction, TyirType};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Lowers `target op= value`, where the operation gives a value of type `result`.
    pub(crate) fn augmented_assignment(
        &mut self,
        target: &AssignTarget,
        operator: BinaryOperator,
        value: &TypedExpr,
        result: &Type,
        span: Span,
    ) {
        match target {
            AssignTarget::Name(symbol) => {
                let slot = self.slots[symbol];
                let current = self.load(slot, span);
                let right = self.expression(value);
                if let Some(changed) = self.in_place(operator, current, right, result, span) {
                    self.release(changed, span);
                    return;
                }
                let lowered = self.binary(operator, current, right, result, span);
                self.store(slot, lowered, span);
            }
            AssignTarget::Item { list, index } => {
                self.augmented_item(list, index, operator, value, result, span);
            }
            AssignTarget::Entry { dict, key } => {
                self.augmented_entry(
                    dict,
                    key,
                    |lowerer, current| {
                        let right = lowerer.expression(value);
                        match lowerer.in_place(operator, current, right, result, span) {
                            Some(changed) => changed,
                            None => lowerer.binary(operator, current, right, result, span),
                        }
                    },
                    span,
                );
            }
            AssignTarget::Tuple(_) => {
                unreachable!("Python refuses a tuple as the target of an augmented assignment")
            }
        }
    }

    /// Lowers `list[index] op= value`. The list and the index are evaluated once, and the item
    /// is read before the value is evaluated, as CPython does. The item keeps a reference of its
    /// own while the value is evaluated, which may change the list.
    fn augmented_item(
        &mut self,
        list: &TypedExpr,
        index: &TypedExpr,
        operator: BinaryOperator,
        value: &TypedExpr,
        result: &Type,
        span: Span,
    ) {
        let list = self.expression(list);
        let index = self.expression(index);
        let item = self.lowered_type(result);
        let bits = self.call_runtime(
            RuntimeFunction::ListGet,
            vec![list.value, index.value],
            span,
        );
        let current = self.builder.push_value(
            Operation::FromBits {
                bits: bits.value,
                ty: item,
            },
            item,
            span,
        );
        if self.counted(item) {
            self.builder.push(Operation::Increment(current), span);
        }
        let current = self.produced(current, item);
        let right = self.expression(value);
        let lowered = match self.in_place(operator, current, right, result, span) {
            Some(changed) => changed,
            None => self.binary(operator, current, right, result, span),
        };
        let bits = self.bits(lowered, span);
        self.call_runtime(
            RuntimeFunction::ListSet,
            vec![list.value, index.value, bits],
            span,
        );
        self.release(lowered, span);
        self.release(index, span);
        self.release(list, span);
    }

    /// Changes a list in place for `+=` and `*=`, and a set for `|=`, `&=`, `-=` and `^=`,
    /// releases `right`, and returns the collection; returns `None`, releasing nothing, for any
    /// other operation. A `tuple[T, ...]` is a list object too, but is never changed: its
    /// result is a new tuple, as for a fixed tuple.
    fn in_place(
        &mut self,
        operator: BinaryOperator,
        current: Lowered,
        right: Lowered,
        result: &Type,
        span: Span,
    ) -> Option<Lowered> {
        if current.ty == TyirType::DICT {
            // `|=` is the only augmented operator of a dict, and updates it in place.
            self.dict_update(current.value, right.value, span);
            self.release(right, span);
            return Some(current);
        }
        if current.ty == TyirType::SET {
            self.set_update(operator, current.value, right.value, span);
            self.release(right, span);
            return Some(current);
        }
        if !matches!(result, Type::List(_)) {
            return None;
        }
        match operator {
            BinaryOperator::Add => {
                self.call_runtime(
                    RuntimeFunction::ListExtend,
                    vec![current.value, right.value],
                    span,
                );
            }
            BinaryOperator::Mult => {
                // The repetition is built first, so a count that raises leaves the list as it
                // was, as in CPython.
                let repeated = self.call_runtime(
                    RuntimeFunction::ListRepeat,
                    vec![current.value, right.value],
                    span,
                );
                self.call_runtime(RuntimeFunction::ListClear, vec![current.value], span);
                self.call_runtime(
                    RuntimeFunction::ListExtend,
                    vec![current.value, repeated.value],
                    span,
                );
                self.release(repeated, span);
            }
            _ => return None,
        }
        self.release(right, span);
        Some(current)
    }
}
