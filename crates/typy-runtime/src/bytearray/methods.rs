//! The methods that change a bytearray or make a new one from it: the list methods, item
//! and slice deletion and assignment, `+` and `*`, and their in-place forms (LIB-28).

use super::{
    BytearrayObject, byte, bytes_of, data_of, from_data, position, reserve, room, set_length,
};
use crate::Raising;
use crate::exception::{ExceptionKind, ExceptionObject, raise};
use crate::slice::select;
use crate::string::StrObject;
use crate::word::inline;

/// Appends the byte `value`; one outside 0 to 255 raises `ValueError`. Returns the exception,
/// or null.
///
/// # Safety
///
/// `array` points to a live bytearray, and `value` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_append(
    array: *mut BytearrayObject,
    value: i64,
) -> *mut ExceptionObject {
    let value = match byte(value) {
        Ok(value) => value,
        Err(error) => return error,
    };
    // SAFETY: the caller's guarantee; `reserve` makes room for one more byte.
    unsafe {
        let size = bytes_of(array).len();
        reserve(array, size + 1);
        room(array)[size] = value;
        set_length(array, size + 1);
    }
    core::ptr::null_mut()
}

/// Appends the bytes of `data`, a `bytes` or a bytearray's buffer, which may be the
/// bytearray's own.
///
/// # Safety
///
/// `array` points to a live bytearray, and `data` to a live `bytes`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_extend(
    array: *mut BytearrayObject,
    data: *const StrObject,
) {
    // SAFETY: the caller's guarantee; the bytes are copied before the buffer may move.
    unsafe {
        let own = core::ptr::eq(data, (*array).buffer);
        let added = data_of(data).len();
        let size = bytes_of(array).len();
        reserve(array, size + added);
        // `reserve` may move the buffer, so the bytearray's own bytes are read from it again.
        let source = if own {
            (*array).buffer
        } else {
            data.cast_mut()
        };
        core::ptr::copy(
            crate::string::latin1_data(source),
            crate::string::latin1_data((*array).buffer).add(size),
            added,
        );
        set_length(array, size + added);
    }
}

/// Appends each `int` item of `values`; at an item outside 0 to 255 it stops with
/// `ValueError`, keeping the items before it, as CPython does.
///
/// # Safety
///
/// `array` points to a live bytearray, and `values` to a live list of `int` items.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_extend_list(
    array: *mut BytearrayObject,
    values: *const crate::ListObject,
) -> *mut ExceptionObject {
    // SAFETY: the caller's guarantee.
    unsafe {
        for index in 0..(*values).length {
            let error = super::typy_rt_bytearray_append(array, (*values).item(index).cast_signed());
            if !error.is_null() {
                return error;
            }
        }
    }
    core::ptr::null_mut()
}

/// Inserts the byte `value` before position `index`, counted from the end when negative and
/// kept inside the bytearray, as `list.insert` does.
///
/// # Safety
///
/// `array` points to a live bytearray, and `index` and `value` are `int` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_insert(
    array: *mut BytearrayObject,
    index: i64,
    value: i64,
) -> *mut ExceptionObject {
    let value = match byte(value) {
        Ok(value) => value,
        Err(error) => return error,
    };
    // SAFETY: the caller's guarantee; `reserve` makes room for one more byte.
    unsafe {
        let size = bytes_of(array).len();
        let length = i64::try_from(size).unwrap_or(i64::MAX);
        let index = crate::slice::clamped(index);
        let at = if index < 0 {
            index.saturating_add(length).max(0)
        } else {
            index.min(length)
        };
        let at = usize::try_from(at).unwrap_or(0);
        reserve(array, size + 1);
        room(array).copy_within(at..size, at + 1);
        room(array)[at] = value;
        set_length(array, size + 1);
    }
    core::ptr::null_mut()
}

/// Removes and returns the byte at `index`, as an `int` word; an empty bytearray or a position
/// outside raises `IndexError`.
///
/// # Safety
///
/// `array` points to a live bytearray, and `index` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_pop(
    array: *mut BytearrayObject,
    index: i64,
) -> Raising<i64> {
    // SAFETY: the caller's guarantee; `position` keeps the index inside.
    unsafe {
        let size = bytes_of(array).len();
        if size == 0 {
            return Raising::error(
                1,
                raise(ExceptionKind::IndexError, "pop from empty bytearray"),
            );
        }
        match position(array, index, "pop index out of range") {
            Ok(at) => {
                let value = room(array)[at];
                room(array).copy_within(at + 1..size, at);
                set_length(array, size - 1);
                Raising::ok(inline(i64::from(value)).unwrap_or(1))
            }
            Err(error) => Raising::error(1, error),
        }
    }
}

