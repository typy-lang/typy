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
mod tests {
    use super::{typy_rt_dict_last_entry, typy_rt_dict_previous};
    use crate::dict::{typy_rt_dict_new, typy_rt_dict_set};
    use crate::list::ItemKind;
    use crate::word::{inline, inline_value};

    fn word(value: i64) -> i64 {
        inline(value).unwrap()
    }

    #[test]
    fn reversed_entries_skip_holes_and_notice_a_change_of_size() {
        let int = word(ItemKind::Int as i64);
        let dict = typy_rt_dict_new(int, 0, int, 0, word(0));
        // SAFETY: the dict is live, and every key and value is an inline word.
        unsafe {
            assert_eq!(inline_value(typy_rt_dict_last_entry(dict)), -1);
            for key in 0..4 {
                typy_rt_dict_set(dict, word(key).cast_unsigned(), word(key).cast_unsigned());
            }
            let popped = crate::dict::typy_rt_dict_pop(dict, word(2).cast_unsigned());
            assert!(popped.exception.is_null());
            let last = typy_rt_dict_last_entry(dict);
            assert_eq!(inline_value(last), 3);
            let length = word(3);
            let mut seen = Vec::new();
            let mut position = last;
            loop {
                let found = typy_rt_dict_previous(dict, position, length);
                assert!(found.exception.is_null());
                let index = inline_value(found.value);
                if index < 0 {
                    break;
                }
                seen.push(index);
                position = word(index - 1);
            }
            assert_eq!(seen, [3, 1, 0]);
            assert_eq!(
                inline_value(typy_rt_dict_previous(dict, word(99), length).value),
                3
            );
            let changed = typy_rt_dict_previous(dict, last, word(4));
            // SAFETY: the exception is live and owned here.
            assert_eq!(
                (*changed.exception).kind(),
                crate::ExceptionKind::RuntimeError
            );
            crate::refcount::typy_rt_decrement(changed.exception.cast());
            crate::refcount::typy_rt_decrement(dict.cast());
        }
    }
}
