//! Tests of exceptions a call makes with `*` and `**`: their arguments, `args` read as a
//! `tuple[T, ...]`, and the keywords an exception class refuses.

use super::{
    typy_rt_exception_arguments_all, typy_rt_exception_arguments_tuple,
    typy_rt_exception_from_list, typy_rt_exception_keywords,
};
use crate::Raising;
use crate::exception::classes::VALUE_ERROR;
use crate::exception::text::{typy_rt_exception_repr, typy_rt_exception_str};
use crate::exception::{ExceptionKind, ExceptionObject, raise};
use crate::list::{ItemKind, ListObject, new_list, typy_rt_list_append};
use crate::refcount::typy_rt_decrement;
use crate::string::{StrObject, code_point, from_str, length};
use crate::tuple::{Field, TupleType};
use crate::word::inline;

/// Returns the text of a `str` and releases it.
fn text(result: Raising<*mut StrObject>) -> String {
    assert!(result.exception.is_null());
    // SAFETY: the string is live and owned here.
    unsafe {
        let value = (0..length(result.value))
            .map(|index| char::from_u32(code_point(result.value, index)).unwrap())
            .collect();
        typy_rt_decrement(result.value.cast());
        value
    }
}

/// Returns `str` and `repr` of an exception.
fn texts(exception: *mut ExceptionObject) -> (String, String) {
    // SAFETY: the exception is live.
    unsafe {
        (
            text(typy_rt_exception_str(exception.cast())),
            text(typy_rt_exception_repr(exception.cast())),
        )
    }
}

/// Returns a new list of `kind` holding `items`, each appended with a reference of its own.
fn list_of(kind: ItemKind, tuple: *const TupleType, items: &[u64]) -> *mut ListObject {
    let list = new_list(crate::list::ItemType::of(kind as u32, tuple), items.len());
    for item in items {
        // SAFETY: the list is live and each item is of its type.
        unsafe { typy_rt_list_append(list, *item) };
    }
    list
}

/// Returns the `int` word of an item kind, as the compiler passes it.
fn kind_word(kind: ItemKind) -> i64 {
    inline(kind as i64).unwrap()
}

/// Returns the `int` word of `value` as an item.
fn int(value: i64) -> u64 {
    inline(value).unwrap().cast_unsigned()
}

/// Makes an exception from `list`, then releases the list.
fn exception_of(list: *mut ListObject) -> *mut ExceptionObject {
    // SAFETY: the class and the list are live.
    unsafe {
        let exception = typy_rt_exception_from_list(&raw const VALUE_ERROR, list);
        typy_rt_decrement(list.cast());
        exception
    }
}

/// Returns the items of a `tuple[T, ...]` of plain items, and releases it.
fn items(tuple: *mut ListObject) -> Vec<u64> {
    // SAFETY: the tuple is live and owned here.
    unsafe {
        assert!((*tuple).is_tuple());
        let found = (0..(*tuple).length)
            .map(|index| (*tuple).item(index))
            .collect();
        typy_rt_decrement(tuple.cast());
        found
    }
}

#[test]
fn exp_22_an_exception_takes_the_values_of_a_list_as_its_arguments() {
    let exception = exception_of(list_of(ItemKind::Int, core::ptr::null(), &[int(1), int(2)]));
    assert_eq!(
        texts(exception),
        ("(1, 2)".to_owned(), "ValueError(1, 2)".to_owned())
    );
    // SAFETY: the exception is live.
    unsafe {
        assert!(typy_rt_exception_arguments_all(
            exception,
            kind_word(ItemKind::Int),
            0
        ));
        assert!(!typy_rt_exception_arguments_all(
            exception,
            kind_word(ItemKind::Object),
            0
        ));
        let tuple = typy_rt_exception_arguments_tuple(exception, kind_word(ItemKind::Int), 0);
        assert_eq!(items(tuple), [int(1), int(2)]);
        typy_rt_decrement(exception.cast());
    }
}

#[test]
fn exp_22_an_empty_list_gives_an_exception_without_arguments() {
    let exception = exception_of(list_of(ItemKind::Bool, core::ptr::null(), &[]));
    assert_eq!(texts(exception), (String::new(), "ValueError()".to_owned()));
    // SAFETY: the exception is live.
    unsafe {
        assert!(typy_rt_exception_arguments_all(
            exception,
            kind_word(ItemKind::Float),
            0
        ));
        let tuple = typy_rt_exception_arguments_tuple(exception, kind_word(ItemKind::Float), 0);
        assert_eq!(items(tuple), Vec::<u64>::new());
        typy_rt_decrement(exception.cast());
    }
}

