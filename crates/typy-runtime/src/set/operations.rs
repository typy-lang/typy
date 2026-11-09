//! The set functions generated code calls (DAT-10): creating, adding, finding and removing
//! items, iterating, comparing, combining and writing a set as text.

use super::algebra::{copy, is_subset};
use super::{SetObject, add, discard, lookup, new_set, take};
use crate::Raising;
use crate::exception::{ExceptionKind, ExceptionObject, raise, raise_with_message};
use crate::list::{ItemType, join_reprs};
use crate::string::StrObject;
use crate::word::{inline, inline_value};

/// Returns the `int` word of a count or a position.
fn word(value: usize) -> i64 {
    inline(i64::try_from(value).unwrap_or(i64::MAX)).unwrap_or(1)
}

/// Returns a new empty set of the item kind whose code is the `int` word `kind`; `tuple` is
/// the address of the descriptor of tuple items, or 0.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_set_new(kind: i64, tuple: u64) -> *mut SetObject {
    let kind = u32::try_from(inline_value(kind)).unwrap_or(u32::MAX);
    let tuple = core::ptr::with_exposed_provenance(usize::try_from(tuple).unwrap_or(0));
    new_set(ItemType::of(kind, tuple))
}

/// Adds `item` unless an equal item is there, with a reference of its own (`set.add`).
///
/// # Safety
///
/// `set` points to a live set, and `item` is a live item of its type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_set_add(set: *mut SetObject, item: u64) {
    // SAFETY: the caller passes a live set and item.
    unsafe {
        let hash = (*set).item_type().hash(item);
        add(set, item, hash);
    }
}

/// Returns whether the set holds an item equal to `item` (`in`).
///
/// # Safety
///
/// `set` points to a live set, and `item` is a live item of its type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_set_contains(set: *const SetObject, item: u64) -> bool {
    // SAFETY: the caller passes a live set and item.
    unsafe { lookup(set, item, (*set).item_type().hash(item)).is_some() }
}

/// Removes an item equal to `item`, if there is one (`set.discard`).
///
/// # Safety
///
/// `set` points to a live set, and `item` is a live item of its type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_set_discard(set: *mut SetObject, item: u64) {
    // SAFETY: the caller passes a live set and item.
    unsafe {
        discard(set, item, (*set).item_type().hash(item));
    }
}

/// Returns a `KeyError` whose message is the `repr` of `item`, as CPython prints a missing key.
/// When the `repr` raises, as for an `int` of more than 4300 digits, the message is the one
/// CPython prints for an exception it cannot convert to text.
///
/// # Safety
///
/// `item` is a live item of `item_type`.
pub(crate) unsafe fn key_error(item_type: ItemType, item: u64) -> *mut ExceptionObject {
    // SAFETY: the caller passes a live item; the exception of a failed `repr` is owned here.
    unsafe {
        let text = item_type.repr(item);
        if text.exception.is_null() {
            return raise_with_message(ExceptionKind::KeyError, text.value);
        }
        crate::refcount::typy_rt_decrement(text.exception.cast());
        raise(ExceptionKind::KeyError, "<exception str() failed>")
    }
}

/// Removes an item equal to `item`, and returns the `KeyError` CPython raises when there is
/// none, or null (`set.remove`).
///
/// # Safety
///
/// `set` points to a live set, and `item` is a live item of its type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_set_remove(
    set: *mut SetObject,
    item: u64,
) -> *mut ExceptionObject {
    // SAFETY: the caller passes a live set and item.
    unsafe {
        let item_type = (*set).item_type();
        if discard(set, item, item_type.hash(item)) {
            core::ptr::null_mut()
        } else {
            key_error(item_type, item)
        }
    }
}

/// Removes and returns an item, whose references pass to the caller, from the slot at or after
/// the set's finger, as CPython's `set.pop` does; an empty set raises `KeyError`. A tuple is
/// returned as the address of its words, which stay valid until the set next changes.
///
/// # Safety
///
/// `set` points to a live set.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_set_pop(set: *mut SetObject) -> Raising<u64> {
    // SAFETY: the caller passes a live set; the search stays inside its table.
    unsafe {
        if (*set).used == 0 {
            return Raising::error(0, raise(ExceptionKind::KeyError, "'pop from an empty set'"));
        }
        let mask = (*set).mask;
        let mut slot = (*set).finger & mask;
        while (*set).state(slot) != super::ACTIVE {
            slot = if slot == mask { 0 } else { slot + 1 };
        }
        (*set).finger = slot + 1;
        Raising::ok(take(set, slot))
    }
}

