//! The methods of `bytes` (LIB-03). Most are the string algorithms, which read the type of the
//! object they are given: on a `bytes` they treat only ASCII as whitespace and line breaks and
//! give `bytes` results. These symbols carry the `bytes` signatures; the case methods and the
//! `is` methods are ASCII only, as CPython's are.

use crate::ListObject;
use crate::Raising;
use crate::string::{StrBuilder, StrObject, code_point, length};
use crate::word::inline_value;

macro_rules! forward {
    ($(#[$doc:meta])* $name:ident => $target:path, ($($argument:ident: $kind:ty),*) -> $result:ty) => {
        $(#[$doc])*
        ///
        /// # Safety
        ///
        /// The objects are live, of the types the signature names, and the rest are `int` words.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($argument: $kind),*) -> $result {
            // SAFETY: the caller's guarantee, which the string function needs.
            unsafe { $target($($argument),*) }
        }
    };
}

type Text = *const StrObject;

forward!(
    /// `find`, `rfind`, `index`, `rindex` and `count` of `bytes`.
    typy_rt_bytes_search => crate::string::typy_rt_str_search,
    (data: Text, needle: Text, start: i64, end: i64, flags: i64) -> Raising<i64>
);
forward!(
    /// `startswith` and `endswith` of `bytes`.
    typy_rt_bytes_match => crate::string::typy_rt_str_match,
    (data: Text, affix: Text, start: i64, end: i64, flags: i64) -> bool
);
forward!(
    /// `startswith` and `endswith` of `bytes` with a tuple of `bytes`.
    typy_rt_bytes_match_any => crate::string::typy_rt_str_match_any,
    (data: Text, affixes: *const ListObject, start: i64, end: i64, flags: i64) -> bool
);
forward!(
    /// `split` and `rsplit` of `bytes`.
    typy_rt_bytes_split => crate::string::typy_rt_str_split,
    (data: Text, separator: Text, maxsplit: i64, flags: i64) -> Raising<*mut ListObject>
);
forward!(
    /// `splitlines` of `bytes`, at `\n`, `\r` and `\r\n` only.
    typy_rt_bytes_splitlines => crate::string::typy_rt_str_splitlines,
    (data: Text, keepends: i64) -> *mut ListObject
);
forward!(
    /// `partition` and `rpartition` of `bytes`.
    typy_rt_bytes_partition => crate::string::typy_rt_str_partition,
    (data: Text, separator: Text, backwards: i64) -> Raising<*mut ListObject>
);
forward!(
    /// `join` of `bytes`.
    typy_rt_bytes_join => crate::string::typy_rt_str_join,
    (separator: Text, parts: *const ListObject) -> *mut StrObject
);
forward!(
    /// `strip`, `lstrip` and `rstrip` of `bytes`.
    typy_rt_bytes_strip => crate::string::typy_rt_str_strip,
    (data: Text, chars: Text, flags: i64) -> *mut StrObject
);
forward!(
    /// `removeprefix` and `removesuffix` of `bytes`.
    typy_rt_bytes_remove_affix => crate::string::typy_rt_str_remove_affix,
    (data: Text, affix: Text, suffix: i64) -> *mut StrObject
);
forward!(
    /// `replace` of `bytes`.
    typy_rt_bytes_replace => crate::string::typy_rt_str_replace,
    (data: Text, old: Text, new: Text, count: i64) -> *mut StrObject
);
forward!(
    /// `center`, `ljust` and `rjust` of `bytes`.
    typy_rt_bytes_pad => crate::string::typy_rt_str_pad,
    (data: Text, width: i64, fill: Text, mode: i64) -> Raising<*mut StrObject>
);
forward!(
    /// `zfill` of `bytes`.
    typy_rt_bytes_zfill => crate::string::typy_rt_str_zfill,
    (data: Text, width: i64) -> *mut StrObject
);
forward!(
    /// `expandtabs` of `bytes`.
    typy_rt_bytes_expandtabs => crate::string::typy_rt_str_expandtabs,
    (data: Text, tabsize: i64) -> *mut StrObject
);
forward!(
    /// `isascii` of `bytes`.
    typy_rt_bytes_isascii => crate::string::typy_rt_str_isascii,
    (data: Text) -> bool
);

const fn is_upper(byte: u32) -> bool {
    matches!(byte, 0x41..=0x5a)
}

const fn is_lower(byte: u32) -> bool {
    matches!(byte, 0x61..=0x7a)
}

const fn upper(byte: u32) -> u32 {
    if is_lower(byte) { byte - 0x20 } else { byte }
}

const fn lower(byte: u32) -> u32 {
    if is_upper(byte) { byte + 0x20 } else { byte }
}

/// Returns a new `bytes` with ASCII letters in the case `mode`, an `int` word, asks for: 0
/// `upper`, 1 `lower`, 3 `swapcase`, 4 `capitalize`, 5 `title`; other bytes stay as they are.
///
/// # Safety
///
/// `data` points to a live `bytes`, and `mode` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytes_case(data: Text, mode: i64) -> *mut StrObject {
    let mode = inline_value(mode);
    // SAFETY: the caller passes a live `bytes`; every index stays inside it.
    unsafe {
        let whole = length(data);
        let mut out = StrBuilder::with_capacity(whole);
        let mut previous_cased = false;
        for index in 0..whole {
            let byte = code_point(data, index);
            out.push(match mode {
                3 if is_upper(byte) => lower(byte),
                0 | 3 => upper(byte),
                4 if index == 0 => upper(byte),
                1 | 4 => lower(byte),
                _ => {
                    let cased = previous_cased;
                    previous_cased = is_upper(byte) || is_lower(byte);
                    if cased { lower(byte) } else { upper(byte) }
                }
            });
        }
        out.finish_like(data)
    }
}

