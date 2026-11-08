//! Lowers lists: displays, items, item assignment, `append`, `len`, `in`, equality, text, truth
//! and `for` over a list (DAT-07, DAT-11, EXP-07, CST-03, LIB-03). Items cross into the runtime
//! as eight bytes each; every runtime list function borrows its arguments.

use typy_check::{AssignTarget, Method, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{
    Comparison, Constant, IntOperator, Operation, RuntimeFunction, ScalarKind, Terminator,
    TyirType, ValueId,
};

use crate::function::{FunctionLowerer, LoopTargets};
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
    fn bits(&mut self, value: Lowered, span: Span) -> ValueId {
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

    /// Calls a method of a built-in type (LIB-03).
    pub(crate) fn method(
        &mut self,
        receiver: &TypedExpr,
        method: Method,
        arguments: &[TypedExpr],
        span: Span,
    ) -> Lowered {
        let receiver = self.expression(receiver);
        let arguments: Vec<Lowered> = arguments
            .iter()
            .map(|argument| self.expression(argument))
            .collect();
        let result = match (method, arguments.as_slice()) {
            (Method::ListAppend, [item]) => {
                let bits = self.bits(*item, span);
                self.call_runtime(
                    RuntimeFunction::ListAppend,
                    vec![receiver.value, bits],
                    span,
                )
            }
            (Method::ListAppend, _) => unreachable!("the checker gives `append` one argument"),
        };
        for argument in arguments {
            self.release(argument, span);
        }
        self.release(receiver, span);
        result
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

    /// Lowers `for target in list`. The list is evaluated once and kept in a hidden slot; each
    /// step reads the length again, as CPython's list iterator does, so items appended in the
    /// body are visited and a list that shrinks ends the loop (CST-03).
    pub(crate) fn for_each(
        &mut self,
        target: &AssignTarget,
        iterable: &TypedExpr,
        body: &[typy_check::TypedStmt],
        orelse: &[typy_check::TypedStmt],
        span: Span,
    ) {
        let list_type = self.lowered_type(&iterable.ty);
        let item = match &iterable.ty {
            typy_check::Type::List(item) => self.lowered_type(item),
            _ => unreachable!("the checker gives `for` a list here"),
        };
        let list_slot = self.hidden_slot("list", list_type);
        let position = self.hidden_slot("position", TyirType::Int);
        let lowered = self.expression(iterable);
        self.store(list_slot, lowered, span);
        let zero = Self::borrowed(
            self.constant(Constant::Int("0".to_owned()), span),
            TyirType::Int,
        );
        self.store(position, zero, span);
        let header = self.builder.create_block();
        let inside = self.builder.create_block();
        let advance = self.builder.create_block();
        let otherwise = self.builder.create_block();
        let exit = self.builder.create_block();
        self.jump_to(header);
        self.builder.switch_to(header);
        let list = self.load(list_slot, span);
        let length = self.list_length(list, span);
        let index = self.load(position, span);
        let more = self.builder.push_value(
            Operation::Compare {
                kind: ScalarKind::Int,
                comparison: Comparison::Less,
                left: index.value,
                right: length.value,
            },
            TyirType::Bool,
            span,
        );
        self.builder.terminate(Terminator::Branch {
            condition: more,
            then: inside,
            otherwise,
        });
        self.builder.switch_to(inside);
        let list = self.load(list_slot, span);
        let index = self.load(position, span);
        let value = self.list_item(list, index, item, span);
        self.assign_target(target, value, span);
        self.loops.push(LoopTargets {
            break_target: exit,
            continue_target: advance,
        });
        self.statements(body);
        self.loops.pop();
        self.jump_to(advance);
        self.builder.switch_to(advance);
        let current = self.load(position, span).value;
        let one = self.constant(Constant::Int("1".to_owned()), span);
        let next = self.builder.push_value(
            Operation::IntBinary {
                operator: IntOperator::Add,
                left: current,
                right: one,
            },
            TyirType::Int,
            span,
        );
        let next = self.produced(next, TyirType::Int);
        self.store(position, next, span);
        self.jump_to(header);
        self.builder.switch_to(otherwise);
        self.statements(orelse);
        self.jump_to(exit);
        self.builder.switch_to(exit);
    }

    /// Ends the current block with a jump to `target`, unless it already ends.
    fn jump_to(&mut self, target: typy_tyir::BlockId) {
        if !self.builder.is_terminated() {
            self.builder.terminate(Terminator::Jump {
                target,
                arguments: Vec::new(),
            });
        }
    }
}
