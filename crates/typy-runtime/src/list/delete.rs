//! `del` of a list item and of a list slice (SMT-05), with CPython's results and messages.

use super::{ListObject, take_out};
use crate::exception::ExceptionObject;
use crate::slice::select;

/// Removes the item at `index`, counted from the end when negative, and releases it, as
/// `del list[index]` does; an index outside the list raises `IndexError: list assignment
/// index out of range`.
///
/// # Safety
///
/// `list` points to a live list, and `index` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_delete(
    list: *mut ListObject,
    index: i64,
) -> *mut ExceptionObject {
    // SAFETY: the caller passes a live list; `position` keeps the index inside it.
    unsafe {
        match super::operations::position(
            index,
            (*list).length,
            "list assignment index out of range",
        ) {
            Ok(position) => {
                let item = take_out(list, position);
                (*list).item_type().release(item);
                core::ptr::null_mut()
            }
            Err(error) => error,
        }
    }
}

/// Removes the items `list[start:stop:step]` selects and releases them, as `del` of a slice
/// does; `present` says which bounds are given, and a zero step raises `ValueError`.
///
/// # Safety
///
/// `list` points to a live list, and the bounds are `int` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_list_delete_slice(
    list: *mut ListObject,
    start: i64,
    stop: i64,
    stride: i64,
    present: i64,
) -> *mut ExceptionObject {
    // SAFETY: the caller passes a live list; every position read or written is inside it.
    unsafe {
        let length = (*list).length;
        let selection = match select(start, stop, stride, present, length) {
            Ok(selection) => selection,
            Err(error) => return error,
        };
        if selection.count == 0 {
            return core::ptr::null_mut();
        }
        // The selected positions, lowest first, are those from `lowest` every `gap`.
        let final_position = selection.position(selection.count - 1);
        let lowest = selection.position(0).min(final_position);
        let highest = selection.position(0).max(final_position);
        let gap = usize::try_from(selection.step.unsigned_abs()).unwrap_or(1);
        let removed = |index: usize| {
            (lowest..=highest).contains(&index) && (index - lowest).is_multiple_of(gap)
        };
        let item_type = (*list).item_type();
        let words = (*list).words();
        let mut kept = 0;
        for index in 0..length {
            if removed(index) {
                item_type.release((*list).item(index));
            } else {
                if kept != index {
                    core::ptr::copy((*list).slot(index), (*list).slot(kept), words);
                }
                kept += 1;
            }
        }
        (*list).length = kept;
        core::ptr::null_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::{typy_rt_list_delete, typy_rt_list_delete_slice};
    use crate::list::{ItemKind, ListObject, typy_rt_list_append, typy_rt_list_new};
    use crate::word::{inline, inline_value};

    fn word(value: i64) -> i64 {
        inline(value).unwrap()
    }

    fn list_of(values: &[i64]) -> *mut ListObject {
        let list = typy_rt_list_new(word(ItemKind::Int as i64), word(0), 0);
        for value in values {
            // SAFETY: the list is live and the items are inline words.
            unsafe { typy_rt_list_append(list, word(*value).cast_unsigned()) };
        }
        list
    }

    fn values(list: *const ListObject) -> Vec<i64> {
        // SAFETY: the list is live.
        unsafe {
            (0..(*list).length)
                .map(|index| inline_value((*list).item(index).cast_signed()))
                .collect()
        }
    }

    fn release(list: *mut ListObject) {
        // SAFETY: the test holds the list's reference.
        unsafe { crate::refcount::typy_rt_decrement(list.cast()) };
    }

    #[test]
    fn deleting_an_item_counts_from_either_end_and_refuses_one_outside() {
        let list = list_of(&[1, 2, 3, 4]);
        // SAFETY: the list is live.
        unsafe {
            assert!(typy_rt_list_delete(list, word(1)).is_null());
            assert!(typy_rt_list_delete(list, word(-1)).is_null());
            assert_eq!(values(list), [1, 3]);
            let error = typy_rt_list_delete(list, word(2));
            assert_eq!((*error).kind(), crate::ExceptionKind::IndexError);
            crate::refcount::typy_rt_decrement(error.cast());
        }
        release(list);
    }

    #[test]
    fn deleting_a_slice_removes_what_the_slice_selects() {
        // The bounds, with their mask of start 1, stop 2 and step 4, and what stays.
        let cases: [(i64, i64, i64, i64, &[i64]); 6] = [
            (1, 3, 0, 3, &[0, 3, 4, 5]),
            (0, 0, 2, 4, &[1, 3, 5]),
            (0, 0, -2, 4, &[0, 2, 4]),
            (4, 0, -1, 5, &[5]),
            (0, 0, 0, 0, &[]),
            (3, 1, 0, 3, &[0, 1, 2, 3, 4, 5]),
        ];
        for (start, stop, stride, mask, kept) in cases {
            let list = list_of(&[0, 1, 2, 3, 4, 5]);
            // SAFETY: the list is live and the bounds are `int` words.
            let error = unsafe {
                typy_rt_list_delete_slice(list, word(start), word(stop), word(stride), word(mask))
            };
            assert!(error.is_null());
            assert_eq!(values(list), kept, "{start}:{stop}:{stride} mask {mask}");
            release(list);
        }
    }

    #[test]
    fn a_zero_step_raises_value_error() {
        let list = list_of(&[1]);
        // SAFETY: the list is live.
        unsafe {
            let error = typy_rt_list_delete_slice(list, word(0), word(0), word(0), word(4));
            assert_eq!((*error).kind(), crate::ExceptionKind::ValueError);
            crate::refcount::typy_rt_decrement(error.cast());
        }
        release(list);
    }
}
