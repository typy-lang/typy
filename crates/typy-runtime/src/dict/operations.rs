//! The dict functions generated code calls (DAT-09): creating, reading and setting entries,
//! removing them, iterating, merging, comparing and copying.

use super::{
    DictObject, clear, find, insert, log2_for, new_dict, remove, resize, take_last, usable_entries,
};
use crate::Raising;
use crate::allocator::{allocate, array_size};
use crate::exception::{ExceptionKind, raise};
use crate::header::DICT_TYPE_ID;
use crate::list::{ItemType, ListObject};
use crate::word::{inline, inline_value};

/// Returns the `int` word of a count or a position.
fn word(value: usize) -> i64 {
    inline(i64::try_from(value).unwrap_or(i64::MAX)).unwrap_or(1)
}

/// Returns the position an `int` word holds.
fn position(word: i64) -> usize {
    usize::try_from(inline_value(word)).unwrap_or(0)
}

/// Returns the item type whose kind has the code in the `int` word `kind`, with the descriptor
/// at `tuple` for tuples.
fn item_type(kind: i64, tuple: u64) -> ItemType {
    let tuple = core::ptr::with_exposed_provenance(usize::try_from(tuple).unwrap_or(0));
    ItemType::of(u32::try_from(inline_value(kind)).unwrap_or(u32::MAX), tuple)
}

/// Returns a new empty dict of the key kind and value kind whose codes are the `int` words
/// `key_kind` and `value_kind`, with their tuple descriptors or 0, and a table for `capacity`
/// entries, an `int` word, when it is not 0.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_dict_new(
    key_kind: i64,
    key_tuple: u64,
    value_kind: i64,
    value_tuple: u64,
    capacity: i64,
) -> *mut DictObject {
    new_dict(
        item_type(key_kind, key_tuple),
        item_type(value_kind, value_tuple),
        position(capacity),
    )
}

/// Sets `dict[key] = value`; the dict takes references of its own.
///
/// # Safety
///
/// `dict` points to a live dict, and `key` and `value` are live items of its types.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_set(dict: *mut DictObject, key: u64, value: u64) {
    // SAFETY: the caller's guarantee.
    unsafe { insert(dict, key, (*dict).key_type().hash(key), value) }
}

/// Returns the position of the entry of `key`, as an `int` word, or -1.
///
/// # Safety
///
/// `dict` points to a live dict, and `key` is a live key of its type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_find(dict: *const DictObject, key: u64) -> i64 {
    // SAFETY: the caller's guarantee.
    unsafe {
        let hash = (*dict).key_type().hash(key);
        find(dict, key, hash).map_or_else(|| inline(-1).unwrap_or(1), word)
    }
}

/// Returns whether the dict has `key` (`in`).
///
/// # Safety
///
/// As for `typy_rt_dict_find`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_contains(dict: *const DictObject, key: u64) -> bool {
    // SAFETY: the caller's guarantee.
    unsafe { find(dict, key, (*dict).key_type().hash(key)).is_some() }
}

/// Returns `dict[key]`, borrowed from the dict, or the `KeyError` CPython raises with the
/// `repr` of the key.
///
/// # Safety
///
/// As for `typy_rt_dict_find`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_get(dict: *const DictObject, key: u64) -> Raising<u64> {
    // SAFETY: the caller's guarantee.
    unsafe {
        let key_type = (*dict).key_type();
        match find(dict, key, key_type.hash(key)) {
            Some(position) => Raising::ok((*dict).value(position)),
            None => Raising::error(0, crate::set::key_error(key_type, key)),
        }
    }
}

/// Returns `dict.get(key, default)`: the value of `key`, borrowed from the dict, or `default`.
///
/// # Safety
///
/// As for `typy_rt_dict_find`, and `default` is a live value of the dict's value type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_get_or(
    dict: *const DictObject,
    key: u64,
    default: u64,
) -> u64 {
    // SAFETY: the caller's guarantee.
    unsafe {
        let hash = (*dict).key_type().hash(key);
        find(dict, key, hash).map_or(default, |position| (*dict).value(position))
    }
}

/// Returns the key of the entry at the `int` word `position`, borrowed from the dict.
///
/// # Safety
///
/// `dict` points to a live dict, and `position` is an entry `typy_rt_dict_find`,
/// `typy_rt_dict_next` or `typy_rt_dict_popitem` returned, before the dict changed again.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_key(dict: *const DictObject, position: i64) -> u64 {
    // SAFETY: the caller's guarantee.
    unsafe { (*dict).key(self::position(position)) }
}

/// Returns the value of the entry at the `int` word `position`, as `typy_rt_dict_key` does.
///
/// # Safety
///
/// As for `typy_rt_dict_key`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_value(dict: *const DictObject, position: i64) -> u64 {
    // SAFETY: the caller's guarantee.
    unsafe { (*dict).value(self::position(position)) }
}

