//! Splits and joins strings: `split`, `rsplit`, `splitlines`, `partition`, `rpartition` and
//! `join`, as CPython's `stringlib` does them (LIB-03).

use super::builder::{StrBuilder, is_bytes, space_for, substring};
use super::{StrObject, code_point, length};
use crate::ListObject;
use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::list::{ItemKind, new_list};
use crate::slice::clamped;
use crate::word::inline_value;

/// Appends a new string to a list of strings, which takes over its reference.
///
/// # Safety
///
/// `list` points to a live list of objects.
unsafe fn push(list: *mut ListObject, text: *mut StrObject) {
    // SAFETY: the caller passes a live list; the list keeps the string's only reference.
    unsafe {
        crate::list::typy_rt_list_append(list, text.expose_provenance() as u64);
        crate::refcount::typy_rt_decrement(text.cast());
    }
}

/// Returns whether `needle` occurs in `text` at `at`.
///
/// # Safety
///
/// Both are live strings, and the needle fits in the text from `at`.
unsafe fn occurs_at(text: *const StrObject, needle: *const StrObject, at: usize) -> bool {
    // SAFETY: the caller keeps every index inside both strings.
    unsafe {
        (0..length(needle))
            .all(|offset| code_point(text, at + offset) == code_point(needle, offset))
    }
}

fn empty_separator<T>(value: T) -> Raising<T> {
    Raising::error(value, raise(ExceptionKind::ValueError, "empty separator"))
}

/// Returns `text.split(separator, maxsplit)`, or `rsplit` when bit 1 of `flags` is set, as a
/// new list. Without a separator, bit 0 clear, runs of whitespace separate the parts and none
/// are empty. A negative `maxsplit` splits everywhere; an empty separator raises `ValueError`.
///
/// # Safety
///
/// Both point to live strings, and `maxsplit` and `flags` are `int` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_split(
    text: *const StrObject,
    separator: *const StrObject,
    maxsplit: i64,
    flags: i64,
) -> Raising<*mut ListObject> {
    let flags = inline_value(flags);
    let limit = clamped(maxsplit);
    let limit = if limit < 0 {
        usize::MAX
    } else {
        usize::try_from(limit).unwrap_or(usize::MAX)
    };
    let backwards = flags & 2 != 0;
    // SAFETY: the caller passes live strings; every index stays inside them.
    unsafe {
        let parts = new_list(ItemKind::Object, 0);
        if flags & 1 == 0 {
            if backwards {
                split_whitespace_backwards(text, limit, parts);
            } else {
                split_whitespace(text, limit, parts);
            }
            return Raising::ok(parts);
        }
        let size = length(separator);
        if size == 0 {
            crate::refcount::typy_rt_decrement(parts.cast());
            return empty_separator(core::ptr::null_mut());
        }
        let whole = length(text);
        if backwards {
            let (mut end, mut splits) = (whole, 0);
            let mut at = whole;
            while splits < limit && at >= size {
                if occurs_at(text, separator, at - size) {
                    push(parts, substring(text, at, end));
                    at -= size;
                    end = at;
                    splits += 1;
                } else {
                    at -= 1;
                }
            }
            push(parts, substring(text, 0, end));
            crate::list::typy_rt_list_reverse(parts);
        } else {
            let (mut start, mut splits, mut at) = (0, 0, 0);
            while splits < limit && at + size <= whole {
                if occurs_at(text, separator, at) {
                    push(parts, substring(text, start, at));
                    at += size;
                    start = at;
                    splits += 1;
                } else {
                    at += 1;
                }
            }
            push(parts, substring(text, start, whole));
        }
        Raising::ok(parts)
    }
}

/// Splits on runs of whitespace from the left; after `limit` splits the rest is one part,
/// without its leading whitespace.
///
/// # Safety
///
/// `text` is a live string and `parts` a live list of objects.
unsafe fn split_whitespace(text: *const StrObject, limit: usize, parts: *mut ListObject) {
    // SAFETY: the caller's guarantee; every index stays inside the string.
    unsafe {
        let is_space = space_for(text);
        let whole = length(text);
        let (mut at, mut splits) = (0, 0);
        while splits < limit {
            while at < whole && is_space(code_point(text, at)) {
                at += 1;
            }
            if at == whole {
                return;
            }
            let start = at;
            while at < whole && !is_space(code_point(text, at)) {
                at += 1;
            }
            push(parts, substring(text, start, at));
            splits += 1;
        }
        while at < whole && is_space(code_point(text, at)) {
            at += 1;
        }
        if at < whole {
            push(parts, substring(text, at, whole));
        }
    }
}

