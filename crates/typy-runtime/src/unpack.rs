//! Unpacking an iterable into targets (SMT-02, EXP-17): `a, b = source` and `a, *rest, b =
//! source` take the values of any iterable, which a new list holds, and CPython 3.15's
//! `ValueError` when there are too few or too many.

#[cfg(test)]
mod tests;

use crate::Raising;
use crate::exception::{ExceptionKind, ExceptionObject};
use crate::header::{DICT_TYPE_ID, LIST_TYPE_ID, ObjectHeader, TUPLE_TYPE_ID};
use crate::list::ListObject;
use crate::message::MessageBuffer;
use crate::word::{inline, inline_value};

/// Returns a new list of the values of `source` for `before` targets, then `after` targets,
/// with a target between them that takes the rest when `starred`. The list holds items of
/// the kind `kind`, an `int` word, with the tuple descriptor `descriptor`, as `list_new`
/// takes them. Without a target that takes the rest, the list holds exactly `before +
/// after` values, and at most one more is read, as CPython reads an iterator; with one, it
/// holds every value, at least `before + after`.
///
/// # Safety
///
/// `source` is the address of a live iterable object, and `descriptor` is 0 or the address
/// of the descriptor of the items, which are tuples.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_unpack(
    source: u64,
    kind: i64,
    descriptor: u64,
    before: i64,
    after: i64,
    starred: bool,
) -> Raising<*mut ListObject> {
    let wanted = inline_value(before) + inline_value(after);
    let object = core::ptr::with_exposed_provenance_mut::<ObjectHeader>(
        usize::try_from(source).unwrap_or(0),
    );
    let none = core::ptr::null_mut();
    let list = crate::list::typy_rt_list_new(kind, inline(0).unwrap_or(1), descriptor);
    // SAFETY: the caller's guarantee.
    unsafe {
        let iterator = crate::iterator::typy_rt_iterable_iterator(
            object,
            core::ptr::with_exposed_provenance(usize::try_from(descriptor).unwrap_or(0)),
        );
        let mut count = 0;
        let mut raised: *mut ExceptionObject = core::ptr::null_mut();
        while starred || count <= wanted {
            let step = crate::iterator::typy_rt_iterator_advance(iterator, inline(0).unwrap_or(1));
            if !step.exception.is_null() {
                raised = step.exception;
                break;
            }
            if !step.value {
                break;
            }
            crate::list::typy_rt_list_append(
                list,
                crate::iterator::typy_rt_iterator_item(iterator),
            );
            count += 1;
        }
        crate::refcount::typy_rt_decrement(iterator.cast());
        if raised.is_null() {
            raised = miscounted(object, count, wanted, starred);
        }
        if raised.is_null() {
            return Raising::ok(list);
        }
        crate::refcount::typy_rt_decrement(list.cast());
        Raising::error(none, raised)
    }
}

/// Returns CPython 3.15's `ValueError` when `count` values, of the iterable `object`, do not
/// fit `wanted` targets, at least that many when `starred`; null when they fit. A list, a
/// tuple and a dict, whose lengths are known, give their length.
///
/// # Safety
///
/// `object` points to a live iterable object.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "a list or a tuple comes from malloc, aligned for every type"
)]
unsafe fn miscounted(
    object: *mut ObjectHeader,
    count: i64,
    wanted: i64,
    starred: bool,
) -> *mut ExceptionObject {
    let mut text = MessageBuffer::new();
    if count < wanted {
        text.push_str("not enough values to unpack (expected ");
        if starred {
            text.push_str("at least ");
        }
        text.push_number(wanted);
        text.push_str(", got ");
        text.push_number(count);
        text.push_str(")");
        return text.raise(ExceptionKind::ValueError);
    }
    if starred || count <= wanted {
        return core::ptr::null_mut();
    }
    text.push_str("too many values to unpack (expected ");
    text.push_number(wanted);
    // SAFETY: the caller's guarantee; the type ID says which object it is.
    let length = unsafe {
        match (*object).type_id {
            LIST_TYPE_ID | TUPLE_TYPE_ID => {
                i64::try_from((*object.cast::<ListObject>()).length).ok()
            }
            DICT_TYPE_ID => Some(inline_value(crate::dict::typy_rt_dict_length(
                object.cast(),
            ))),
            _ => None,
        }
    };
    if let Some(length) = length {
        text.push_str(", got ");
        text.push_number(length);
    }
    text.push_str(")");
    text.raise(ExceptionKind::ValueError)
}
