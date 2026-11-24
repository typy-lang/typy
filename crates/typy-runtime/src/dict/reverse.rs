//! Steps backwards through a dict's entries for `reversed` of a dict or a view, as CPython's
//! `dictreviter_iternext` does (DAT-16).

use super::DictObject;
use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::word::{inline, inline_value};

/// Returns the `int` word of a count or an index.
fn word(value: usize) -> i64 {
    inline(i64::try_from(value).unwrap_or(i64::MAX)).unwrap_or(1)
}

/// Returns the index of the last entry of the dict, used or a hole, as an `int` word, or -1
/// when it has none: where `reversed` starts.
///
/// # Safety
///
/// `dict` points to a live dict.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_last_entry(dict: *const DictObject) -> i64 {
    // SAFETY: the caller's guarantee.
    let filled = unsafe { (*dict).filled };
    inline(i64::try_from(filled).unwrap_or(i64::MAX) - 1).unwrap_or(1)
}

/// Returns the last entry at or before the `int` word `position` that holds an item, as an
/// `int` word, or -1 when there is none. A dict whose number of items is no longer `length`
/// raises `RuntimeError: dictionary changed size during iteration`; unlike the forward
/// iterator, CPython's reverse iterator does not notice keys that changed at the same size.
///
/// # Safety
///
/// `dict` points to a live dict, and the others are `int` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_previous(
    dict: *const DictObject,
    position: i64,
    length: i64,
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
        // Entries past the last one hold no item, as in CPython, where a table rebuilt at
        // the same size has fewer entries than the position the iterator kept.
        let none = inline(-1).unwrap_or(1);
        let Ok(start) = usize::try_from(inline_value(position)) else {
            return Raising::ok(none);
        };
        let found = (0..dict.filled.min(start + 1))
            .rev()
            .find(|index| dict.is_live(*index));
        Raising::ok(found.map_or(none, word))
    }
}

#[cfg(test)]
mod tests;
