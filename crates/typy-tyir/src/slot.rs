//! Slots: the stack variables that hold a function's locals.

use crate::types::TyirType;

/// Index of a slot in its function.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SlotId(pub u32);

/// A local variable of a function. A counted slot starts empty and owns the reference it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    /// The Python name, for the text form and debug information.
    pub name: String,
    /// The type of the values it holds.
    pub ty: TyirType,
}
