//! The list functions that generated code calls. Every argument is borrowed, as for every
//! runtime function: a list that stores an item adds a reference to it, and an item read from a
//! list is borrowed from the list.

use super::{ItemKind, ListObject, new_list, reserve_one};
use crate::Raising;
use crate::allocator::{allocate, array_size, free};
use crate::exception::{ExceptionKind, ExceptionObject, raise};
use crate::string::{StrKind, StrObject, code_point, length, new_string, set_code_point};
use crate::word::{inline, inline_value, is_inline};

/// Returns a new empty list of the item kind whose code is the `int` word `kind`, with room
/// for `capacity` items.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_list_new(kind: i64, capacity: i64) -> *mut ListObject {
    let kind = ItemKind::from_code(u32::try_from(inline_value(kind)).unwrap_or(u32::MAX));
    new_list(kind, usize::try_from(inline_value(capacity)).unwrap_or(0))
}

/// Adds `item` at the end of `list`, with a reference of its own (`list.append`).
///
/// # Safety
///
/// `list` points to a live list, and `item` is a live item of its kind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_append(list: *mut ListObject, item: u64) {
    // SAFETY: the caller passes a live list and item; `reserve_one` makes room for it.
    unsafe {
        reserve_one(list);
        (*list).item_kind().retain(item);
        *(*list).items.add((*list).length) = item;
        (*list).length += 1;
    }
}

/// Returns the number of items, as an `int` word (`len`).
///
/// # Safety
///
/// `list` points to a live list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_length(list: *const ListObject) -> i64 {
    // SAFETY: the caller passes a live list.
    let length = unsafe { (*list).length };
    inline(i64::try_from(length).unwrap_or(i64::MAX)).unwrap_or(1)
}

/// Returns the position an `int` word names in a list of `length` items, counting a negative
/// one from the end, or the `IndexError` CPython raises with `message` (DAT-11).
fn position(index: i64, length: usize, message: &str) -> Result<usize, *mut ExceptionObject> {
    if !is_inline(index) {
        return Err(raise(
            ExceptionKind::IndexError,
            "cannot fit 'int' into an index-sized integer",
        ));
    }
    let length = i64::try_from(length).unwrap_or(i64::MAX);
    let index = inline_value(index);
    let from_start = if index < 0 { index + length } else { index };
    if (0..length).contains(&from_start) {
        Ok(usize::try_from(from_start).unwrap_or(0))
    } else {
        Err(raise(ExceptionKind::IndexError, message))
    }
}

/// Returns `list[index]`, borrowed from the list.
///
/// # Safety
///
/// `list` points to a live list, and `index` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_get(list: *const ListObject, index: i64) -> Raising<u64> {
    // SAFETY: the caller passes a live list; `position` keeps the index inside it.
    unsafe {
        match position(index, (*list).length, "list index out of range") {
            Ok(index) => Raising::ok(*(*list).items.add(index)),
            Err(exception) => Raising::error(0, exception),
        }
    }
}

/// Stores `item` at `list[index]`, with a reference of its own, and releases the item it
/// replaces; returns the exception, or null.
///
/// # Safety
///
/// `list` points to a live list, `index` is an `int` word, and `item` is a live item of the
/// list's kind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_set(
    list: *mut ListObject,
    index: i64,
    item: u64,
) -> *mut ExceptionObject {
    // SAFETY: the caller passes a live list and item; `position` keeps the index inside it.
    unsafe {
        match position(index, (*list).length, "list assignment index out of range") {
            Ok(index) => {
                let kind = (*list).item_kind();
                kind.retain(item);
                let slot = (*list).items.add(index);
                let old = *slot;
                *slot = item;
                kind.release(old);
                core::ptr::null_mut()
            }
            Err(exception) => exception,
        }
    }
}

/// Returns `left == right`: the same length, and items equal one by one.
///
/// # Safety
///
/// Both point to live lists of one item kind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_equal(
    left: *const ListObject,
    right: *const ListObject,
) -> bool {
    // SAFETY: the caller passes live lists; every index stays below both lengths.
    unsafe {
        if (*left).length != (*right).length {
            return false;
        }
        let kind = (*left).item_kind();
        (0..(*left).length)
            .all(|index| kind.equal(*(*left).items.add(index), *(*right).items.add(index)))
    }
}

/// Returns `item in list`: whether an item is equal to it.
///
/// # Safety
///
/// `list` points to a live list, and `item` is a live item of its kind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_contains(list: *const ListObject, item: u64) -> bool {
    // SAFETY: the caller passes a live list and item; every index stays below its length.
    unsafe {
        let kind = (*list).item_kind();
        (0..(*list).length).any(|index| kind.equal(*(*list).items.add(index), item))
    }
}

/// Returns `repr(list)`: the `repr` of each item, separated by `, ` between brackets.
///
/// # Safety
///
/// `list` points to a live list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_repr(list: *const ListObject) -> Raising<*mut StrObject> {
    // SAFETY: the caller passes a live list; each piece is a new string this function owns.
    unsafe {
        let count = (*list).length;
        let kind = (*list).item_kind();
        #[allow(
            clippy::cast_ptr_alignment,
            reason = "malloc returns memory aligned for every type"
        )]
        let pieces =
            allocate(array_size(count, core::mem::size_of::<usize>(), 0)).cast::<*mut StrObject>();
        for index in 0..count {
            let piece = kind.repr(*(*list).items.add(index));
            if !piece.exception.is_null() {
                release_pieces(pieces, index);
                return Raising::error(core::ptr::null_mut(), piece.exception);
            }
            *pieces.add(index) = piece.value;
        }
        let text = join(pieces, count);
        release_pieces(pieces, count);
        Raising::ok(text)
    }
}

/// Releases the first `count` pieces and frees the array that holds them.
///
/// # Safety
///
/// `pieces` holds `count` live strings that the caller owns.
unsafe fn release_pieces(pieces: *mut *mut StrObject, count: usize) {
    // SAFETY: the caller passes `count` live strings it owns.
    unsafe {
        for index in 0..count {
            crate::refcount::typy_rt_decrement((*pieces.add(index)).cast());
        }
    }
    free(pieces.cast::<u8>());
}

/// Returns `[piece, piece, ...]` as one new string, as wide as its widest piece needs.
///
/// # Safety
///
/// `pieces` holds `count` live strings.
unsafe fn join(pieces: *const *mut StrObject, count: usize) -> *mut StrObject {
    // SAFETY: the caller passes `count` live strings; every index stays below each length.
    unsafe {
        let separators = count.saturating_sub(1) * 2;
        let mut total = 2 + separators;
        let mut kind = StrKind::Latin1;
        for index in 0..count {
            let piece = *pieces.add(index);
            total += length(piece);
            kind = kind.max((*piece).kind);
        }
        let text = new_string(total, kind);
        let mut at = 0;
        let mut put = |point: u32| {
            set_code_point(text, at, point);
            at += 1;
        };
        put(u32::from(b'['));
        for index in 0..count {
            if index > 0 {
                put(u32::from(b','));
                put(u32::from(b' '));
            }
            let piece = *pieces.add(index);
            for position in 0..length(piece) {
                put(code_point(piece, position));
            }
        }
        put(u32::from(b']'));
        text
    }
}