/// Removes the entry of `key` and returns its value, whose reference passes to the caller, or
/// raises `KeyError` (`dict.pop(key)`).
///
/// # Safety
///
/// As for `typy_rt_dict_find`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_pop(dict: *mut DictObject, key: u64) -> Raising<u64> {
    // SAFETY: the caller's guarantee; the removed key is released here.
    unsafe {
        let key_type = (*dict).key_type();
        match remove(dict, key, key_type.hash(key)) {
            Some(position) => {
                key_type.release((*dict).key(position));
                Raising::ok((*dict).value(position))
            }
            None => Raising::error(0, crate::set::key_error(key_type, key)),
        }
    }
}

/// Removes the entry of `key` and returns its value, or `default` with a reference of its own
/// when there is none (`dict.pop(key, default)`).
///
/// # Safety
///
/// As for `typy_rt_dict_find`, and `default` is a live value of the dict's value type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_pop_or(dict: *mut DictObject, key: u64, default: u64) -> u64 {
    // SAFETY: the caller's guarantee.
    unsafe {
        if typy_rt_dict_contains(dict, key) {
            return typy_rt_dict_pop(dict, key).value;
        }
        (*dict).value_type().retain(default);
        default
    }
}

/// Removes the last entry and returns its position as an `int` word; its key and value, with
/// their references, pass to the caller. An empty dict raises CPython's `KeyError`.
///
/// # Safety
///
/// `dict` points to a live dict.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_popitem(dict: *mut DictObject) -> Raising<i64> {
    // SAFETY: the caller's guarantee.
    match unsafe { take_last(dict) } {
        Some(position) => Raising::ok(word(position)),
        None => Raising::error(
            0,
            raise(ExceptionKind::KeyError, "'popitem(): dictionary is empty'"),
        ),
    }
}

/// Returns the value of `key`, borrowed, after adding `key` with `default` when it is missing
/// (`dict.setdefault`).
///
/// # Safety
///
/// As for `typy_rt_dict_pop_or`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_setdefault(
    dict: *mut DictObject,
    key: u64,
    default: u64,
) -> u64 {
    // SAFETY: the caller's guarantee.
    unsafe {
        let hash = (*dict).key_type().hash(key);
        if find(dict, key, hash).is_none() {
            insert(dict, key, hash, default);
        }
        (*dict).value(find(dict, key, hash).unwrap_or(0))
    }
}

/// Returns the number of entries, as an `int` word (`len`).
///
/// # Safety
///
/// `dict` points to a live dict.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_length(dict: *const DictObject) -> i64 {
    // SAFETY: the caller's guarantee.
    word(unsafe { (*dict).used })
}

/// Removes every entry (`dict.clear`).
///
/// # Safety
///
/// `dict` points to a live dict.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_clear(dict: *mut DictObject) {
    // SAFETY: the caller's guarantee.
    unsafe { clear(dict) }
}

/// Returns the next entry at or after the `int` word `position` that holds an item, as an
/// `int` word, or -1 at the end, as CPython's dict iterator does: a dict whose number of
/// entries is no longer `length` raises `RuntimeError`, and so does an entry found after
/// `visited` reached `length`, which only a change of keys of the same size makes possible.
///
/// # Safety
///
/// `dict` points to a live dict, and the others are `int` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_next(
    dict: *const DictObject,
    position: i64,
    length: i64,
    visited: i64,
) -> Raising<i64> {
    // SAFETY: the caller's guarantee; the search stays below `filled`.
    unsafe {
        let dict = &*dict;
        if word(dict.used) != length {
            return Raising::error(
                0,
                raise(
                    ExceptionKind::RuntimeError,
                    "dictionary changed size during iteration",
                ),
            );
        }
        let start = self::position(position);
        let Some(found) = (start..dict.filled).find(|index| dict.is_live(*index)) else {
            return Raising::ok(inline(-1).unwrap_or(1));
        };
        if visited == length {
            return Raising::error(
                0,
                raise(
                    ExceptionKind::RuntimeError,
                    "dictionary keys changed during iteration",
                ),
            );
        }
        Raising::ok(word(found))
    }
}

/// Returns a dict with the same table as `source`, holes and all, each key and value with a
/// reference of its own, CPython's `clone_combined_dict_keys`.
///
/// # Safety
///
/// `source` points to a live dict with a table.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "malloc returns memory aligned for every type"
)]
unsafe fn clone_table(target: &mut DictObject, source: &DictObject) {
    let size = 1_usize << source.log2;
    let capacity = usable_entries(source.log2);
    let stride = source.stride();
    let (key_type, value_type) = (source.key_type(), source.value_type());
    // SAFETY: the new buffers are as large as the source's; each live entry is counted again.
    unsafe {
        let indices = allocate(array_size(size, 8, 0)).cast::<i64>();
        let entries = allocate(array_size(capacity, 8 * stride, 0)).cast::<u64>();
        let live = allocate(capacity);
        core::ptr::copy_nonoverlapping(source.indices, indices, size);
        core::ptr::copy_nonoverlapping(source.entries, entries, capacity * stride);
        core::ptr::copy_nonoverlapping(source.live, live, capacity);
        for index in source.live_entries() {
            key_type.retain(source.key(index));
            value_type.retain(source.value(index));
        }
        clear(target);
        target.indices = indices;
        target.entries = entries;
        target.live = live;
        target.log2 = source.log2;
        target.used = source.used;
        target.filled = source.filled;
        target.usable = source.usable;
    }
}

