//! Functions over every item of a list: sorting, `min`, `max`, `sum`, `any` and `all`
//! (LIB-02, LIB-03).

use super::ListObject;
use super::item::ItemKind;
use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::word::inline_value;

/// The code of `<`, as an `int` word, in the numbering of `Comparison::code`.
const LESS: i64 = (2 << 1) | 1;
/// The code of `>`.
const GREATER: i64 = (4 << 1) | 1;
/// The length of the runs sorted by insertion before they are merged.
const RUN: usize = 32;

/// Sorts the items in place, stably, comparing only with `<` as CPython does (`list.sort`).
/// With `reverse`, an `int` word that is not 0, the list is reversed, sorted and reversed
/// again, as CPython does, so equal items keep their order.
///
/// # Safety
///
/// `list` points to a live list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_sort(list: *mut ListObject, reverse: i64) {
    // SAFETY: the caller passes a live list, whose first `length` items are initialised.
    unsafe {
        let length = (*list).length;
        if length < 2 {
            return;
        }
        let kind = (*list).item_kind();
        let items = core::slice::from_raw_parts_mut((*list).items, length);
        let reverse = inline_value(reverse) != 0;
        if reverse {
            items.reverse();
        }
        sort(kind, items);
        if reverse {
            items.reverse();
        }
    }
}

/// Returns the items of a list; an empty list may have no buffer at all.
///
/// # Safety
///
/// `list` points to a live list, which outlives the slice and is not changed while it is used.
unsafe fn items_of<'list>(list: *const ListObject) -> &'list [u64] {
    // SAFETY: the caller passes a live list, whose first `length` items are initialised.
    unsafe {
        if (*list).length == 0 {
            &[]
        } else {
            core::slice::from_raw_parts((*list).items, (*list).length)
        }
    }
}

/// Sorts `items` stably: runs of `RUN` items by binary insertion, then merged bottom up.
///
/// # Safety
///
/// Every item is a live item of `kind`.
unsafe fn sort(kind: ItemKind, items: &mut [u64]) {
    // SAFETY: the caller passes live items of `kind`.
    let less = |left: u64, right: u64| unsafe { kind.order(left, right, LESS) };
    for run in items.chunks_mut(RUN) {
        insertion_sort(run, &less);
    }
    if items.len() <= RUN {
        return;
    }
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "malloc returns memory aligned for every type"
    )]
    let buffer =
        crate::allocator::allocate(crate::allocator::array_size(items.len(), 8, 0)).cast::<u64>();
    // SAFETY: the buffer has room for every item, and is freed below.
    let scratch = unsafe { core::slice::from_raw_parts_mut(buffer, items.len()) };
    let mut width = RUN;
    while width < items.len() {
        let mut start = 0;
        while start + width < items.len() {
            let end = (start + 2 * width).min(items.len());
            merge(&mut items[start..end], width, scratch, &less);
            start = end;
        }
        width *= 2;
    }
    crate::allocator::free(buffer.cast());
}