/// Removes the first byte equal to `value`; one outside 0 to 255 or not there raises
/// `ValueError`.
///
/// # Safety
///
/// `array` points to a live bytearray, and `value` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_remove(
    array: *mut BytearrayObject,
    value: i64,
) -> *mut ExceptionObject {
    let value = match byte(value) {
        Ok(value) => value,
        Err(error) => return error,
    };
    // SAFETY: the caller's guarantee.
    unsafe {
        let size = bytes_of(array).len();
        let Some(at) = bytes_of(array).iter().position(|each| *each == value) else {
            return raise(ExceptionKind::ValueError, "value not found in bytearray");
        };
        room(array).copy_within(at + 1..size, at);
        set_length(array, size - 1);
    }
    core::ptr::null_mut()
}

/// Reverses the bytes in place.
///
/// # Safety
///
/// `array` points to a live bytearray.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_reverse(array: *mut BytearrayObject) {
    // SAFETY: the caller's guarantee.
    unsafe {
        let size = bytes_of(array).len();
        room(array)[..size].reverse();
    }
}

/// Removes every byte.
///
/// # Safety
///
/// `array` points to a live bytearray.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_clear(array: *mut BytearrayObject) {
    // SAFETY: the caller's guarantee.
    unsafe { set_length(array, 0) };
}

/// Returns `array.copy()`: a new bytearray of the same bytes.
///
/// # Safety
///
/// `array` points to a live bytearray.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_copy(
    array: *const BytearrayObject,
) -> *mut BytearrayObject {
    // SAFETY: the caller's guarantee.
    from_data(unsafe { bytes_of(array) })
}

/// Deletes the byte at `index`; a position outside raises `IndexError`. Returns the
/// exception, or null.
///
/// # Safety
///
/// `array` points to a live bytearray, and `index` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_delete(
    array: *mut BytearrayObject,
    index: i64,
) -> *mut ExceptionObject {
    // SAFETY: the caller's guarantee; `position` keeps the index inside.
    unsafe {
        match position(array, index, "bytearray index out of range") {
            Ok(at) => {
                let size = bytes_of(array).len();
                room(array).copy_within(at + 1..size, at);
                set_length(array, size - 1);
                core::ptr::null_mut()
            }
            Err(error) => error,
        }
    }
}

/// Returns `array[start:stop:stride]`, a new bytearray; `present` says which bounds are given.
///
/// # Safety
///
/// `array` points to a live bytearray, and the bounds are `int` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_slice(
    array: *const BytearrayObject,
    start: i64,
    stop: i64,
    stride: i64,
    present: i64,
) -> Raising<*mut BytearrayObject> {
    // SAFETY: the caller's guarantee; the selection stays inside the bytearray.
    unsafe {
        let bytes = bytes_of(array);
        match select(start, stop, stride, present, bytes.len()) {
            Ok(selection) => {
                let sliced = super::wrap(crate::bytes::new_bytes(selection.count), selection.count);
                for index in 0..selection.count {
                    super::room(sliced)[index] = bytes[selection.position(index)];
                }
                set_length(sliced, selection.count);
                Raising::ok(sliced)
            }
            Err(error) => Raising::error(core::ptr::null_mut(), error),
        }
    }
}

/// Replaces `array[start:stop:stride]` with the bytes of `data`, a `bytes` or a bytearray's
/// buffer, which may be the bytearray's own; a slice of step 1 may change the length, and any
/// other takes exactly as many bytes as it selects, `ValueError` otherwise. Returns the
/// exception, or null.
///
/// # Safety
///
/// `array` points to a live bytearray, `data` to a live `bytes`, and the bounds are `int`
/// words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_set_slice(
    array: *mut BytearrayObject,
    start: i64,
    stop: i64,
    stride: i64,
    present: i64,
    data: *const StrObject,
) -> *mut ExceptionObject {
    // SAFETY: the caller's guarantee; the bytes are copied first when they are the
    // bytearray's own, which the change would overwrite.
    unsafe {
        let own = core::ptr::eq(data, (*array).buffer);
        let copied = own.then(|| crate::bytes::from_slice(data_of(data)));
        let source = copied.unwrap_or(data.cast_mut());
        let error = replace(array, (start, stop, stride, present), data_of(source));
        if let Some(copied) = copied {
            crate::refcount::typy_rt_decrement(copied.cast());
        }
        error
    }
}

/// Replaces the selected bytes of `array` with `new`.
///
/// # Safety
///
/// `array` points to a live bytearray that `new` is not part of, and the bounds are `int`
/// words.
unsafe fn replace(
    array: *mut BytearrayObject,
    (start, stop, stride, present): (i64, i64, i64, i64),
    new: &[u8],
) -> *mut ExceptionObject {
    // SAFETY: the caller's guarantee; every position is inside the bytearray.
    unsafe {
        let size = bytes_of(array).len();
        let selection = match select(start, stop, stride, present, size) {
            Ok(selection) => selection,
            Err(error) => return error,
        };
        if selection.step != 1 {
            if new.len() != selection.count {
                return extended_size_error(new.len(), selection.count);
            }
            for (index, value) in new.iter().enumerate() {
                room(array)[selection.position(index)] = *value;
            }
            return core::ptr::null_mut();
        }
        let from = selection.start_position();
        let to = from + selection.count;
        let grown = size - selection.count + new.len();
        reserve(array, grown);
        room(array).copy_within(to..size, from + new.len());
        room(array)[from..from + new.len()].copy_from_slice(new);
        set_length(array, grown);
        core::ptr::null_mut()
    }
}

