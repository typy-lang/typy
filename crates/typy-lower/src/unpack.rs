//! Lowers unpacking targets (SMT-02, EXP-17): a tuple of a fixed number of items gives each
//! target its item, and a list of those a starred target takes; any other iterable goes
//! through the runtime, which puts its values in a new list and raises CPython's
//! `ValueError` when there are too few or too many, and each target then takes its value.

use typy_check::{AssignTarget, Type};
use typy_diagnostics::Span;
use typy_tyir::{Constant, Operation, RuntimeFunction, TyirType};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

/// The bits of the presence mask of a slice that gives a start and a stop, as the runtime's
/// `slice::START` and `slice::STOP` are.
const START_AND_STOP: i64 = 3;
/// The bits of the presence mask of a slice that gives a start alone.
const START: i64 = 1;

impl FunctionLowerer<'_> {
    /// Stores the values of `value` into `before`, then `rest` as a list, then `after`; the
    /// value is released. `item` is the type of the values, or of those `rest` takes from a
    /// tuple of a fixed number of items.
    pub(crate) fn assign_unpacked(
        &mut self,
        (before, rest, after): (&[AssignTarget], Option<&AssignTarget>, &[AssignTarget]),
        item: &Type,
        value: Lowered,
        span: Span,
    ) {
        let item = self.lowered_type(item);
        if let TyirType::Tuple(_) = value.ty {
            self.assign_tuple_unpacked((before, rest, after), item, value, span);
            return;
        }
        let (kind, descriptor) = self.item_layout(item, span);
        let source = self.bits(Self::borrowed(value.value, value.ty), span);
        let counts = [before.len(), after.len()]
            .map(|count| self.constant(Constant::Int(count.to_string()), span));
        let starred = self.constant(Constant::Bool(rest.is_some()), span);
        let values = self.call_runtime(
            RuntimeFunction::Unpack,
            vec![source, kind, descriptor, counts[0], counts[1], starred],
            span,
        );
        self.release(value, span);
        // The targets take their values from left to right, as CPython assigns them; the
        // last ones count from the end of the list.
        let count = |length: usize| i64::try_from(length).unwrap_or(i64::MAX);
        let leading: Vec<(i64, &AssignTarget)> = before
            .iter()
            .enumerate()
            .map(|(index, target)| (count(index), target))
            .collect();
        let trailing: Vec<(i64, &AssignTarget)> = after
            .iter()
            .enumerate()
            .map(|(index, target)| (count(index) - count(after.len()), target))
            .collect();
        self.assign_values(&leading, values, item, span);
        if let Some(rest) = rest {
            let taken = self.list_between(values.value, before.len(), after.len(), span);
            self.assign_target(rest, taken, span);
        }
        self.assign_values(&trailing, values, item, span);
        self.release(values, span);
    }

    /// Stores item `position` of the list `list`, counted from the end when negative, into
    /// each target, in order.
    fn assign_values(
        &mut self,
        targets: &[(i64, &AssignTarget)],
        list: Lowered,
        item: TyirType,
        span: Span,
    ) {
        for (position, target) in targets {
            let index = self.constant(Constant::Int(position.to_string()), span);
            let bits = self.call_runtime(RuntimeFunction::ListGet, vec![list.value, index], span);
            let value = self.builder.push_value(
                Operation::FromBits {
                    bits: bits.value,
                    ty: item,
                },
                item,
                span,
            );
            self.assign_target(target, Self::borrowed(value, item), span);
        }
    }

    /// Returns a new list of the items of `list` after the first `before` and before the last
    /// `after`.
    pub(crate) fn list_between(
        &mut self,
        list: typy_tyir::ValueId,
        before: usize,
        after: usize,
        span: Span,
    ) -> Lowered {
        let start = self.constant(Constant::Int(before.to_string()), span);
        let stop = self.constant(Constant::Int(format!("-{after}")), span);
        let one = self.constant(Constant::Int("1".to_owned()), span);
        let present = if after == 0 { START } else { START_AND_STOP };
        let present = self.constant(Constant::Int(present.to_string()), span);
        self.call_runtime(
            RuntimeFunction::ListSlice,
            vec![list, start, stop, one, present],
            span,
        )
    }

    /// Stores the items of a tuple of a fixed number of items into `before`, a new list of
    /// those between into `rest`, and the last into `after`; the tuple is released.
    fn assign_tuple_unpacked(
        &mut self,
        (before, rest, after): (&[AssignTarget], Option<&AssignTarget>, &[AssignTarget]),
        item: TyirType,
        tuple: Lowered,
        span: Span,
    ) {
        let (_, types) = self.item_types(tuple.ty);
        let middle = before.len()..types.len() - after.len();
        for (index, target) in before.iter().enumerate() {
            let value = self.borrowed_item(tuple, index, types[index], span);
            self.assign_target(target, value, span);
        }
        if let Some(rest) = rest {
            let list = self.list_display(&[], item, span);
            for index in middle.clone() {
                let value = self.borrowed_item(tuple, index, types[index], span);
                let bits = self.bits(value, span);
                self.call_runtime(RuntimeFunction::ListAppend, vec![list.value, bits], span);
            }
            self.assign_target(rest, list, span);
        }
        for (offset, target) in after.iter().enumerate() {
            let index = middle.end + offset;
            let value = self.borrowed_item(tuple, index, types[index], span);
            self.assign_target(target, value, span);
        }
        self.release(tuple, span);
    }
}