/// Sorts a short run by binary insertion; an item goes after every item it is not less than.
fn insertion_sort(run: &mut [u64], less: &impl Fn(u64, u64) -> bool) {
    for next in 1..run.len() {
        let pivot = run[next];
        let (mut low, mut high) = (0, next);
        while low < high {
            let middle = low + (high - low) / 2;
            if less(pivot, run[middle]) {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        run.copy_within(low..next, low + 1);
        run[low] = pivot;
    }
}

/// Merges the sorted halves `items[..middle]` and `items[middle..]`; an item of the right half
/// goes first only when it is less than the left one, which keeps the sort stable.
fn merge(items: &mut [u64], middle: usize, scratch: &mut [u64], less: &impl Fn(u64, u64) -> bool) {
    scratch[..middle].copy_from_slice(&items[..middle]);
    let (mut left, mut right, mut out) = (0, middle, 0);
    while left < middle && right < items.len() {
        if less(items[right], scratch[left]) {
            items[out] = items[right];
            right += 1;
        } else {
            items[out] = scratch[left];
            left += 1;
        }
        out += 1;
    }
    items[out..out + middle - left].copy_from_slice(&scratch[left..middle]);
}

/// Returns the smallest item, or with `largest`, an `int` word that is not 0, the largest, with
/// a reference of its own (`min` and `max`). As in CPython, an item replaces the one kept so far
/// only when it is less, or greater, so the first of equal items wins; an empty list raises
/// `ValueError`.
///
/// # Safety
///
/// `list` points to a live list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_extreme(
    list: *const ListObject,
    largest: i64,
) -> Raising<u64> {
    let largest = inline_value(largest) != 0;
    // SAFETY: the caller passes a live list, whose first `length` items are initialised.
    unsafe {
        let kind = (*list).item_kind();
        let items = items_of(list);
        let Some((&first, rest)) = items.split_first() else {
            let message = if largest {
                "max() iterable argument is empty"
            } else {
                "min() iterable argument is empty"
            };
            return Raising::error(0, raise(ExceptionKind::ValueError, message));
        };
        let comparison = if largest { GREATER } else { LESS };
        let kept = rest.iter().fold(first, |kept, &item| {
            if kind.order(item, kept, comparison) {
                item
            } else {
                kept
            }
        });
        kind.retain(kept);
        Raising::ok(kept)
    }
}

/// Returns `sum(list, start)` for a list of `int` items, as a new `int` word.
///
/// # Safety
///
/// `list` points to a live list of `int` items, and `start` is a live `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_sum_int(list: *const ListObject, start: i64) -> i64 {
    // SAFETY: the caller passes a live list of `int` words and a live `start`.
    unsafe {
        crate::refcount::typy_rt_int_increment(start);
        let items = items_of(list);
        items.iter().fold(start, |total, &item| {
            let next = crate::int::typy_rt_int_add(total, item.cast_signed());
            crate::refcount::typy_rt_int_decrement(total);
            next
        })
    }
}

/// Returns `sum(list, start)` for a list of `float` items, with the compensated summation of
/// CPython 3.12 and later, Neumaier's improvement of Kahan's: the rounding error of each
/// addition is collected and added once at the end, unless it is not finite.
///
/// # Safety
///
/// `list` points to a live list of `float` items.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_sum_float(list: *const ListObject, start: f64) -> f64 {
    // SAFETY: the caller passes a live list of float bits.
    let items = unsafe { items_of(list) };
    let mut total = start;
    let mut compensation = 0.0;
    for &item in items {
        let value = f64::from_bits(item);
        let next = total + value;
        if libm_fabs(total) >= libm_fabs(value) {
            compensation += (total - next) + value;
        } else {
            compensation += (value - next) + total;
        }
        total = next;
    }
    if compensation != 0.0 && compensation.is_finite() {
        total += compensation;
    }
    total
}

/// Returns the magnitude of a double, without the C library.
fn libm_fabs(value: f64) -> f64 {
    f64::from_bits(value.to_bits() & !(1 << 63))
}

/// Returns `any(list)`: whether some item is true.
///
/// # Safety
///
/// `list` points to a live list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_any(list: *const ListObject) -> bool {
    // SAFETY: the caller passes a live list, whose first `length` items are initialised.
    unsafe {
        let kind = (*list).item_kind();
        items_of(list).iter().any(|&item| kind.is_true(item))
    }
}

/// Returns `all(list)`: whether every item is true.
///
/// # Safety
///
/// `list` points to a live list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_all(list: *const ListObject) -> bool {
    // SAFETY: the caller passes a live list, whose first `length` items are initialised.
    unsafe {
        let kind = (*list).item_kind();
        items_of(list).iter().all(|&item| kind.is_true(item))
    }
}

#[cfg(test)]
mod tests;
