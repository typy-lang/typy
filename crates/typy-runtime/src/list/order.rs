//! Lists as values: `+`, `*` and the comparisons, as CPython's `list_concat`, `list_repeat`
//! and `list_richcompare` give them.

use super::{ListObject, new_list, push_retained};
use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::word::{inline_value, is_inline};

/// Returns `left + right`, a new list of the items of both.
///
/// # Safety
///
/// Both point to live lists of one item kind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_concat(
    left: *const ListObject,
    right: *const ListObject,
) -> *mut ListObject {
    // SAFETY: the caller passes live lists; every index stays below each length.
    unsafe {
        let result = new_list((*left).item_type(), (*left).length + (*right).length);
        for list in [left, right] {
            for index in 0..(*list).length {
                push_retained(result, (*list).item(index));
            }
        }
        result
    }
}

/// Returns `list * count`, the items repeated `count` times; an empty list for a count below one.
///
/// # Safety
///
/// `list` points to a live list, and `count` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_repeat(
    list: *const ListObject,
    count: i64,
) -> Raising<*mut ListObject> {
    // SAFETY: the caller passes a live list; every index stays below its length.
    unsafe {
        if !is_inline(count) && !crate::int::is_negative(count) {
            let error = raise(
                ExceptionKind::OverflowError,
                "cannot fit 'int' into an index-sized integer",
            );
            return Raising::error(core::ptr::null_mut(), error);
        }
        let times = if is_inline(count) {
            usize::try_from(inline_value(count).max(0)).unwrap_or(0)
        } else {
            0
        };
        let Some(total) = (*list).length.checked_mul(times) else {
            let error = raise(
                ExceptionKind::OverflowError,
                "cannot fit 'int' into an index-sized integer",
            );
            return Raising::error(core::ptr::null_mut(), error);
        };
        let result = new_list((*list).item_type(), total);
        for _ in 0..times {
            for index in 0..(*list).length {
                push_retained(result, (*list).item(index));
            }
        }
        Raising::ok(result)
    }
}

/// Returns whether two lists stand in the comparison whose code is the `int` word `comparison`.
/// The first pair of items that are not equal decides it; when one list runs out first, the
/// lengths do.
///
/// # Safety
///
/// Both point to live lists of one item kind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_compare(
    left: *const ListObject,
    right: *const ListObject,
    comparison: i64,
) -> bool {
    // SAFETY: the caller passes live lists; every index stays below both lengths.
    unsafe {
        let item_type = (*left).item_type();
        let shared = (*left).length.min((*right).length);
        for index in 0..shared {
            let (a, b) = ((*left).item(index), (*right).item(index));
            if !item_type.equal(a, b) {
                return match inline_value(comparison) {
                    0 => false,
                    1 => true,
                    _ => item_type.order(a, b, comparison),
                };
            }
        }
        crate::compare::holds((*left).length.cmp(&(*right).length), comparison)
    }
}
