//! `del` of a list item and of a list slice (SMT-05), and assignment to a list slice (DAT-13),
//! with CPython's results and messages.

use super::{ListObject, reserve, take_out};
use crate::exception::ExceptionObject;
use crate::slice::select;

/// Removes the item at `index`, counted from the end when negative, and releases it, as
/// `del list[index]` does; an index outside the list raises `IndexError: list assignment
/// index out of range`.
///
/// # Safety
///
/// `list` points to a live list, and `index` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_delete(
    list: *mut ListObject,
    index: i64,
) -> *mut ExceptionObject {
    // SAFETY: the caller passes a live list; `position` keeps the index inside it.
    unsafe {
        match super::operations::position(
            index,
            (*list).length,
            "list assignment index out of range",
        ) {
            Ok(position) => {
                let item = take_out(list, position);
                (*list).item_type().release(item);
                core::ptr::null_mut()
            }
            Err(error) => error,
        }
    }
}

/// Removes the items `list[start:stop:step]` selects and releases them, as `del` of a slice
/// does; `present` says which bounds are given, and a zero step raises `ValueError`.
///
/// # Safety
///
/// `list` points to a live list, and the bounds are `int` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_delete_slice(
    list: *mut ListObject,
    start: i64,
    stop: i64,
    stride: i64,
    present: i64,
) -> *mut ExceptionObject {
    // SAFETY: the caller passes a live list; every position read or written is inside it.
    unsafe {
        let length = (*list).length;
        let selection = match select(start, stop, stride, present, length) {
            Ok(selection) => selection,
            Err(error) => return error,
        };
        if selection.count == 0 {
            return core::ptr::null_mut();
        }
        // The selected positions, lowest first, are those from `lowest` every `gap`.
        let final_position = selection.position(selection.count - 1);
        let lowest = selection.position(0).min(final_position);
        let highest = selection.position(0).max(final_position);
        let gap = usize::try_from(selection.step.unsigned_abs()).unwrap_or(1);
        let removed = |index: usize| {
            (lowest..=highest).contains(&index) && (index - lowest).is_multiple_of(gap)
        };
        let item_type = (*list).item_type();
        let words = (*list).words();
        let mut kept = 0;
        for index in 0..length {
            if removed(index) {
                item_type.release((*list).item(index));
            } else {
                if kept != index {
                    core::ptr::copy((*list).slot(index), (*list).slot(kept), words);
                }
                kept += 1;
            }
        }
        (*list).length = kept;
        core::ptr::null_mut()
    }
}

/// Replaces what `list[start:stop:step]` selects with the items of `source`, as assigning to
/// a slice does (DAT-13). A slice without a step, or with a step of 1, takes any number of
/// items, and the list grows or shrinks; any other step takes exactly as many items as it
/// selects, or raises `ValueError`. The source's items are counted before the list changes,
/// so a list may be assigned to a slice of itself.
///
/// # Safety
///
/// `list` and `source` point to live lists of one item type, and the bounds are `int` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_assign_slice(
    list: *mut ListObject,
    start: i64,
    stop: i64,
    stride: i64,
    present: i64,
    source: *const ListObject,
) -> *mut ExceptionObject {
    // SAFETY: the caller passes live lists; every position read or written is inside them.
    unsafe {
        let selection = match select(start, stop, stride, present, (*list).length) {
            Ok(selection) => selection,
            Err(error) => return error,
        };
        let count = (*source).length;
        let words = (*list).words();
        let item_type = (*list).item_type();
        // The new items, each with a reference of its own, copied before the list changes.
        let size = crate::allocator::array_size(count.max(1), 8 * words, 0);
        #[allow(
            clippy::cast_ptr_alignment,
            reason = "malloc returns memory aligned for every type"
        )]
        let incoming = crate::allocator::allocate(size).cast::<u64>();
        for index in 0..count {
            item_type.retain((*source).item(index));
            core::ptr::copy_nonoverlapping(
                (*source).slot(index),
                incoming.add(index * words),
                words,
            );
        }
        if selection.step == 1 {
            replace_run(
                list,
                selection.start_position(),
                selection.count,
                incoming,
                count,
            );
        } else if count != selection.count {
            for index in 0..count {
                item_type.release(item_at(incoming, index, words, item_type));
            }
            crate::allocator::free(incoming.cast());
            let mut message = crate::message::MessageBuffer::new();
            message.push_str("attempt to assign sequence of size ");
            message.push_number(i64::try_from(count).unwrap_or(i64::MAX));
            message.push_str(" to extended slice of size ");
            message.push_number(i64::try_from(selection.count).unwrap_or(i64::MAX));
            return message.raise(crate::ExceptionKind::ValueError);
        } else {
            for index in 0..count {
                let slot = (*list).slot(selection.position(index));
                item_type.release((*list).item(selection.position(index)));
                core::ptr::copy_nonoverlapping(incoming.add(index * words), slot, words);
            }
        }
        crate::allocator::free(incoming.cast());
        core::ptr::null_mut()
    }
}

/// Returns item `index` of a run of items laid out as a list stores them.
///
/// # Safety
///
/// The run holds more than `index` items of `item_type`.
unsafe fn item_at(run: *mut u64, index: usize, words: usize, item_type: super::ItemType) -> u64 {
    // SAFETY: the caller's guarantee.
    unsafe { item_type.load(run.add(index * words)) }
}

/// Replaces the `removed` items from `start` with the `count` items of `incoming`, which hold
/// their own references, moving the rest of the list to fit.
///
/// # Safety
///
/// `list` is live, `start + removed` is within it, and `incoming` holds `count` items of its type.
unsafe fn replace_run(
    list: *mut ListObject,
    start: usize,
    removed: usize,
    incoming: *mut u64,
    count: usize,
) {
    // SAFETY: the caller's guarantee; `reserve` makes room before the tail moves right.
    unsafe {
        let item_type = (*list).item_type();
        let words = (*list).words();
        for index in start..start + removed {
            item_type.release((*list).item(index));
        }
        let length = (*list).length;
        let tail = length - start - removed;
        if count > removed {
            reserve(list, count - removed);
        }
        core::ptr::copy(
            (*list).slot(start + removed),
            (*list).slot(start + count),
            tail * words,
        );
        core::ptr::copy_nonoverlapping(incoming, (*list).slot(start), count * words);
        (*list).length = length - removed + count;
    }
}

#[cfg(test)]
mod tests;
