//! Tests of `del` of list items and slices, and assignment to a list slice.

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

/// A slice's start, stop, step and mask, the items assigned to it, and the list after.
type Assignment = (i64, i64, i64, i64, &'static [i64], &'static [i64]);

#[test]
fn assigning_a_slice_replaces_what_it_selects() {
    let cases: [Assignment; 6] = [
        (1, 3, 0, 3, &[9], &[0, 9, 3, 4]),
        (1, 1, 0, 3, &[7, 8], &[0, 7, 8, 1, 2, 3, 4]),
        (0, 0, 0, 0, &[], &[]),
        (0, 0, 2, 4, &[5, 6, 7], &[5, 1, 6, 3, 7]),
        (0, 0, -2, 4, &[5, 6, 7], &[7, 1, 6, 3, 5]),
        (3, 1, 0, 3, &[9], &[0, 1, 2, 9, 3, 4]),
    ];
    for (start, stop, stride, mask, items, expected) in cases {
        let list = list_of(&[0, 1, 2, 3, 4]);
        let source = list_of(items);
        // SAFETY: both lists are live and the bounds are `int` words.
        let error = unsafe {
            super::typy_rt_list_assign_slice(
                list,
                word(start),
                word(stop),
                word(stride),
                word(mask),
                source,
            )
        };
        assert!(error.is_null());
        assert_eq!(
            values(list),
            expected,
            "{start}:{stop}:{stride} mask {mask}"
        );
        release(source);
        release(list);
    }
}

#[test]
fn a_list_assigned_to_a_slice_of_itself_is_copied_first() {
    let list = list_of(&[1, 2, 3]);
    // SAFETY: the list is live.
    let error =
        unsafe { super::typy_rt_list_assign_slice(list, word(1), word(0), word(0), word(1), list) };
    assert!(error.is_null());
    assert_eq!(values(list), [1, 1, 2, 3]);
    release(list);
}

#[test]
fn an_extended_slice_takes_exactly_as_many_items() {
    let list = list_of(&[0, 1, 2, 3]);
    let source = list_of(&[9]);
    // SAFETY: both lists are live.
    unsafe {
        let error =
            super::typy_rt_list_assign_slice(list, word(0), word(0), word(2), word(4), source);
        assert_eq!((*error).kind(), crate::ExceptionKind::ValueError);
        let message = (*error).message;
        let text: String = (0..crate::string::length(message))
            .map(|index| char::from_u32(crate::string::code_point(message, index)).unwrap())
            .collect();
        assert_eq!(
            text,
            "attempt to assign sequence of size 1 to extended slice of size 2"
        );
        crate::refcount::typy_rt_decrement(error.cast());
    }
    assert_eq!(values(list), [0, 1, 2, 3]);
    release(source);
    release(list);
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