/// Splits on runs of whitespace from the right, as `rsplit()` does.
///
/// # Safety
///
/// `text` is a live string and `parts` a live list of objects.
unsafe fn split_whitespace_backwards(text: *const StrObject, limit: usize, parts: *mut ListObject) {
    // SAFETY: the caller's guarantee; every index stays inside the string.
    unsafe {
        let is_space = space_for(text);
        let (mut at, mut splits) = (length(text), 0);
        while splits < limit {
            while at > 0 && is_space(code_point(text, at - 1)) {
                at -= 1;
            }
            if at == 0 {
                break;
            }
            let end = at;
            while at > 0 && !is_space(code_point(text, at - 1)) {
                at -= 1;
            }
            push(parts, substring(text, at, end));
            splits += 1;
        }
        if splits == limit {
            while at > 0 && is_space(code_point(text, at - 1)) {
                at -= 1;
            }
            if at > 0 {
                push(parts, substring(text, 0, at));
            }
        }
        crate::list::typy_rt_list_reverse(parts);
    }
}

/// Returns whether a code point ends a line for `splitlines`.
fn is_line_break(point: u32) -> bool {
    matches!(point, 0x0a..=0x0d | 0x1c..=0x1e | 0x85 | 0x2028 | 0x2029)
}

/// Returns `text.splitlines(keepends)` as a new list: the lines, with their line breaks when
/// `keepends`, an `int` word, is not 0. `\r\n` is one break.
///
/// # Safety
///
/// `text` points to a live string, and `keepends` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_splitlines(
    text: *const StrObject,
    keepends: i64,
) -> *mut ListObject {
    let keep = inline_value(keepends) != 0;
    // SAFETY: the caller passes a live string; every index stays inside it.
    unsafe {
        let whole = length(text);
        let is_line_break: fn(u32) -> bool = if is_bytes(text) {
            |point| point == 0x0a || point == 0x0d
        } else {
            is_line_break
        };
        let lines = new_list(ItemKind::Object, 0);
        let mut at = 0;
        while at < whole {
            let start = at;
            while at < whole && !is_line_break(code_point(text, at)) {
                at += 1;
            }
            let mut end = at;
            if at < whole {
                let pair = code_point(text, at) == 0x0d
                    && at + 1 < whole
                    && code_point(text, at + 1) == 0x0a;
                at += if pair { 2 } else { 1 };
                if keep {
                    end = at;
                }
            }
            push(lines, substring(text, start, end));
        }
        lines
    }
}

/// Returns `text.partition(separator)`, or `rpartition` when `backwards`, an `int` word, is
/// not 0: a new list of the part before the separator, the separator and the part after. When
/// the separator is not found, the text is the first part, or the last for `rpartition`.
///
/// # Safety
///
/// Both point to live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_partition(
    text: *const StrObject,
    separator: *const StrObject,
    backwards: i64,
) -> Raising<*mut ListObject> {
    // SAFETY: the caller passes live strings; every index stays inside them.
    unsafe {
        let (whole, size) = (length(text), length(separator));
        if size == 0 {
            return empty_separator(core::ptr::null_mut());
        }
        let backwards = inline_value(backwards) != 0;
        let positions = (0..=whole.saturating_sub(size)).filter(|_| size <= whole);
        let found = if backwards {
            positions.rev().find(|at| occurs_at(text, separator, *at))
        } else {
            positions.clone().find(|at| occurs_at(text, separator, *at))
        };
        let parts = new_list(ItemKind::Object, 3);
        let (before, middle, after) = match found {
            Some(at) => (
                substring(text, 0, at),
                substring(separator, 0, size),
                substring(text, at + size, whole),
            ),
            None if backwards => (
                substring(text, 0, 0),
                substring(text, 0, 0),
                substring(text, 0, whole),
            ),
            None => (
                substring(text, 0, whole),
                substring(text, 0, 0),
                substring(text, 0, 0),
            ),
        };
        push(parts, before);
        push(parts, middle);
        push(parts, after);
        Raising::ok(parts)
    }
}

/// Returns `separator.join(parts)`: the strings of a list or tuple, with the separator between
/// each two.
///
/// # Safety
///
/// `separator` points to a live string and `parts` to a live list of live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_join(
    separator: *const StrObject,
    parts: *const ListObject,
) -> *mut StrObject {
    // SAFETY: the caller passes a live list of live strings.
    unsafe {
        let mut builder = StrBuilder::with_capacity(0);
        for index in 0..(*parts).length {
            if index > 0 {
                builder.push_str(separator);
            }
            let part = core::ptr::with_exposed_provenance::<StrObject>(
                usize::try_from((*parts).item(index)).unwrap_or(0),
            );
            builder.push_str(part);
        }
        builder.finish_like(separator)
    }
}
