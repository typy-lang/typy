//! Functions over every item of a list: sorting, `min`, `max`, `sum`, `any` and `all`
//! (LIB-02, LIB-03).

use super::ListObject;
use crate::Raising;
use crate::allocator::{allocate, array_size, free};
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
        let reverse = inline_value(reverse) != 0;
        if reverse {
            super::methods::typy_rt_list_reverse(list);
        }
        let item_type = (*list).item_type();
        if item_type.kind() == super::ItemKind::Tuple {
            sort_tuples(list);
        } else {
            let items = core::slice::from_raw_parts_mut((*list).items, length);
            sort(items, &|left: u64, right: u64| {
                item_type.order(left, right, LESS)
            });
        }
        if reverse {
            super::methods::typy_rt_list_reverse(list);
        }
    }
}

/// Sorts a list of tuples, which are compared through their addresses: the positions are sorted
/// by the items they hold, then the items are copied into that order through one buffer.
///
/// # Safety
///
/// `list` points to a live list of at least two items.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "malloc returns memory aligned for every type"
)]
unsafe fn sort_tuples(list: *mut ListObject) {
    // SAFETY: the caller passes a live list; every position stays below its length.
    unsafe {
        let (length, words) = ((*list).length, (*list).words());
        let item_type = (*list).item_type();
        let order = allocate(array_size(length, core::mem::size_of::<usize>(), 0)).cast::<usize>();
        let positions = core::slice::from_raw_parts_mut(order, length);
        for (index, position) in positions.iter_mut().enumerate() {
            *position = index;
        }
        sort(positions, &|left: usize, right: usize| {
            item_type.order((*list).item(left), (*list).item(right), LESS)
        });
        let sorted = allocate(array_size(length, 8 * words, 0)).cast::<u64>();
        for (index, position) in positions.iter().enumerate() {
            core::ptr::copy_nonoverlapping(
                (*list).slot(*position),
                sorted.add(index * words),
                words,
            );
        }
        core::ptr::copy_nonoverlapping(sorted, (*list).items, length * words);
        free(sorted.cast());
        free(order.cast());
    }
}

/// Returns the items of a list of eight-byte items; an empty list may have no buffer at all.
///
/// # Safety
///
/// `list` points to a live list of eight-byte items, which outlives the slice and is not
/// changed while it is used.
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
fn sort<T: Copy>(items: &mut [T], less: &impl Fn(T, T) -> bool) {
    for run in items.chunks_mut(RUN) {
        insertion_sort(run, less);
    }
    if items.len() <= RUN {
        return;
    }
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "malloc returns memory aligned for every type"
    )]
    let buffer = allocate(array_size(items.len(), core::mem::size_of::<T>(), 0)).cast::<T>();
    // SAFETY: the buffer has room for every item; `merge` writes each slot before reading it.
    let scratch = unsafe { core::slice::from_raw_parts_mut(buffer, items.len()) };
    let mut width = RUN;
    while width < items.len() {
        let mut start = 0;
        while start + width < items.len() {
            let end = (start + 2 * width).min(items.len());
            merge(&mut items[start..end], width, scratch, less);
            start = end;
        }
        width *= 2;
    }
    free(buffer.cast());
}

/// Sorts a short run by binary insertion; an item goes after every item it is not less than.
fn insertion_sort<T: Copy>(run: &mut [T], less: &impl Fn(T, T) -> bool) {
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
fn merge<T: Copy>(items: &mut [T], middle: usize, scratch: &mut [T], less: &impl Fn(T, T) -> bool) {
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
/// `ValueError`. A tuple is returned as its address in the list, which the caller copies.
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
        let item_type = (*list).item_type();
        if (*list).length == 0 {
            let message = if largest {
                "max() iterable argument is empty"
            } else {
                "min() iterable argument is empty"
            };
            return Raising::error(0, raise(ExceptionKind::ValueError, message));
        }
        let comparison = if largest { GREATER } else { LESS };
        let kept = (1..(*list).length).fold((*list).item(0), |kept, index| {
            let item = (*list).item(index);
            if item_type.order(item, kept, comparison) {
                item
            } else {
                kept
            }
        });
        item_type.retain(kept);
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
        let item_type = (*list).item_type();
        (0..(*list).length).any(|index| item_type.is_true((*list).item(index)))
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
        let item_type = (*list).item_type();
        (0..(*list).length).all(|index| item_type.is_true((*list).item(index)))
    }
}

#[cfg(test)]
mod tests;