/// Removes every item (`set.clear`).
///
/// # Safety
///
/// `set` points to a live set.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_set_clear(set: *mut SetObject) {
    // SAFETY: the caller passes a live set.
    unsafe { super::clear(set) }
}

/// Returns the number of items, as an `int` word (`len`).
///
/// # Safety
///
/// `set` points to a live set.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_set_length(set: *const SetObject) -> i64 {
    // SAFETY: the caller passes a live set.
    word(unsafe { (*set).used })
}

/// Returns the first active slot at or after the `int` word `position`, as an `int` word, or
/// -1 when there is none. A set whose number of items is no longer `length`, the number when
/// the loop began, raises `RuntimeError`, as CPython's set iterator does.
///
/// # Safety
///
/// `set` points to a live set, and `position` and `length` are `int` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_set_next(
    set: *const SetObject,
    position: i64,
    length: i64,
) -> Raising<i64> {
    // SAFETY: the caller passes a live set; the search stays inside its table.
    unsafe {
        let set = &*set;
        if word(set.used) != length {
            return Raising::error(
                0,
                raise(
                    ExceptionKind::RuntimeError,
                    "Set changed size during iteration",
                ),
            );
        }
        let start = usize::try_from(inline_value(position)).unwrap_or(0);
        let found = (start..=set.mask).find(|slot| set.state(*slot) == super::ACTIVE);
        Raising::ok(found.map_or_else(|| inline(-1).unwrap_or(1), word))
    }
}

/// Returns the item in the active slot whose position is the `int` word `slot`, borrowed from
/// the set.
///
/// # Safety
///
/// `set` points to a live set, and `slot` is an active slot that `typy_rt_set_next` returned.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_set_item(set: *const SetObject, slot: i64) -> u64 {
    // SAFETY: the caller's guarantee.
    unsafe { (*set).item(usize::try_from(inline_value(slot)).unwrap_or(0)) }
}

/// Returns `repr(set)`: the `repr` of each item, in table order, between braces, or `set()` for
/// an empty set. Only an `int` of more than 4300 digits raises.
///
/// # Safety
///
/// `set` points to a live set.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_set_repr(set: *const SetObject) -> Raising<*mut StrObject> {
    // SAFETY: the caller passes a live set; each active slot holds a live item.
    unsafe {
        let set = &*set;
        if set.used == 0 {
            return Raising::ok(crate::string::from_str("set()"));
        }
        let item_type = set.item_type();
        let items = set.active().map(|slot| (item_type, set.item(slot)));
        join_reprs(items, set.used, "{", "}")
    }
}

/// Returns a new set with the same items (`set.copy`, `set(s)`).
///
/// # Safety
///
/// `set` points to a live set.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_set_copy(set: *const SetObject) -> *mut SetObject {
    // SAFETY: the caller passes a live set.
    unsafe { copy(set) }
}

/// Returns whether two sets stand in the comparison whose code is the `int` word `comparison`:
/// equal for `==`, a subset for `<=` and a proper one for `<`, a superset for `>=` and `>`.
///
/// # Safety
///
/// Both point to live sets of one item type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_set_compare(
    left: *const SetObject,
    right: *const SetObject,
    comparison: i64,
) -> bool {
    // SAFETY: the caller passes two live sets.
    unsafe {
        let (left_size, right_size) = ((*left).used, (*right).used);
        match inline_value(comparison) {
            0 => left_size == right_size && is_subset(left, right),
            1 => !(left_size == right_size && is_subset(left, right)),
            2 => left_size < right_size && is_subset(left, right),
            3 => is_subset(left, right),
            4 => left_size > right_size && is_subset(right, left),
            _ => is_subset(right, left),
        }
    }
}