/// Returns CPython's `ValueError` for `given` bytes assigned to an extended slice of
/// `selected` bytes.
fn extended_size_error(given: usize, selected: usize) -> *mut ExceptionObject {
    let mut message = crate::message::MessageBuffer::new();
    message.push_str("attempt to assign bytes of size ");
    message.push_number(i64::try_from(given).unwrap_or(i64::MAX));
    message.push_str(" to extended slice of size ");
    message.push_number(i64::try_from(selected).unwrap_or(i64::MAX));
    message.raise(ExceptionKind::ValueError)
}

/// Deletes `array[start:stop:stride]`. Returns the exception, or null.
///
/// # Safety
///
/// `array` points to a live bytearray, and the bounds are `int` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_delete_slice(
    array: *mut BytearrayObject,
    start: i64,
    stop: i64,
    stride: i64,
    present: i64,
) -> *mut ExceptionObject {
    // SAFETY: the caller's guarantee; every position is inside the bytearray.
    unsafe {
        let size = bytes_of(array).len();
        let selection = match select(start, stop, stride, present, size) {
            Ok(selection) => selection,
            Err(error) => return error,
        };
        // The selected positions step evenly from the start, so a position is selected when
        // its distance from the start, in the slice's direction, is a multiple of the step
        // within the count.
        let first = selection.start;
        let interval = selection.step;
        let count = i64::try_from(selection.count).unwrap_or(i64::MAX);
        let mut kept = 0;
        for at in 0..size {
            let distance = i64::try_from(at).unwrap_or(i64::MAX) - first;
            let selected = distance % interval == 0 && (0..count).contains(&(distance / interval));
            if !selected {
                room(array)[kept] = room(array)[at];
                kept += 1;
            }
        }
        set_length(array, kept);
    }
    core::ptr::null_mut()
}

/// Returns `left + right` of a bytearray and a `bytes` or a bytearray's buffer: a new
/// bytearray.
///
/// # Safety
///
/// `left` points to a live bytearray, and `right` to a live `bytes`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_concat(
    left: *const BytearrayObject,
    right: *const StrObject,
) -> *mut BytearrayObject {
    // SAFETY: the caller's guarantee.
    unsafe {
        let combined = from_data(bytes_of(left));
        super::typy_rt_bytearray_extend(combined, right);
        combined
    }
}

/// Returns `array * count`, a new bytearray; a count below one gives an empty one, and one too
/// large raises `MemoryError`.
///
/// # Safety
///
/// `array` points to a live bytearray, and `count` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_repeat(
    array: *const BytearrayObject,
    count: i64,
) -> Raising<*mut BytearrayObject> {
    // SAFETY: the caller's guarantee; the repeated `bytes` is new.
    unsafe {
        let repeated = crate::bytes::typy_rt_bytes_repeat((*array).buffer, count);
        if repeated.exception.is_null() {
            Raising::ok(super::typy_rt_bytearray_adopt(repeated.value))
        } else {
            Raising::error(core::ptr::null_mut(), repeated.exception)
        }
    }
}

/// Repeats the bytes of `array` in place, `array *= count`. Returns the exception, or null.
///
/// # Safety
///
/// `array` points to a live bytearray, and `count` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_repeat_in_place(
    array: *mut BytearrayObject,
    count: i64,
) -> *mut ExceptionObject {
    // SAFETY: the caller's guarantee; the repeated `bytes` is new.
    unsafe {
        let repeated = crate::bytes::typy_rt_bytes_repeat((*array).buffer, count);
        if !repeated.exception.is_null() {
            return repeated.exception;
        }
        let size = data_of(repeated.value).len();
        reserve(array, size);
        room(array)[..size].copy_from_slice(data_of(repeated.value));
        set_length(array, size);
        crate::refcount::typy_rt_decrement(repeated.value.cast());
    }
    core::ptr::null_mut()
}

/// Returns whether the `int` word `value` is a byte of `array`, for `value in array`; one
/// outside 0 to 255 raises `ValueError`.
///
/// # Safety
///
/// `array` points to a live bytearray, and `value` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_contains_byte(
    array: *const BytearrayObject,
    value: i64,
) -> Raising<bool> {
    match byte(value) {
        // SAFETY: the caller's guarantee.
        Ok(value) => Raising::ok(unsafe { bytes_of(array).contains(&value) }),
        Err(error) => Raising::error(false, error),
    }
}
