//! `reversed()` of a value held as one of several built-in types, as an abstract collection
//! such as `Sequence[int]` or `Reversible[str]` holds it (LIB-20, ADR-0086): an iterator that
//! reads its source from the last item to the first, as CPython's `list_reverseiterator`,
//! `reversed`, `range_iterator` and dict reverse iterators do.

use super::{IteratorKind, IteratorObject, made, range_item, text_item};
use crate::dict::DictObject;
use crate::exception::{ExceptionKind, raise};
use crate::header::{
    BYTES_TYPE_ID, DICT_TYPE_ID, DICT_VIEW_TYPE_ID, ObjectHeader, RANGE_TYPE_ID, STR_TYPE_ID,
    TUPLE_TYPE_ID,
};
use crate::list::ListObject;
use crate::refcount::typy_rt_increment;
use crate::string::{StrObject, length};
use crate::tuple::TupleType;
use crate::word::{inline, inline_value};

/// Returns a new iterator that reads `source` from its last item to its first: a list, a
/// `tuple[T, ...]`, a `str`, a `bytes`, a range, a dict or a view of one. `pair_type` is the
/// layout of the items of a view of items, and null for any other source.
///
/// # Safety
///
/// `source` points to a live object of one of those types, and `pair_type` is a live tuple
/// descriptor when it is a view of items.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, a frame block or a static, aligned for every type, and starts with its header"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_iterator_reversed(
    source: *mut ObjectHeader,
    pair_type: *const TupleType,
) -> *mut IteratorObject {
    // SAFETY: the caller's guarantee; the header says what the object is.
    unsafe {
        let (object, kind, pair) = match (*source).type_id {
            STR_TYPE_ID => (source, IteratorKind::ReversedStr, core::ptr::null()),
            BYTES_TYPE_ID => (source, IteratorKind::ReversedBytes, core::ptr::null()),
            RANGE_TYPE_ID => (source, IteratorKind::ReversedRange, core::ptr::null()),
            TUPLE_TYPE_ID => (source, IteratorKind::ReversedTuple, core::ptr::null()),
            DICT_TYPE_ID => (source, IteratorKind::ReversedDictKeys, core::ptr::null()),
            DICT_VIEW_TYPE_ID => {
                let view = &*source.cast::<crate::dict::view::ViewObject>();
                let (kind, pair) = match view.kind {
                    0 => (IteratorKind::ReversedDictKeys, core::ptr::null()),
                    1 => (IteratorKind::ReversedDictValues, core::ptr::null()),
                    _ => (IteratorKind::ReversedDictItems, pair_type),
                };
                (view.dict.cast::<ObjectHeader>(), kind, pair)
            }
            _ => (source, IteratorKind::ReversedList, core::ptr::null()),
        };
        typy_rt_increment(object);
        // A dict is read from its last entry, and checked against the number of its items.
        let (position, count) = match kind {
            IteratorKind::ReversedStr | IteratorKind::ReversedBytes => {
                (length(object.cast::<StrObject>()), 0)
            }
            IteratorKind::ReversedRange => (crate::range::count(object.cast()), 0),
            IteratorKind::ReversedList | IteratorKind::ReversedTuple => {
                ((*object.cast::<ListObject>()).length, 0)
            }
            _ => {
                let dict = &*object.cast::<DictObject>();
                (dict.filled, dict.used)
            }
        };
        made(object, kind, (position, count), pair)
    }
}

/// Steps a reversed iterator to the item before the last one it gave, and returns whether
/// there was one. A list read past its end, after it shrank, ends, as CPython's does.
///
/// # Safety
///
/// The iterator is live and of a reversed kind, and its source is live.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, a frame block or a static, aligned for every type, and starts with its header"
)]
pub(super) unsafe fn step(
    iterator: &mut IteratorObject,
) -> Result<bool, *mut crate::ExceptionObject> {
    let source = iterator.source;
    if matches!(
        iterator.kind,
        IteratorKind::ReversedDictKeys
            | IteratorKind::ReversedDictValues
            | IteratorKind::ReversedDictItems
    ) {
        // SAFETY: the caller's guarantee.
        return unsafe { dict_step(iterator, source.cast()) };
    }
    let Some(index) = iterator.position.checked_sub(1) else {
        return Ok(false);
    };
    // SAFETY: the caller's guarantee; the index is below the source's length.
    unsafe {
        match iterator.kind {
            IteratorKind::ReversedStr | IteratorKind::ReversedBytes => {
                text_item(iterator, source.cast(), index);
            }
            IteratorKind::ReversedRange => {
                let Some(value) = crate::range::item(source.cast(), index) else {
                    return Ok(false);
                };
                range_item(iterator, value);
            }
            _ => {
                let list = &*source.cast::<ListObject>();
                if index >= list.length {
                    return Ok(false);
                }
                iterator.current = list.item(index);
            }
        }
    }
    iterator.position = index;
    Ok(true)
}

/// Steps a reversed iterator of a dict's keys, values or items to the live entry before the
/// last one it gave. A dict whose size changed raises CPython's `RuntimeError`.
///
/// # Safety
///
/// The iterator is live, and `dict` is its live source.
unsafe fn dict_step(
    iterator: &mut IteratorObject,
    dict: *const DictObject,
) -> Result<bool, *mut crate::ExceptionObject> {
    // SAFETY: the caller's guarantee; each entry looked at is below `filled`.
    unsafe {
        let table = &*dict;
        if table.used != iterator.length {
            return Err(raise(
                ExceptionKind::RuntimeError,
                "dictionary changed size during iteration",
            ));
        }
        let end = iterator.position.min(table.filled);
        let Some(entry) = (0..end).rev().find(|index| table.is_live(*index)) else {
            return Ok(false);
        };
        let at = inline(i64::try_from(entry).unwrap_or(i64::MAX)).unwrap_or(1);
        let key = crate::dict::typy_rt_dict_key(dict, at);
        let value = crate::dict::typy_rt_dict_value(dict, at);
        iterator.current = match iterator.kind {
            IteratorKind::ReversedDictKeys => key,
            IteratorKind::ReversedDictValues => value,
            _ => {
                let address = iterator.pair.expose_provenance() as u64;
                crate::tuple::store_field(iterator.pair_type, address, 0, key);
                crate::tuple::store_field(iterator.pair_type, address, 1, value);
                address
            }
        };
        iterator.position = entry;
        debug_assert!(inline_value(at) >= 0);
    }
    Ok(true)
}
