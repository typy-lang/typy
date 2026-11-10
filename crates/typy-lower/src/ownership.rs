//! Owned and borrowed values, and the reference counting of slots (ADR-0001).

use typy_diagnostics::Span;
use typy_tyir::{Operation, SlotId, TyirType, ValueId};

use crate::function::FunctionLowerer;

/// A lowered expression: its value, its type, and whether this function owns a reference to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Lowered {
    pub(crate) value: ValueId,
    pub(crate) ty: TyirType,
    pub(crate) owned: bool,
}

impl FunctionLowerer<'_> {
    /// Returns a value the function now owns, such as the result of a call; it is tracked until it is released or moved.
    pub(crate) fn produced(&mut self, value: ValueId, ty: TyirType) -> Lowered {
        let owned = self.counted(ty);
        if owned {
            self.temporaries.push(value);
        }
        Lowered { value, ty, owned }
    }

    /// Returns a value the function does not own: a slot's value or an immortal constant.
    pub(crate) fn borrowed(value: ValueId, ty: TyirType) -> Lowered {
        Lowered {
            value,
            ty,
            owned: false,
        }
    }

    fn untrack(&mut self, value: ValueId) {
        if let Some(position) = self
            .temporaries
            .iter()
            .rposition(|tracked| *tracked == value)
        {
            self.temporaries.remove(position);
        }
    }

    /// Returns `to`, made from `from` by an operation that hands on its references, as owned when
    /// `from` was and borrowed otherwise.
    pub(crate) fn transfer(&mut self, from: Lowered, to: ValueId, ty: TyirType) -> Lowered {
        if from.owned {
            self.untrack(from.value);
            return self.produced(to, ty);
        }
        Self::borrowed(to, ty)
    }

    /// Returns the result of a runtime function that is borrowed from one of its arguments, such
    /// as the dict a view reads, which the function must not release.
    pub(crate) fn borrowed_result(&mut self, lowered: Lowered) -> Lowered {
        self.untrack(lowered.value);
        Self::borrowed(lowered.value, lowered.ty)
    }

    /// Drops the function's reference to a value it owns.
    pub(crate) fn release(&mut self, lowered: Lowered, span: Span) {
        if lowered.owned {
            self.builder.push(Operation::Decrement(lowered.value), span);
            self.untrack(lowered.value);
        }
    }

    /// Returns a value with one reference that the caller takes over: an owned value moves, a borrowed one is counted.
    pub(crate) fn take_reference(&mut self, lowered: Lowered, span: Span) -> ValueId {
        if lowered.owned {
            self.untrack(lowered.value);
        } else if self.counted(lowered.ty) {
            self.builder.push(Operation::Increment(lowered.value), span);
        }
        lowered.value
    }

    /// Stores a value in a slot, which takes over its reference and releases the value it held.
    pub(crate) fn store(&mut self, slot: SlotId, lowered: Lowered, span: Span) {
        let value = self.take_reference(lowered, span);
        if self.counted(lowered.ty) {
            let old = self
                .builder
                .push_value(Operation::Load(slot), lowered.ty, span);
            self.builder.push(Operation::Store { slot, value }, span);
            self.builder.push(Operation::Decrement(old), span);
        } else {
            self.builder.push(Operation::Store { slot, value }, span);
        }
    }

    /// Loads a slot's value, borrowed from the slot.
    pub(crate) fn load(&mut self, slot: SlotId, span: Span) -> Lowered {
        let ty = self.builder.slot_type(slot);
        let value = self.builder.push_value(Operation::Load(slot), ty, span);
        Self::borrowed(value, ty)
    }

    /// Releases the value of every counted slot, on the way out of the function.
    pub(crate) fn release_slots(&mut self, span: Span) {
        self.release_slots_but(None, span);
    }

    /// Releases the value of every counted slot but `kept`, whose reference a `return` hands
    /// to the caller, and but the parameters that borrow their caller's reference (MEM-04).
    pub(crate) fn release_slots_but(&mut self, kept: Option<SlotId>, span: Span) {
        for index in 0..self.builder.slot_count() {
            let slot = SlotId(u32::try_from(index).unwrap_or(u32::MAX));
            if Some(slot) == kept || self.borrowed.contains(&slot) {
                continue;
            }
            let ty = self.builder.slot_type(slot);
            if self.counted(ty) {
                let value = self.builder.push_value(Operation::Load(slot), ty, span);
                self.builder.push(Operation::Decrement(value), span);
            }
        }
    }

    /// Releases every owned temporary that is live, without forgetting them, on an error path.
    pub(crate) fn release_live_temporaries(&mut self, span: Span) {
        for value in self.temporaries.clone() {
            self.builder.push(Operation::Decrement(value), span);
        }
    }

    /// Adds a hidden slot for a value the lowering itself keeps, such as a loop counter.
    pub(crate) fn hidden_slot(&mut self, name: &str, ty: TyirType) -> SlotId {
        self.builder.add_slot(format!("${name}"), ty)
    }
}
