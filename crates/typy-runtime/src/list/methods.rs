//! The methods of `list` (LIB-03), and slicing (DAT-12), with CPython's results and messages.

use super::{ListObject, new_list, reserve_one};
use crate::Raising;
use crate::exception::{ExceptionKind, ExceptionObject, raise};
use crate::slice::select;
use crate::word::{inline, inline_value, is_inline};

/// Returns `list[start:stop:step]`, a new list (DAT-12); `present` says which bounds are given.
///
/// # Safety
///
/// `list` points to a live list, and the bounds are `int` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_slice(
    list: *const ListObject,
    start: i64,
    stop: i64,
    stride: i64,
    present: i64,
) -> Raising<*mut ListObject> {
    // SAFETY: the caller passes a live list; every selected position is inside it.
    unsafe {
        let selection = match select(start, stop, stride, present, (*list).length) {
            Ok(selection) => selection,
            Err(error) => return Raising::error(core::ptr::null_mut(), error),
        };
        let kind = (*list).item_kind();
        let result = new_list(kind, selection.count);
        for index in 0..selection.count {
            let item = *(*list).items.add(selection.position(index));
            kind.retain(item);
            *(*result).items.add(index) = item;
        }
        (*result).length = selection.count;
        Raising::ok(result)
    }
}

/// Removes the item at `index`, or the last item when `present`, an `int` word, is 0, and
/// returns it with the reference the list held, which the caller now owns (`list.pop`). The
/// flag is a word rather than a `bool`, so no `bool` crosses into the runtime as an argument.
///
/// # Safety
///
/// `list` points to a live list, and `index` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_pop(
    list: *mut ListObject,
    index: i64,
    present: i64,
) -> Raising<u64> {
    // SAFETY: the caller passes a live list; the position is checked against its length.
    unsafe {
        let length = (*list).length;
        if length == 0 {
            return Raising::error(0, raise(ExceptionKind::IndexError, "pop from empty list"));
        }
        let length_word = i64::try_from(length).unwrap_or(i64::MAX);
        let position = if inline_value(present) == 0 {
            Some(length_word - 1)
        } else if is_inline(index) {
            let index = inline_value(index);
            Some(if index < 0 {
                index + length_word
            } else {
                index
            })
        } else {
            None
        };
        let Some(position) = position
            .filter(|position| (0..length_word).contains(position))
            .and_then(|position| usize::try_from(position).ok())
        else {
            return Raising::error(
                0,
                raise(ExceptionKind::IndexError, "pop index out of range"),
            );
        };
        let items = (*list).items;
        let item = *items.add(position);
        core::ptr::copy(
            items.add(position + 1),
            items.add(position),
            length - position - 1,
        );
        (*list).length = length - 1;
        Raising::ok(item)
    }
}

/// Inserts `item` before position `index`, clamped to the list, as `list.insert` does.
///
/// # Safety
///
/// `list` points to a live list, `index` is an `int` word, and `item` is a live item of its kind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_insert(list: *mut ListObject, index: i64, item: u64) {
    // SAFETY: the caller passes a live list and item; `reserve_one` makes room.
    unsafe {
        let length = i64::try_from((*list).length).unwrap_or(i64::MAX);
        let index = if is_inline(index) {
            inline_value(index)
        } else if crate::int::is_negative(index) {
            i64::MIN
        } else {
            i64::MAX
        };
        let position = if index < 0 {
            (index.saturating_add(length)).max(0)
        } else {
            index.min(length)
        };
        let position = usize::try_from(position).unwrap_or(0);
        reserve_one(list);
        (*list).item_kind().retain(item);
        let items = (*list).items;
        core::ptr::copy(
            items.add(position),
            items.add(position + 1),
            (*list).length - position,
        );
        *items.add(position) = item;
        (*list).length += 1;
    }
}

