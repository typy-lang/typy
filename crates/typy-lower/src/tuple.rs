//! Lowers tuples: displays, items, unpacking, concatenation, comparison and text (DAT-08,
//! SMT-02, EXP-06).

use typy_check::{AssignTarget, TypedExpr};
use typy_diagnostics::Span;
use typy_source::syntax::CompareOperator;
use typy_tyir::{Constant, Operation, RuntimeFunction, Terminator, TupleTypeId, TyirType, ValueId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns the item types of a tuple type.
    fn item_types(&self, ty: TyirType) -> (TupleTypeId, Vec<TyirType>) {
        let TyirType::Tuple(id) = ty else {
            unreachable!("the checker gives a tuple operation only tuples")
        };
        let items = self.tuple_types.items(id).unwrap_or_default().to_vec();
        (id, items)
    }

    /// Returns item `index` of a tuple, borrowed from it.
    pub(crate) fn borrowed_item(
        &mut self,
        tuple: Lowered,
        index: usize,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let index = u32::try_from(index).unwrap_or(u32::MAX);
        let value = self.builder.push_value(
            Operation::TupleItem {
                tuple: tuple.value,
                index,
            },
            ty,
            span,
        );
        Self::borrowed(value, ty)
    }

    /// Lowers a tuple display. Every item is evaluated first, so an exception raised by a later
    /// item still releases the earlier ones; the tuple then takes over their references.
    pub(crate) fn tuple_display(
        &mut self,
        items: &[TypedExpr],
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let (id, _) = self.item_types(ty);
        let lowered: Vec<Lowered> = items.iter().map(|item| self.expression(item)).collect();
        let values: Vec<ValueId> = lowered
            .into_iter()
            .map(|item| self.take_reference(item, span))
            .collect();
        let value = self.builder.push_value(
            Operation::MakeTuple {
                ty: id,
                items: values,
            },
            ty,
            span,
        );
        self.produced(value, ty)
    }

    /// Returns a `tuple[T, ...]` with items of type `item`: a new list that `inner` builds,
    /// sealed, or a list made from the items of a fixed tuple, sealed (DAT-14).
    pub(crate) fn tuple_of(&mut self, inner: &TypedExpr, item: TyirType, span: Span) -> Lowered {
        let lowered = self.expression(inner);
        let list = if lowered.ty == TyirType::LIST {
            lowered
        } else {
            let (_, types) = self.item_types(lowered.ty);
            let list = self.list_display(&[], item, span);
            for (index, ty) in types.into_iter().enumerate() {
                let value = self.borrowed_item(lowered, index, ty, span);
                let bits = self.bits(value, span);
                self.call_runtime(RuntimeFunction::ListAppend, vec![list.value, bits], span);
            }
            self.release(lowered, span);
            list
        };
        self.call_runtime(RuntimeFunction::TupleSeal, vec![list.value], span);
        list
    }

    /// Returns a new tuple of the items at `positions`, each with a reference of its own; the
    /// tuple sliced is released.
    pub(crate) fn tuple_slice(
        &mut self,
        tuple: Lowered,
        positions: &[usize],
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let (_, items) = self.item_types(tuple.ty);
        let (id, _) = self.item_types(ty);
        let values: Vec<ValueId> = positions
            .iter()
            .map(|position| {
                let item = self.borrowed_item(tuple, *position, items[*position], span);
                self.take_reference(item, span)
            })
            .collect();
        let value = self.builder.push_value(
            Operation::MakeTuple {
                ty: id,
                items: values,
            },
            ty,
            span,
        );
        let sliced = self.produced(value, ty);
        self.release(tuple, span);
        sliced
    }

    /// Returns item `index` of a tuple. An item of a tuple the function owns gets its own
    /// reference before the tuple is released, so it outlives the tuple.
    pub(crate) fn tuple_item(
        &mut self,
        tuple: Lowered,
        index: usize,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let item = self.borrowed_item(tuple, index, ty, span);
        if !tuple.owned {
            return item;
        }
        let kept = if self.counted(ty) {
            self.builder.push(Operation::Increment(item.value), span);
            self.produced(item.value, ty)
        } else {
            item
        };
        self.release(tuple, span);
        kept
    }

    /// Stores a value into an assignment target; a tuple target stores each item into its own
    /// target, from left to right, then releases the tuple (SMT-02).
    pub(crate) fn assign_target(&mut self, target: &AssignTarget, value: Lowered, span: Span) {
        match target {
            AssignTarget::Name(symbol) => self.assign_name(*symbol, value, span),
            AssignTarget::Item { list, index } => self.assign_item(list, index, value, span),
            AssignTarget::Slice {
                list,
                lower,
                upper,
                step,
            } => {
                // The value is evaluated first; the list and the bounds after it (DAT-13).
                let bounds = [lower.as_deref(), upper.as_deref(), step.as_deref()];
                self.call_with_slice(
                    RuntimeFunction::ListAssignSlice,
                    list,
                    bounds,
                    vec![value.value],
                    span,
                );
                self.release(value, span);
            }
            AssignTarget::Entry { dict, key } => self.assign_entry(dict, key, value, span),
            AssignTarget::Attribute { object, index } => {
                self.assign_attribute(object, *index, value, span);
            }
            AssignTarget::Tuple(targets) => {
                let (_, items) = self.item_types(value.ty);
                for (index, (target, ty)) in targets.iter().zip(items).enumerate() {
                    let item = self.borrowed_item(value, index, ty, span);
                    self.assign_target(target, item, span);
                }
                self.release(value, span);
            }
        }
    }

    /// Returns `left + right`, a tuple of the items of both; the operands are released.
    pub(crate) fn tuple_concat(
        &mut self,
        left: Lowered,
        right: Lowered,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let (id, _) = self.item_types(ty);
        let mut values = Vec::new();
        for operand in [left, right] {
            let (_, items) = self.item_types(operand.ty);
            for (index, item) in items.into_iter().enumerate() {
                let item = self.borrowed_item(operand, index, item, span);
                values.push(self.take_reference(item, span));
            }
        }
        let value = self.builder.push_value(
            Operation::MakeTuple {
                ty: id,
                items: values,
            },
            ty,
            span,
        );
        self.release(left, span);
        self.release(right, span);
        self.produced(value, ty)
    }

    /// Returns the number of items of a tuple type.
    pub(crate) fn tuple_length(&self, ty: TyirType) -> usize {
        self.item_types(ty).1.len()
    }

    /// Returns `repr(tuple)`, which `str` also gives: the `repr` of each item between
    /// parentheses, with a comma after a single item, as CPython writes `(1,)`.
    pub(crate) fn tuple_text(&mut self, tuple: Lowered, span: Span) -> Lowered {
        let (_, items) = self.item_types(tuple.ty);
        let count = items.len();
        let mut text = Self::borrowed(self.string("(", span), TyirType::STR);
        for (index, ty) in items.into_iter().enumerate() {
            let item = self.borrowed_item(tuple, index, ty, span);
            let piece = self.repr_of(item, span);
            text = self.concat(text, piece, span);
            let separator = match (index + 1 == count, count) {
                (true, 1) => Some(","),
                (true, _) => None,
                (false, _) => Some(", "),
            };
            if let Some(separator) = separator {
                let separator = Self::borrowed(self.string(separator, span), TyirType::STR);
                text = self.concat(text, separator, span);
            }
        }
        let close = Self::borrowed(self.string(")", span), TyirType::STR);
        self.concat(text, close, span)
    }

    /// Returns `left + right` for two strings; both are released.
    fn concat(&mut self, left: Lowered, right: Lowered, span: Span) -> Lowered {
        let joined = self.call_runtime(
            RuntimeFunction::StrConcat,
            vec![left.value, right.value],
            span,
        );
        self.release(left, span);
        self.release(right, span);
        joined
    }

    /// Compares two tuples as CPython does: the first pair of items that are not equal decides
    /// the result, and tuples whose items are all equal are equal. The operands are not released.
    pub(crate) fn compare_tuples(
        &mut self,
        operator: CompareOperator,
        left: Lowered,
        right: Lowered,
        span: Span,
    ) -> ValueId {
        let (_, left_items) = self.item_types(left.ty);
        let (_, right_items) = self.item_types(right.ty);
        let join = self.builder.create_block();
        let result = self.builder.add_block_parameter(join, TyirType::Bool);
        for (index, (left_type, right_type)) in left_items.into_iter().zip(right_items).enumerate()
        {
            let a = self.borrowed_item(left, index, left_type, span);
            let b = self.borrowed_item(right, index, right_type, span);
            let equal = self.compare(CompareOperator::Eq, a, b, span);
            let next = self.builder.create_block();
            let differ = self.builder.create_block();
            self.builder.terminate(Terminator::Branch {
                condition: equal,
                then: next,
                otherwise: differ,
            });
            self.builder.switch_to(differ);
            let decided = match operator {
                CompareOperator::Eq => self.constant(Constant::Bool(false), span),
                CompareOperator::NotEq => self.constant(Constant::Bool(true), span),
                ordering => self.compare(ordering, a, b, span),
            };
            self.builder.terminate(Terminator::Jump {
                target: join,
                arguments: vec![decided],
            });
            self.builder.switch_to(next);
        }
        let all_equal = matches!(
            operator,
            CompareOperator::Eq | CompareOperator::LtE | CompareOperator::GtE
        );
        let equal = self.constant(Constant::Bool(all_equal), span);
        self.builder.terminate(Terminator::Jump {
            target: join,
            arguments: vec![equal],
        });
        self.builder.switch_to(join);
        result
    }

    /// Returns the first value of a slot of type `ty`, so releasing the slot is always safe, or
    /// `None` when the type is not counted and needs no first value.
    pub(crate) fn initial_value(&mut self, ty: TyirType, span: Span) -> Option<ValueId> {
        self.counted(ty).then(|| self.zero_value(ty, span))
    }

    /// Returns a value of type `ty` that is never freed: zero, false, `None`, the empty string,
    /// or a tuple of such values.
    fn zero_value(&mut self, ty: TyirType, span: Span) -> ValueId {
        match ty {
            TyirType::Int => self.constant(Constant::Int("0".to_owned()), span),
            TyirType::Float => self.constant(Constant::Float(0.0), span),
            TyirType::Bool => self.constant(Constant::Bool(false), span),
            TyirType::Unit => self.constant(Constant::Unit, span),
            TyirType::Object(typy_tyir::TypeId::LIST) => {
                self.constant(Constant::PlaceholderList, span)
            }
            TyirType::Object(typy_tyir::TypeId::SET) => {
                self.constant(Constant::PlaceholderSet, span)
            }
            TyirType::Object(typy_tyir::TypeId::DICT) => {
                self.constant(Constant::PlaceholderDict, span)
            }
            // A view slot starts as a view of the placeholder dict, which holds no reference
            // to an immortal object that matters, and is freed like any other view.
            TyirType::Object(typy_tyir::TypeId::DICT_VIEW) => {
                let dict = self.constant(Constant::PlaceholderDict, span);
                let kind = self.constant(Constant::Int("0".to_owned()), span);
                self.call_runtime(RuntimeFunction::DictView, vec![dict, kind], span)
                    .value
            }
            TyirType::Object(typy_tyir::TypeId::BYTES) => self.bytes_constant(&[], span),
            // An instance, exception, iterator or range slot starts as null, which counting
            // passes over.
            TyirType::Object(id)
                if id.class_index().is_some()
                    || id == typy_tyir::TypeId::EXCEPTION
                    || id == typy_tyir::TypeId::ITERATOR
                    || id == typy_tyir::TypeId::TRACEBACK
                    || id == typy_tyir::TypeId::RANGE =>
            {
                self.constant(Constant::NoInstance(id), span)
            }
            // A type object slot starts as `object`, which is immortal.
            TyirType::Object(typy_tyir::TypeId::TYPE) => self.constant(
                Constant::TypeObject(typy_tyir::TypeObjectRef::Runtime(
                    typy_tyir::RuntimeType::Object,
                )),
                span,
            ),
            TyirType::Object(_) => self.string("", span),
            TyirType::Bits => unreachable!("no slot holds list item bytes"),
            TyirType::Optional(_) => self.none_of(ty, span),
            TyirType::Tuple(id) => {
                let (_, items) = self.item_types(ty);
                let values: Vec<ValueId> = items
                    .into_iter()
                    .map(|item| self.zero_value(item, span))
                    .collect();
                self.builder.push_value(
                    Operation::MakeTuple {
                        ty: id,
                        items: values,
                    },
                    ty,
                    span,
                )
            }
        }
    }
}
