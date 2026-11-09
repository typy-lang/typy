//! The methods of `str` that read or rebuild text without Unicode tables: searching, prefixes
//! and suffixes, stripping, replacing and padding, with CPython's results (LIB-03).

use super::builder::{StrBuilder, is_bytes, space_for, substring};
use super::{StrObject, code_point, length};
use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::slice::clamped;
use crate::word::{inline, inline_value};

/// The bit of a flags word set when a call gives `start`.
const START: i64 = 1;
/// The bit set when a call gives `end`.
const END: i64 = 2;

/// Returns `start` and `end` adjusted to a string of `length` code points, as CPython's
/// `ADJUST_INDICES` does: a negative bound counts from the end, and `end` stops at the length.
/// `start` may stay past the end, which makes every search fail.
fn adjust(start: i64, end: i64, flags: i64, length: usize) -> (i64, i64) {
    let length = i64::try_from(length).unwrap_or(i64::MAX);
    let mut start = if flags & START == 0 {
        0
    } else {
        clamped(start)
    };
    let mut end = if flags & END == 0 {
        length
    } else {
        clamped(end)
    };
    if end > length {
        end = length;
    } else if end < 0 {
        end = (end.saturating_add(length)).max(0);
    }
    if start < 0 {
        start = (start.saturating_add(length)).max(0);
    }
    (start, end)
}

/// Returns whether `needle` occurs in `haystack` at `at`.
///
/// # Safety
///
/// Both point to live strings, and the needle fits in the haystack from `at`.
unsafe fn occurs_at(haystack: *const StrObject, needle: *const StrObject, at: usize) -> bool {
    // SAFETY: the caller keeps every index inside both strings.
    unsafe {
        (0..length(needle))
            .all(|offset| code_point(haystack, at + offset) == code_point(needle, offset))
    }
}

/// Returns the word of a position.
fn word(value: usize) -> i64 {
    inline(i64::try_from(value).unwrap_or(i64::MAX)).unwrap_or(1)
}

/// Searches `text[start:end]` for `needle`, as the operation in bits 2 and up of `flags` asks:
/// 0 `find`, 1 `rfind`, 2 `index`, 3 `rindex`, 4 `count`. Returns an `int` word: the position,
/// -1 when `find` or `rfind` finds nothing, or the number of occurrences that do not overlap.
/// `index` and `rindex` raise `ValueError: substring not found` instead of returning -1, or
/// `subsection not found` for `bytes`.
///
/// # Safety
///
/// Both point to live strings, and `start`, `end` and `flags` are `int` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_search(
    text: *const StrObject,
    needle: *const StrObject,
    start: i64,
    end: i64,
    flags: i64,
) -> Raising<i64> {
    let flags = inline_value(flags);
    let operation = flags >> 2;
    // SAFETY: the caller passes live strings; every index stays inside them.
    unsafe {
        let (first, last) = adjust(start, end, flags, length(text));
        let size = i64::try_from(length(needle)).unwrap_or(i64::MAX);
        if operation == 4 {
            if last - first < size {
                return Raising::ok(word(0));
            }
            if size == 0 {
                return Raising::ok(word(usize::try_from(last - first + 1).unwrap_or(0)));
            }
            let (mut at, mut count) = (first, 0);
            while at + size <= last {
                if occurs_at(text, needle, usize::try_from(at).unwrap_or(0)) {
                    count += 1;
                    at += size;
                } else {
                    at += 1;
                }
            }
            return Raising::ok(word(count));
        }
        let found = if last - first < size {
            None
        } else if operation % 2 == 0 {
            (first..=last - size)
                .find(|at| occurs_at(text, needle, usize::try_from(*at).unwrap_or(0)))
        } else {
            (first..=last - size)
                .rev()
                .find(|at| occurs_at(text, needle, usize::try_from(*at).unwrap_or(0)))
        };
        match (found, operation >= 2) {
            (Some(at), _) => Raising::ok(inline(at).unwrap_or(1)),
            (None, false) => Raising::ok(inline(-1).unwrap_or(1)),
            (None, true) => {
                let message = if is_bytes(text) {
                    "subsection not found"
                } else {
                    "substring not found"
                };
                Raising::error(1, raise(ExceptionKind::ValueError, message))
            }
        }
    }
}

