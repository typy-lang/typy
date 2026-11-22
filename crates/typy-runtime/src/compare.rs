//! Turns an ordering into the result of a comparison, numbered as TyIR's `Comparison::code`.

use core::cmp::Ordering;

use crate::word::inline_value;

/// Returns whether `ordering` satisfies the comparison whose code is the tagged `int` `comparison`: 0 `==`, 1 `!=`, 2 `<`, 3 `<=`, 4 `>`, 5 `>=`.
pub(crate) fn holds(ordering: Ordering, comparison: i64) -> bool {
    match inline_value(comparison) {
        0 => ordering.is_eq(),
        1 => ordering.is_ne(),
        2 => ordering.is_lt(),
        3 => ordering.is_le(),
        4 => ordering.is_gt(),
        _ => ordering.is_ge(),
    }
}

#[cfg(test)]
mod tests;
