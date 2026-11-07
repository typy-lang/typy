//! The state of definite assignment (TYP-12): which variables are assigned on every path.

use std::collections::BTreeSet;

use crate::typed::SymbolId;

/// Which variables are assigned on every path; `None` when the point cannot be reached.
pub(super) type Assigned = Option<BTreeSet<SymbolId>>;

pub(super) fn meet(left: Assigned, right: Assigned) -> Assigned {
    match (left, right) {
        (None, other) | (other, None) => other,
        (Some(left), Some(right)) => Some(left.intersection(&right).copied().collect()),
    }
}