/// Appends the items of `other` to `list`, as `list.extend` and `list += other` do. The count is
/// read first, so a list extended by itself doubles once.
///
/// # Safety
///
/// Both point to live lists of one item kind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_extend(list: *mut ListObject, other: *const ListObject) {
    // SAFETY: the caller passes live lists; `count` items of `other` exist throughout.
    unsafe {
        let count = (*other).length;
        for index in 0..count {
            super::typy_rt_list_append(list, *(*other).items.add(index));
        }
    }
}

/// Returns the position of the first item equal to `item`, or `None`.
///
/// # Safety
///
/// `list` points to a live list, and `item` is a live item of its kind.
unsafe fn find(list: *const ListObject, item: u64) -> Option<usize> {
    // SAFETY: the caller passes a live list and item.
    unsafe {
        let kind = (*list).item_kind();
        (0..(*list).length).find(|index| kind.equal(*(*list).items.add(*index), item))
    }
}

/// Removes the first item equal to `item`, and returns the exception, or null (`list.remove`).
///
/// # Safety
///
/// `list` points to a live list, and `item` is a live item of its kind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_remove(
    list: *mut ListObject,
    item: u64,
) -> *mut ExceptionObject {
    // SAFETY: the caller passes a live list and item; `find` returns a position inside it.
    unsafe {
        let Some(position) = find(list, item) else {
            return raise(ExceptionKind::ValueError, "list.remove(x): x not in list");
        };
        let items = (*list).items;
        let removed = *items.add(position);
        core::ptr::copy(
            items.add(position + 1),
            items.add(position),
            (*list).length - position - 1,
        );
        (*list).length -= 1;
        (*list).item_kind().release(removed);
        core::ptr::null_mut()
    }
}

/// Returns the position of the first item equal to `item`, as an `int` word (`list.index`). The
/// message is CPython 3.14's, `list.index(x): x not in list`, the wording of the newest oracle
/// (OI-14); CPython 3.12 and 3.13 name the item instead.
///
/// # Safety
///
/// `list` points to a live list, and `item` is a live item of its kind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_index(list: *const ListObject, item: u64) -> Raising<i64> {
    // SAFETY: the caller passes a live list and item.
    unsafe {
        if let Some(position) = find(list, item) {
            return Raising::ok(inline(i64::try_from(position).unwrap_or(0)).unwrap_or(1));
        }
        Raising::error(
            1,
            raise(ExceptionKind::ValueError, "list.index(x): x not in list"),
        )
    }
}

/// Returns the number of items equal to `item`, as an `int` word (`list.count`).
///
/// # Safety
///
/// `list` points to a live list, and `item` is a live item of its kind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_count(list: *const ListObject, item: u64) -> i64 {
    // SAFETY: the caller passes a live list and item.
    let count = unsafe {
        let kind = (*list).item_kind();
        (0..(*list).length)
            .filter(|index| kind.equal(*(*list).items.add(*index), item))
            .count()
    };
    inline(i64::try_from(count).unwrap_or(i64::MAX)).unwrap_or(1)
}

/// Removes every item (`list.clear`).
///
/// # Safety
///
/// `list` points to a live list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_clear(list: *mut ListObject) {
    // SAFETY: the caller passes a live list. Its length is set to zero before the items are
    // released, so code an item's release runs cannot see them.
    unsafe {
        let length = (*list).length;
        (*list).length = 0;
        let kind = (*list).item_kind();
        for index in 0..length {
            kind.release(*(*list).items.add(index));
        }
    }
}

/// Returns a new list of the same items (`list.copy`).
///
/// # Safety
///
/// `list` points to a live list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_copy(list: *const ListObject) -> *mut ListObject {
    // SAFETY: the caller passes a live list.
    unsafe {
        let copy = new_list((*list).item_kind(), (*list).length);
        typy_rt_list_extend(copy, list);
        copy
    }
}

/// Reverses the items in place (`list.reverse`).
///
/// # Safety
///
/// `list` points to a live list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_reverse(list: *mut ListObject) {
    // SAFETY: the caller passes a live list, whose first `length` items are initialised.
    unsafe {
        if (*list).length > 1 {
            core::slice::from_raw_parts_mut((*list).items, (*list).length).reverse();
        }
    }
}