/// Returns `text.startswith(affix, start, end)`, or `endswith` when bit 2 of `flags` is set, as
/// CPython's `tailmatch` decides: an empty affix matches unless `start` is past `end`.
///
/// # Safety
///
/// Both point to live strings, and `start`, `end` and `flags` are `int` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_match(
    text: *const StrObject,
    affix: *const StrObject,
    start: i64,
    end: i64,
    flags: i64,
) -> bool {
    let flags = inline_value(flags);
    // SAFETY: the caller passes live strings; every index stays inside them.
    unsafe {
        let (first, last) = adjust(start, end, flags, length(text));
        let size = i64::try_from(length(affix)).unwrap_or(i64::MAX);
        if last - size < first {
            return false;
        }
        let at = if flags & 4 == 0 { first } else { last - size };
        occurs_at(text, affix, usize::try_from(at).unwrap_or(0))
    }
}

/// Returns `text.startswith(affixes, start, end)` or `endswith` for a tuple of strings: whether
/// any of them matches.
///
/// # Safety
///
/// `text` points to a live string, `affixes` to a live `tuple[str, ...]`, and the rest are
/// `int` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_match_any(
    text: *const StrObject,
    affixes: *const crate::ListObject,
    start: i64,
    end: i64,
    flags: i64,
) -> bool {
    // SAFETY: the caller passes a live tuple of live strings.
    unsafe {
        (0..(*affixes).length).any(|index| {
            let affix = core::ptr::with_exposed_provenance::<StrObject>(
                usize::try_from((*affixes).item(index)).unwrap_or(0),
            );
            typy_rt_str_match(text, affix, start, end, flags)
        })
    }
}

/// Returns `text.strip(chars)`: the text without the code points of `chars` at its ends, or
/// without whitespace when bit 0 of `flags` is clear. Bit 1 strips the start and bit 2 the end.
///
/// # Safety
///
/// Both point to live strings, and `flags` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_strip(
    text: *const StrObject,
    chars: *const StrObject,
    flags: i64,
) -> *mut StrObject {
    let flags = inline_value(flags);
    // SAFETY: the caller passes live strings; every index stays inside them.
    unsafe {
        let is_space = space_for(text);
        let stripped = |point: u32| {
            if flags & 1 == 0 {
                is_space(point)
            } else {
                (0..length(chars)).any(|index| code_point(chars, index) == point)
            }
        };
        let (mut start, mut end) = (0, length(text));
        if flags & 2 != 0 {
            while start < end && stripped(code_point(text, start)) {
                start += 1;
            }
        }
        if flags & 4 != 0 {
            while end > start && stripped(code_point(text, end - 1)) {
                end -= 1;
            }
        }
        substring(text, start, end)
    }
}

/// Returns `text.removeprefix(affix)`, or `removesuffix` when `suffix`, an `int` word, is not 0.
///
/// # Safety
///
/// Both point to live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_remove_affix(
    text: *const StrObject,
    affix: *const StrObject,
    suffix: i64,
) -> *mut StrObject {
    // SAFETY: the caller passes live strings; every index stays inside them.
    unsafe {
        let (size, whole) = (length(affix), length(text));
        if size > whole {
            return substring(text, 0, whole);
        }
        if inline_value(suffix) == 0 {
            if occurs_at(text, affix, 0) {
                return substring(text, size, whole);
            }
        } else if occurs_at(text, affix, whole - size) {
            return substring(text, 0, whole - size);
        }
        substring(text, 0, whole)
    }
}

/// Returns `text.replace(old, new, count)`: at most `count` occurrences of `old`, left to right,
/// replaced by `new`, or all of them for a negative count. An empty `old` matches before every
/// code point and at the end, as in CPython.
///
/// # Safety
///
/// The strings are live, and `count` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_replace(
    text: *const StrObject,
    old: *const StrObject,
    new: *const StrObject,
    count: i64,
) -> *mut StrObject {
    let count = clamped(count);
    let mut remaining = if count < 0 {
        u64::MAX
    } else {
        count.unsigned_abs()
    };
    // SAFETY: the caller passes live strings; every index stays inside them.
    unsafe {
        let (whole, size) = (length(text), length(old));
        let mut builder = StrBuilder::with_capacity(whole);
        let mut at = 0;
        if size == 0 {
            while at <= whole {
                if remaining > 0 {
                    builder.push_str(new);
                    remaining -= 1;
                }
                if at < whole {
                    builder.push(code_point(text, at));
                }
                at += 1;
            }
            return builder.finish_like(text);
        }
        while at < whole {
            if remaining > 0 && at + size <= whole && occurs_at(text, old, at) {
                builder.push_str(new);
                remaining -= 1;
                at += size;
            } else {
                builder.push(code_point(text, at));
                at += 1;
            }
        }
        builder.finish_like(text)
    }
}

