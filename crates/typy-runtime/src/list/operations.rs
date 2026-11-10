//! The list functions that generated code calls. Every argument is borrowed, as for every
//! runtime function: a list that stores an item adds a reference to it, and an item read from a
//! list is borrowed from the list.

use super::{ItemType, ListObject, new_list, reserve_one};
use crate::Raising;
use crate::allocator::{allocate, array_size, free};
use crate::exception::{ExceptionKind, ExceptionObject, raise};
use crate::string::{StrKind, StrObject, code_point, length, new_string, set_code_point};
use crate::word::{inline, inline_value, is_inline};

/// Returns a new empty list of the item kind whose code is the `int` word `kind`, with room
/// for `capacity` items; `tuple` is the address of the descriptor of tuple items, or 0.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_list_new(kind: i64, capacity: i64, tuple: u64) -> *mut ListObject {
    let item_type = ItemType::of(
        u32::try_from(inline_value(kind)).unwrap_or(u32::MAX),
        core::ptr::with_exposed_provenance(usize::try_from(tuple).unwrap_or(0)),
    );
    new_list(
        item_type,
        usize::try_from(inline_value(capacity)).unwrap_or(0),
    )
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
        let item_type = (*list).item_type();
        item_type.retain(item);
        item_type.store((*list).slot((*list).length), item);
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
        let message = if (*list).is_tuple() {
            "tuple index out of range"
        } else {
            "list index out of range"
        };
        match position(index, (*list).length, message) {
            Ok(index) => Raising::ok((*list).item(index)),
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
                // The new item is counted first, so releasing the old one cannot free what the
                // new one holds.
                let item_type = (*list).item_type();
                item_type.retain(item);
                item_type.release((*list).item(index));
                item_type.store((*list).slot(index), item);
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
        let item_type = (*left).item_type();
        (0..(*left).length).all(|index| item_type.equal((*left).item(index), (*right).item(index)))
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
        let item_type = (*list).item_type();
        (0..(*list).length).any(|index| item_type.equal((*list).item(index), item))
    }
}

/// Returns `repr(list)`: the `repr` of each item, separated by `, ` between brackets, or for a
/// tuple between parentheses, with a comma after the only item of a one-item tuple.
///
/// # Safety
///
/// `list` points to a live list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_repr(list: *const ListObject) -> Raising<*mut StrObject> {
    // SAFETY: the caller passes a live list; every index stays below its length.
    unsafe {
        let item_type = (*list).item_type();
        let items = (0..(*list).length).map(|index| (item_type, (*list).item(index)));
        let (open, close) = match ((*list).is_tuple(), (*list).length) {
            (false, _) => ("[", "]"),
            (true, 1) => ("(", ",)"),
            (true, _) => ("(", ")"),
        };
        join_reprs(items, (*list).length, open, close)
    }
}

/// Returns the `repr` of `count` items, separated by `, ` between `open` and `close`, as one
/// new string as wide as its widest piece needs; each `repr` is made once. Only an `int` of more
/// than 4300 digits raises.
///
/// # Safety
///
/// `items` gives `count` live items of their types.
pub(crate) unsafe fn join_reprs(
    items: impl Iterator<Item = (ItemType, u64)>,
    count: usize,
    open: &str,
    close: &str,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller passes live items; each piece is a new string this function owns.
    unsafe {
        #[allow(
            clippy::cast_ptr_alignment,
            reason = "malloc returns memory aligned for every type"
        )]
        let pieces =
            allocate(array_size(count, core::mem::size_of::<usize>(), 0)).cast::<*mut StrObject>();
        for (index, (item_type, item)) in items.take(count).enumerate() {
            let piece = item_type.repr(item);
            if !piece.exception.is_null() {
                release_pieces(pieces, index);
                return Raising::error(core::ptr::null_mut(), piece.exception);
            }
            *pieces.add(index) = piece.value;
        }
        let text = join(pieces, count, open, close);
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

/// Returns `open piece, piece, ... close` as one new string, as wide as its widest piece needs.
///
/// # Safety
///
/// `pieces` holds `count` live strings.
unsafe fn join(
    pieces: *const *mut StrObject,
    count: usize,
    open: &str,
    close: &str,
) -> *mut StrObject {
    // SAFETY: the caller passes `count` live strings; every index stays below each length.
    unsafe {
        let separators = count.saturating_sub(1) * 2;
        let mut total = open.len() + close.len() + separators;
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
        open.bytes().for_each(|byte| put(u32::from(byte)));
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
        close.bytes().for_each(|byte| put(u32::from(byte)));
        text
    }
}
