//! Lowers lists: displays, items, item assignment, `len`, `in`, equality and truth (DAT-07,
//! DAT-11, EXP-07). Items cross into the runtime as eight bytes each; every runtime list
//! function borrows its arguments.

use typy_check::TypedExpr;
use typy_diagnostics::Span;
use typy_tyir::{
    Comparison, Constant, Operation, RuntimeFunction, ScalarKind, Terminator, TyirType, ValueId,
};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

/// Returns the runtime's code for the kind of item a list of `item` holds.
fn item_kind(item: TyirType) -> i64 {
    match item {
        TyirType::Int => 0,
        TyirType::Float => 1,
        TyirType::Bool => 2,
        _ => 3,
    }
}

impl FunctionLowerer<'_> {
    /// Returns the eight bytes a list stores for a value, without changing who owns it.
    pub(crate) fn bits(&mut self, value: Lowered, span: Span) -> ValueId {
        self.builder
            .push_value(Operation::ToBits(value.value), TyirType::Bits, span)
    }

    /// Lowers a list display: a list with room for every item, then each item appended in order.
    pub(crate) fn list_display(
        &mut self,
        items: &[TypedExpr],
        item: TyirType,
        span: Span,
    ) -> Lowered {
        let kind = self.constant(Constant::Int(item_kind(item).to_string()), span);
        let capacity = self.constant(Constant::Int(items.len().to_string()), span);
        let list = self.call_runtime(RuntimeFunction::ListNew, vec![kind, capacity], span);
        for element in items {
            let value = self.expression(element);
            let bits = self.bits(value, span);
            self.call_runtime(RuntimeFunction::ListAppend, vec![list.value, bits], span);
            self.release(value, span);
        }
        list
    }

    /// Returns `list[index]`. An item of a list the function owns gets its own reference before
    /// the list is released, so it outlives the list; the index is released.
    pub(crate) fn list_item(
        &mut self,
        list: Lowered,
        index: Lowered,
        item: TyirType,
        span: Span,
    ) -> Lowered {
        let bits = self.call_runtime(
            RuntimeFunction::ListGet,
            vec![list.value, index.value],
            span,
        );
        self.release(index, span);
        let value = self.builder.push_value(
            Operation::FromBits {
                bits: bits.value,
                ty: item,
            },
            item,
            span,
        );
        if !list.owned {
            return Self::borrowed(value, item);
        }
        let kept = if self.counted(item) {
            self.builder.push(Operation::Increment(value), span);
            self.produced(value, item)
        } else {
            Self::borrowed(value, item)
        };
        self.release(list, span);
        kept
    }

    /// Stores a value into `list[index]`; the value is evaluated first, then the list and the
    /// index, as CPython does. The list takes a reference of its own, so the value is released.
    pub(crate) fn assign_item(
        &mut self,
        list: &TypedExpr,
        index: &TypedExpr,
        value: Lowered,
        span: Span,
    ) {
        let list = self.expression(list);
        let index = self.expression(index);
        let bits = self.bits(value, span);
        self.call_runtime(
            RuntimeFunction::ListSet,
            vec![list.value, index.value, bits],
            span,
        );
        self.release(value, span);
        self.release(index, span);
        self.release(list, span);
    }

    /// Returns `len(list)` as an `int`.
    pub(crate) fn list_length(&mut self, list: Lowered, span: Span) -> Lowered {
        self.call_runtime(RuntimeFunction::ListLength, vec![list.value], span)
    }

    /// Returns whether a list has items, for a truth test (EXP-10).
    pub(crate) fn list_truth(&mut self, list: Lowered, span: Span) -> ValueId {
        let length = self.list_length(list, span);
        self.builder.push_value(
            Operation::IsTrue {
                kind: ScalarKind::Int,
                value: length.value,
            },
            TyirType::Bool,
            span,
        )
    }

    /// Returns `item in list`; the operands are not released.
    pub(crate) fn list_contains(&mut self, item: Lowered, list: Lowered, span: Span) -> ValueId {
        let bits = self.bits(item, span);
        self.call_runtime(RuntimeFunction::ListContains, vec![list.value, bits], span)
            .value
    }

    /// Returns `left == right`, or `left != right`, for two lists; the operands are not released.
    pub(crate) fn list_equality(
        &mut self,
        comparison: Comparison,
        left: Lowered,
        right: Lowered,
        span: Span,
    ) -> ValueId {
        let equal = self
            .call_runtime(
                RuntimeFunction::ListEqual,
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

    /// Ends the current block with a jump to `target`, unless it already ends.
    pub(crate) fn jump_to(&mut self, target: typy_tyir::BlockId) {
        if !self.builder.is_terminated() {
            self.builder.terminate(Terminator::Jump {
                target,
                arguments: Vec::new(),
            });
        }
    }
}
