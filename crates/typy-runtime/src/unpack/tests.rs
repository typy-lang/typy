use super::typy_rt_unpack;
use crate::exception::{ExceptionKind, ExceptionObject};
use crate::list::{ItemKind, ListObject, typy_rt_list_append, typy_rt_list_new};

fn word(value: i64) -> i64 {
    crate::word::inline(value).unwrap()
}

/// Returns a new list of the `int` values.
fn ints(values: &[i64]) -> *mut ListObject {
    let list = typy_rt_list_new(word(ItemKind::Int as i64), word(0), 0);
    for value in values {
        // SAFETY: the list is live.
        unsafe { typy_rt_list_append(list, word(*value).cast_unsigned()) };
    }
    list
}

/// Unpacks `source` for `before` and `after` targets, and a starred one when `starred`.
fn unpack(
    source: *mut ListObject,
    before: i64,
    after: i64,
    starred: bool,
) -> (Option<Vec<i64>>, *mut ExceptionObject) {
    // SAFETY: the source is a live list of `int` values.
    let result = unsafe {
        typy_rt_unpack(
            source.expose_provenance() as u64,
            word(ItemKind::Int as i64),
            0,
            word(before),
            word(after),
            starred,
        )
    };
    if !result.exception.is_null() {
        return (None, result.exception);
    }
    // SAFETY: the result is a live list of `int` values that the test owns.
    unsafe {
        let list = result.value;
        let values = (0..(*list).length)
            .map(|index| crate::word::inline_value((*list).item(index).cast_signed()))
            .collect();
        crate::refcount::typy_rt_decrement(list.cast());
        (Some(values), core::ptr::null_mut())
    }
}

/// Returns the text of a raised `ValueError`, which is released.
fn message(exception: *mut ExceptionObject) -> String {
    // SAFETY: the exception is live and owned here.
    unsafe {
        assert_eq!((*exception).kind(), ExceptionKind::ValueError);
        let text = (*exception).message;
        let found = (0..crate::string::length(text))
            .map(|index| char::from_u32(crate::string::code_point(text, index)).unwrap())
            .collect();
        crate::refcount::typy_rt_decrement(exception.cast());
        found
    }
}

#[test]
fn smt_02_unpacking_gives_the_values_or_cpython_s_value_error() {
    let source = ints(&[1, 2, 3]);
    assert_eq!(unpack(source, 3, 0, false).0, Some(vec![1, 2, 3]));
    assert_eq!(unpack(source, 1, 1, true).0, Some(vec![1, 2, 3]));
    assert_eq!(
        message(unpack(source, 2, 0, false).1),
        "too many values to unpack (expected 2, got 3)"
    );
    assert_eq!(
        message(unpack(source, 4, 0, false).1),
        "not enough values to unpack (expected 4, got 3)"
    );
    assert_eq!(
        message(unpack(source, 2, 2, true).1),
        "not enough values to unpack (expected at least 4, got 3)"
    );
    // SAFETY: the test owns the list.
    unsafe { crate::refcount::typy_rt_decrement(source.cast()) };
}