/// Returns `text` padded to `width` code points with `fill`: `center` for a `mode` of 0,
/// `ljust` for 1 and `rjust` for 2, an `int` word. `center` puts the odd extra code point on
/// the left when the width is odd, as CPython does. A fill that is not one code point raises
/// `TypeError`.
///
/// # Safety
///
/// Both point to live strings, and `width` and `mode` are `int` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_pad(
    text: *const StrObject,
    width: i64,
    fill: *const StrObject,
    mode: i64,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller passes live strings; every index stays inside them.
    unsafe {
        if length(fill) != 1 {
            let error = if is_bytes(text) {
                // CPython 3.15's wording, the newest oracle's (OI-14).
                let mut message = crate::message::MessageBuffer::new();
                message.push_str(match inline_value(mode) {
                    0 => "center",
                    1 => "ljust",
                    _ => "rjust",
                });
                message.push_str("(): argument 2 must be a byte string of length 1, not a bytes object of length ");
                message.push_number(i64::try_from(length(fill)).unwrap_or(i64::MAX));
                message.raise(ExceptionKind::TypeError)
            } else {
                raise(
                    ExceptionKind::TypeError,
                    "The fill character must be exactly one character long",
                )
            };
            return Raising::error(core::ptr::null_mut(), error);
        }
        let whole = length(text);
        let width = usize::try_from(clamped(width).max(0)).unwrap_or(usize::MAX);
        if width <= whole {
            return Raising::ok(substring(text, 0, whole));
        }
        let margin = width - whole;
        let left = match inline_value(mode) {
            0 => margin / 2 + (margin & width & 1),
            1 => 0,
            _ => margin,
        };
        let point = code_point(fill, 0);
        let mut builder = StrBuilder::with_capacity(width);
        (0..left).for_each(|_| builder.push(point));
        builder.push_str(text);
        (0..margin - left).for_each(|_| builder.push(point));
        Raising::ok(builder.finish_like(text))
    }
}

/// Returns `text.zfill(width)`: zeros on the left up to `width`, after a leading sign.
///
/// # Safety
///
/// `text` points to a live string, and `width` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_zfill(text: *const StrObject, width: i64) -> *mut StrObject {
    // SAFETY: the caller passes a live string; every index stays inside it.
    unsafe {
        let whole = length(text);
        let width = usize::try_from(clamped(width).max(0)).unwrap_or(usize::MAX);
        if width <= whole {
            return substring(text, 0, whole);
        }
        let signed = whole > 0 && matches!(code_point(text, 0), 0x2b | 0x2d);
        let mut builder = StrBuilder::with_capacity(width);
        if signed {
            builder.push(code_point(text, 0));
        }
        (0..width - whole).for_each(|_| builder.push(u32::from(b'0')));
        builder.push_range(text, usize::from(signed), whole);
        builder.finish_like(text)
    }
}

/// Returns `text.expandtabs(tabsize)`: each tab replaced by spaces up to the next multiple of
/// `tabsize` columns, the column starting again after `\n` and `\r`. A size below one removes
/// the tabs.
///
/// # Safety
///
/// `text` points to a live string, and `tabsize` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_expandtabs(
    text: *const StrObject,
    tabsize: i64,
) -> *mut StrObject {
    let size = usize::try_from(clamped(tabsize).max(0)).unwrap_or(usize::MAX);
    // SAFETY: the caller passes a live string; every index stays inside it.
    unsafe {
        let mut builder = StrBuilder::with_capacity(length(text));
        let mut column = 0;
        for index in 0..length(text) {
            let point = code_point(text, index);
            if point == u32::from(b'\t') {
                if size > 0 {
                    let spaces = size - column % size;
                    (0..spaces).for_each(|_| builder.push(u32::from(b' ')));
                    column += spaces;
                }
            } else {
                builder.push(point);
                column = if point == 0x0a || point == 0x0d {
                    0
                } else {
                    column + 1
                };
            }
        }
        builder.finish_like(text)
    }
}

/// Returns `text.isascii()`: whether every code point is below 128.
///
/// # Safety
///
/// `text` points to a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_isascii(text: *const StrObject) -> bool {
    // SAFETY: the caller passes a live string; every index stays inside it.
    unsafe { (0..length(text)).all(|index| code_point(text, index) < 0x80) }
}

#[cfg(test)]
mod tests;
