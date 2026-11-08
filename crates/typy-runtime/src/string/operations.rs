//! The string operations generated code calls: concatenation, repetition, indexing, length and comparison.

use super::{StrKind, StrObject, code_point, length, new_string, set_code_point};
use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::word::{inline_value, is_inline, small};

/// Returns `left + right`.
///
/// # Safety
///
/// Both arguments point to live `str` objects.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_concat(
    left: *const StrObject,
    right: *const StrObject,
) -> *mut StrObject {
    // SAFETY: the caller passes live strings; every index stays below the source lengths.
    unsafe {
        let (left_length, right_length) = (length(left), length(right));
        let kind = (*left).kind.max((*right).kind);
        let result = new_string(left_length + right_length, kind);
        for index in 0..left_length {
            set_code_point(result, index, code_point(left, index));
        }
        for index in 0..right_length {
            set_code_point(result, left_length + index, code_point(right, index));
        }
        result
    }
}

/// Returns `text * count`: an empty string for a count below one.
///
/// # Safety
///
/// `text` points to a live `str` object, and `count` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_repeat(
    text: *const StrObject,
    count: i64,
) -> Raising<*mut StrObject> {
    if !is_inline(count) {
        // A large positive count is too big to repeat; a large negative one repeats nothing.
        // SAFETY: a large integer word points to a live large integer.
        if unsafe { crate::int::is_negative(count) } {
            return Raising::ok(new_string(0, StrKind::Latin1));
        }
        let error = raise(
            ExceptionKind::OverflowError,
            "cannot fit 'int' into an index-sized integer",
        );
        return Raising::error(core::ptr::null_mut(), error);
    }
    let times = usize::try_from(inline_value(count).max(0)).unwrap_or(usize::MAX);
    // SAFETY: the caller passes a live string.
    let source_length = unsafe { length(text) };
    let Some(total) = source_length.checked_mul(times) else {
        let error = raise(ExceptionKind::OverflowError, "repeated string is too long");
        return Raising::error(core::ptr::null_mut(), error);
    };
    // SAFETY: the source is live and every index stays in bounds.
    unsafe {
        let result = new_string(total, (*text).kind);
        for copy in 0..times {
            for index in 0..source_length {
                set_code_point(
                    result,
                    copy * source_length + index,
                    code_point(text, index),
                );
            }
        }
        Raising::ok(result)
    }
}

/// Returns `text[start:stop:step]`, a new string (DAT-12); `present` says which bounds are given.
///
/// # Safety
///
/// `text` points to a live `str` object, and the bounds are `int` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_slice(
    text: *const StrObject,
    start: i64,
    stop: i64,
    stride: i64,
    present: i64,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller passes a live string; every selected position is inside it.
    unsafe {
        let selection = match crate::slice::select(start, stop, stride, present, length(text)) {
            Ok(selection) => selection,
            Err(error) => return Raising::error(core::ptr::null_mut(), error),
        };
        // A string is as narrow as its widest code point, so the kind is measured on the slice.
        let kind = (0..selection.count)
            .map(|index| StrKind::of(code_point(text, selection.position(index))))
            .max()
            .unwrap_or(StrKind::Latin1);
        let result = new_string(selection.count, kind);
        for index in 0..selection.count {
            set_code_point(result, index, code_point(text, selection.position(index)));
        }
        Raising::ok(result)
    }
}

/// Returns `text[index]`, counting a negative index from the end (DAT-11).
///
/// # Safety
///
/// `text` points to a live `str` object, and `index` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_index(
    text: *const StrObject,
    index: i64,
) -> Raising<*mut StrObject> {
    let out_of_range = |message| {
        Raising::error(
            core::ptr::null_mut(),
            raise(ExceptionKind::IndexError, message),
        )
    };
    if !is_inline(index) {
        return out_of_range("cannot fit 'int' into an index-sized integer");
    }
    // SAFETY: the caller passes a live string.
    let text_length = unsafe { length(text) };
    let position = inline_value(index);
    let position = if position < 0 {
        position + i64::try_from(text_length).unwrap_or(i64::MAX)
    } else {
        position
    };
    let Some(position) = usize::try_from(position)
        .ok()
        .filter(|position| *position < text_length)
    else {
        return out_of_range("string index out of range");
    };
    // SAFETY: the position is in bounds, and the new string holds exactly that code point.
    unsafe {
        let point = code_point(text, position);
        let result = new_string(1, StrKind::of(point));
        set_code_point(result, 0, point);
        Raising::ok(result)
    }
}

/// Returns `len(text)` as an `int` word.
///
/// # Safety
///
/// `text` points to a live `str` object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_length(text: *const StrObject) -> i64 {
    // SAFETY: the caller passes a live string.
    small(unsafe { length(text) })
}

/// Compares two strings by their code points; `comparison` is a tagged `int` numbered as TyIR's `Comparison::code`.
///
/// # Safety
///
/// Both arguments point to live `str` objects.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_compare(
    left: *const StrObject,
    right: *const StrObject,
    comparison: i64,
) -> bool {
    // SAFETY: the caller passes live strings, and every index stays below both lengths.
    let ordering = unsafe {
        let (left_length, right_length) = (length(left), length(right));
        let mut ordering = left_length.cmp(&right_length);
        for index in 0..left_length.min(right_length) {
            let pair = code_point(left, index).cmp(&code_point(right, index));
            if pair.is_ne() {
                ordering = pair;
                break;
            }
        }
        ordering
    };
    crate::compare::holds(ordering, comparison)
}

#[cfg(test)]
mod tests;