/// Sets each entry of `source` in `target`, in order, CPython's `dict_dict_merge`: an empty
/// target takes a clone of a source without holes that is small or more than half full, and
/// otherwise the target is first resized for both when it has too few usable entries.
///
/// # Safety
///
/// Both point to live dicts of one key type and one value type.
pub(crate) unsafe fn merge(target: *mut DictObject, source: *const DictObject) {
    if core::ptr::eq(target, source) {
        return;
    }
    // SAFETY: the caller's guarantee.
    unsafe {
        let (target, source) = (&mut *target, &*source);
        if source.used == 0 {
            return;
        }
        if target.used == 0
            && source.used == source.filled
            && (source.log2 == super::MINIMUM_LOG2 || usable_entries(source.log2 - 1) < source.used)
        {
            clone_table(target, source);
            return;
        }
        if usable_entries(target.log2) < source.used {
            resize(target, log2_for(target.used + source.used));
        }
        for index in source.live_entries() {
            insert(
                target,
                source.key(index),
                source.hash(index),
                source.value(index),
            );
        }
    }
}

/// Returns a new dict with the same entries (`dict.copy`, `dict(d)`): a clone of a table at
/// most a third holes, and otherwise a merge into a new dict, as CPython copies.
///
/// # Safety
///
/// `dict` points to a live dict.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_copy(dict: *const DictObject) -> *mut DictObject {
    // SAFETY: the caller's guarantee.
    unsafe {
        let source = &*dict;
        let copy = new_dict(source.key_type(), source.value_type(), 0);
        if source.used == 0 {
            return copy;
        }
        if source.used >= source.filled * 2 / 3 {
            clone_table(&mut *copy, source);
        } else {
            merge(copy, source);
        }
        copy
    }
}

/// Sets each entry of `other` in `dict` (`dict.update`, `|=`): `other` is the address of a dict,
/// merged as CPython merges one, or of a list of `(key, value)` tuples, set in order.
///
/// # Safety
///
/// `dict` points to a live dict, and `other` is the address of a live dict of its types or of
/// a live list of tuples of its key and value.
#[unsafe(no_mangle)]
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
pub unsafe extern "C" fn typy_rt_dict_update(dict: *mut DictObject, other: u64) {
    let header = core::ptr::with_exposed_provenance::<crate::header::ObjectHeader>(
        usize::try_from(other).unwrap_or(0),
    );
    // SAFETY: the caller's guarantee; the type ID says what `other` is.
    unsafe {
        if (*header).type_id == DICT_TYPE_ID {
            merge(dict, header.cast::<DictObject>());
            return;
        }
        let pairs = &*header.cast::<ListObject>();
        let descriptor = pairs.item_type().descriptor();
        for index in 0..pairs.length {
            let pair = pairs.item(index);
            let key = crate::tuple::field(descriptor, pair, 0);
            let value = crate::tuple::field(descriptor, pair, 1);
            typy_rt_dict_set(dict, key, value);
        }
    }
}

/// Returns `left == right`: the same number of entries, and each key of `left` in `right` with
/// an equal value, as CPython compares dicts.
///
/// # Safety
///
/// Both point to live dicts of one key type and one value type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_equal(
    left: *const DictObject,
    right: *const DictObject,
) -> bool {
    // SAFETY: the caller's guarantee.
    unsafe {
        let (left, right) = (&*left, &*right);
        if left.used != right.used {
            return false;
        }
        let value_type = left.value_type();
        left.live_entries().all(|index| {
            find(right, left.key(index), left.hash(index))
                .is_some_and(|other| value_type.equal(left.value(index), right.value(other)))
        })
    }
}

/// Returns whether a value of the dict equals `value` (`in` on `dict.values()`).
///
/// # Safety
///
/// `dict` points to a live dict, and `value` is a live value of its type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_has_value(dict: *const DictObject, value: u64) -> bool {
    // SAFETY: the caller's guarantee.
    unsafe {
        let dict = &*dict;
        let value_type = dict.value_type();
        dict.live_entries()
            .any(|index| value_type.equal(dict.value(index), value))
    }
}

/// Returns whether the dict maps `key` to a value equal to `value` (`in` on `dict.items()`).
///
/// # Safety
///
/// `dict` points to a live dict, and `key` and `value` are live items of its types.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_has_item(
    dict: *const DictObject,
    key: u64,
    value: u64,
) -> bool {
    // SAFETY: the caller's guarantee.
    unsafe {
        let hash = (*dict).key_type().hash(key);
        find(dict, key, hash)
            .is_some_and(|position| (*dict).value_type().equal((*dict).value(position), value))
    }
}