/// Returns the ASCII test `question`, an `int` word, names: 0 `isalpha`, 1 `isalnum`,
/// 3 `isdigit`, 5 `isspace`, 6 `isupper`, 7 `islower`, 8 `istitle`. An empty `bytes` fails each.
///
/// # Safety
///
/// `data` points to a live `bytes`, and `question` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytes_test(data: Text, question: i64) -> bool {
    // SAFETY: the caller passes a live `bytes`; every index stays inside it.
    unsafe {
        let whole = length(data);
        let bytes = || (0..whole).map(|index| code_point(data, index));
        let every = |test: fn(u32) -> bool| whole > 0 && bytes().all(test);
        match inline_value(question) {
            0 => every(|byte| is_upper(byte) || is_lower(byte)),
            1 => every(|byte| is_upper(byte) || is_lower(byte) || byte.wrapping_sub(0x30) < 10),
            3 => every(|byte| byte.wrapping_sub(0x30) < 10),
            5 => every(|byte| matches!(byte, 0x20 | 0x09..=0x0d)),
            6 => bytes().any(is_upper) && !bytes().any(is_lower),
            7 => bytes().any(is_lower) && !bytes().any(is_upper),
            _ => {
                let (mut found, mut previous_cased) = (false, false);
                for byte in bytes() {
                    if is_upper(byte) {
                        if previous_cased {
                            return false;
                        }
                        (previous_cased, found) = (true, true);
                    } else if is_lower(byte) {
                        if !previous_cased {
                            return false;
                        }
                        (previous_cased, found) = (true, true);
                    } else {
                        previous_cased = false;
                    }
                }
                found
            }
        }
    }
}

/// Returns `data.hex()`: two lowercase hexadecimal digits for each byte, as a new `str`.
///
/// # Safety
///
/// `data` points to a live `bytes`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytes_hex(data: Text) -> *mut StrObject {
    // SAFETY: the caller passes a live `bytes`; every index stays inside it.
    unsafe {
        let whole = length(data);
        let mut out = StrBuilder::with_capacity(2 * whole);
        for index in 0..whole {
            let byte = code_point(data, index);
            for digit in [byte >> 4, byte & 0xf] {
                out.push(u32::from(
                    b"0123456789abcdef"[usize::try_from(digit).unwrap_or(0)],
                ));
            }
        }
        out.finish()
    }
}

#[cfg(test)]
mod tests;