#[test]
fn exp_22_the_arguments_hold_their_own_references_and_free_them_with_the_exception() {
    let word = from_str("word");
    let bits = word.expose_provenance() as u64;
    let exception = exception_of(list_of(ItemKind::Object, core::ptr::null(), &[bits]));
    // SAFETY: the string and the exception are live.
    unsafe {
        assert_eq!((*word).header.reference_count, 2);
        assert_eq!(
            texts(exception),
            ("word".to_owned(), "ValueError('word')".to_owned())
        );
        let tuple = typy_rt_exception_arguments_tuple(exception, kind_word(ItemKind::Object), 0);
        assert_eq!((*word).header.reference_count, 3);
        assert_eq!(items(tuple), [bits]);
        typy_rt_decrement(exception.cast());
        assert_eq!((*word).header.reference_count, 1);
        typy_rt_decrement(word.cast());
    }
}

#[test]
fn exp_22_booleans_take_a_word_each_and_read_as_booleans() {
    let exception = exception_of(list_of(ItemKind::Bool, core::ptr::null(), &[1, 0, 1]));
    assert_eq!(
        texts(exception),
        (
            "(True, False, True)".to_owned(),
            "ValueError(True, False, True)".to_owned()
        )
    );
    // SAFETY: the exception is live.
    unsafe {
        let tuple = typy_rt_exception_arguments_tuple(exception, kind_word(ItemKind::Bool), 0);
        assert_eq!(items(tuple), [1, 0, 1]);
        typy_rt_decrement(exception.cast());
    }
}

#[test]
fn exp_22_tuple_items_are_matched_by_their_layout() {
    let fields: &'static [Field] = Box::leak(Box::new([
        Field::plain(ItemKind::Int, 0),
        Field::plain(ItemKind::Int, 8),
    ]));
    let pair: &'static TupleType = Box::leak(Box::new(TupleType {
        size: 16,
        length: 2,
        fields: fields.as_ptr(),
    }));
    let other: &'static TupleType = Box::leak(Box::new(TupleType {
        size: 8,
        length: 1,
        fields: fields.as_ptr(),
    }));
    let words = [int(3), int(4)];
    let exception = exception_of(list_of(
        ItemKind::Tuple,
        pair,
        &[words.as_ptr().expose_provenance() as u64],
    ));
    let address = |tuple: &'static TupleType| core::ptr::from_ref(tuple).expose_provenance() as u64;
    assert_eq!(
        texts(exception),
        ("(3, 4)".to_owned(), "ValueError((3, 4))".to_owned())
    );
    // SAFETY: the exception is live.
    unsafe {
        let tuple = kind_word(ItemKind::Tuple);
        assert!(typy_rt_exception_arguments_all(
            exception,
            tuple,
            address(pair)
        ));
        assert!(!typy_rt_exception_arguments_all(
            exception,
            tuple,
            address(other)
        ));
        assert!(!typy_rt_exception_arguments_all(
            exception,
            kind_word(ItemKind::Int),
            0
        ));
        let read = typy_rt_exception_arguments_tuple(exception, tuple, address(pair));
        assert_eq!((*read).length, 1);
        typy_rt_decrement(read.cast());
        typy_rt_decrement(exception.cast());
    }
}

#[test]
fn exp_22_the_runtime_s_text_reads_as_one_str() {
    let exception = raise(ExceptionKind::ValueError, "bad");
    // SAFETY: the exception is live.
    unsafe {
        assert!(typy_rt_exception_arguments_all(
            exception,
            kind_word(ItemKind::Object),
            0
        ));
        assert!(!typy_rt_exception_arguments_all(
            exception,
            kind_word(ItemKind::Int),
            0
        ));
        let tuple = typy_rt_exception_arguments_tuple(exception, kind_word(ItemKind::Object), 0);
        assert_eq!((*tuple).length, 1);
        typy_rt_decrement(tuple.cast());
        typy_rt_decrement(exception.cast());
    }
}

#[test]
fn exp_22_an_exception_class_refuses_any_keyword() {
    // SAFETY: the dicts and the class are live.
    unsafe {
        let dict = crate::dict::typy_rt_dict_new(
            kind_word(ItemKind::Object),
            0,
            kind_word(ItemKind::Int),
            0,
            inline(0).unwrap(),
        );
        assert!(typy_rt_exception_keywords(&raw const VALUE_ERROR, dict).is_null());
        let key = from_str("code");
        crate::dict::typy_rt_dict_set(dict, key.expose_provenance() as u64, int(1));
        typy_rt_decrement(key.cast());
        let error = typy_rt_exception_keywords(&raw const VALUE_ERROR, dict);
        assert_eq!(
            texts(error).0,
            "ValueError() takes no keyword arguments".to_owned()
        );
        typy_rt_decrement(error.cast());
        typy_rt_decrement(dict.cast());
    }
}
