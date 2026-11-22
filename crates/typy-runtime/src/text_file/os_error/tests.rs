//! Tests of `OSError` and its subclasses for an error number.

use super::{typy_rt_os_error_new, typy_rt_os_error_str};
use crate::exception::classes::{FILE_NOT_FOUND_ERROR, OS_ERROR};
use crate::list::ItemKind;
use crate::refcount::typy_rt_decrement;
use crate::string::{StrObject, code_point, from_str, length};
use crate::tuple::{Field, TupleType};
use crate::type_object::TypeObject;
use crate::word::inline;

/// Returns the text of a string, and releases it.
fn taken(string: *mut StrObject) -> String {
    // SAFETY: the string is live and owned here.
    unsafe {
        let text = (0..length(string))
            .map(|index| char::from_u32(code_point(string, index)).unwrap())
            .collect();
        typy_rt_decrement(string.cast());
        text
    }
}

/// Returns a layout of these kinds, eight bytes apart.
fn layout(kinds: &[ItemKind]) -> &'static TupleType {
    let fields: Vec<Field> = kinds
        .iter()
        .zip(0..)
        .map(|(kind, index)| Field::plain(*kind, index * 8))
        .collect();
    let fields = fields.leak();
    Box::leak(Box::new(TupleType {
        size: 8 * fields.len() as u64,
        length: fields.len() as u64,
        fields: fields.as_ptr(),
    }))
}

/// Returns the class name and `str` of an `OSError` of `class` built with these items.
fn built(class: &TypeObject, kinds: &[ItemKind], items: &[u64]) -> (&'static str, String) {
    // SAFETY: the items are live values of the layout, which the exception copies.
    unsafe {
        let error = typy_rt_os_error_new(class, layout(kinds), items.as_ptr().cast());
        let written = typy_rt_os_error_str(error.cast());
        assert!(written.exception.is_null());
        let found = ((*error).class().name(), taken(written.value));
        typy_rt_decrement(error.cast());
        found
    }
}

#[test]
fn os_error_picks_its_subclass_only_for_a_small_int_or_a_bool() {
    let gone = from_str("gone");
    let word = |object: *mut StrObject| object.expose_provenance() as u64;
    let small = |value: i64| inline(value).unwrap().cast_unsigned();
    let big = crate::int::value::Integer {
        negative: false,
        magnitude: crate::int::natural::shift_left(&[1], 70),
    }
    .into_word();
    let (int, text_kind) = (ItemKind::Int, ItemKind::Object);
    let cases = [
        (
            &OS_ERROR,
            int,
            small(2),
            "FileNotFoundError",
            "[Errno 2] gone",
        ),
        (
            &OS_ERROR,
            int,
            big.cast_unsigned(),
            "OSError",
            "[Errno 1180591620717411303424] gone",
        ),
        (
            &OS_ERROR,
            ItemKind::Bool,
            1,
            "PermissionError",
            "[Errno True] gone",
        ),
        (
            &OS_ERROR,
            text_kind,
            word(gone),
            "OSError",
            "[Errno gone] gone",
        ),
        (
            &FILE_NOT_FOUND_ERROR,
            int,
            small(13),
            "FileNotFoundError",
            "[Errno 13] gone",
        ),
    ];
    for (class, kind, errno, name, text) in cases {
        assert_eq!(
            built(class, &[kind, text_kind], &[errno, word(gone)]),
            (name, text.to_owned()),
            "{text}"
        );
    }
    // SAFETY: the test holds the only references.
    unsafe {
        crate::refcount::typy_rt_int_decrement(big);
        typy_rt_decrement(gone.cast());
    }
}

#[test]
fn the_file_names_are_written_with_repr_unless_they_are_none() {
    let (gone, name, other) = (from_str("gone"), from_str("a"), from_str("b"));
    let word = |object: *mut StrObject| object.expose_provenance() as u64;
    let two = inline(2).unwrap().cast_unsigned();
    let (int, text_kind) = (ItemKind::Int, ItemKind::Object);
    let optional = ItemKind::OptionalObject;
    let five = [int, text_kind, optional, ItemKind::NoneValue, optional];
    let cases: [(&[ItemKind], [u64; 5], &str); 4] = [
        (
            &five,
            [two, word(gone), word(name), 0, word(other)],
            "[Errno 2] gone: 'a' -> 'b'",
        ),
        (
            &five,
            [two, word(gone), 0, 0, word(other)],
            "[Errno 2] gone",
        ),
        (
            &five,
            [two, word(gone), word(name), 0, 0],
            "[Errno 2] gone: 'a'",
        ),
        (
            &[int, text_kind, int],
            [two, word(gone), two, 0, 0],
            "[Errno 2] gone: 2",
        ),
    ];
    for (kinds, items, text) in cases {
        assert_eq!(
            built(&OS_ERROR, kinds, &items[..kinds.len()]).1,
            text,
            "{text}"
        );
    }
    // SAFETY: the test holds the only references.
    unsafe {
        for object in [gone, name, other] {
            typy_rt_decrement(object.cast());
        }
    }
}

#[test]
fn an_os_error_of_one_argument_or_of_six_is_written_as_base_exception_writes_it() {
    let plain = crate::exception::raise_class(&OS_ERROR, from_str("plain"));
    let one = from_str("one");
    let word = one.expose_provenance() as u64;
    let six = [ItemKind::Object; 6];
    // SAFETY: the exceptions and the string are live and owned here.
    unsafe {
        assert_eq!(taken(typy_rt_os_error_str(plain.cast()).value), "plain");
        typy_rt_decrement(plain.cast());
        let error = crate::exception::typy_rt_exception_new(
            &raw const OS_ERROR,
            layout(&six),
            [word; 6].as_ptr().cast(),
        );
        assert_eq!(
            taken(typy_rt_os_error_str(error.cast()).value),
            "('one', 'one', 'one', 'one', 'one', 'one')"
        );
        typy_rt_decrement(error.cast());
        typy_rt_decrement(one.cast());
    }
}

#[test]
fn an_error_about_a_descriptor_holds_it_as_the_file_name() {
    let error = super::os_error_of_descriptor(21, 3);
    // SAFETY: the exception is live and owned here, with three arguments.
    unsafe {
        assert_eq!((*error).class().name(), "IsADirectoryError");
        let layout = (*error).arguments_type;
        let address = (*error).arguments.expose_provenance() as u64;
        assert_eq!((*layout).length, 3);
        assert_eq!(crate::tuple::field_type(layout, 2).kind(), ItemKind::Int);
        assert_eq!(
            crate::tuple::field(layout, address, 2),
            inline(3).unwrap().cast_unsigned()
        );
        assert_eq!(
            taken(typy_rt_os_error_str(error.cast()).value),
            "[Errno 21] Is a directory: 3"
        );
        typy_rt_decrement(error.cast());
    }
}

#[test]
fn an_errno_whose_str_raises_raises_from_str() {
    let gone = from_str("gone");
    // SAFETY: both operands are `int` words; `10 ** 5000` has more digits than `str`
    // writes.
    let large =
        unsafe { crate::int::typy_rt_int_power(inline(10).unwrap(), inline(5000).unwrap()) }.value;
    let items = [large.cast_unsigned(), gone.expose_provenance() as u64];
    // SAFETY: the items are live values of the layout, which the exception copies.
    unsafe {
        let error = typy_rt_os_error_new(
            &raw const OS_ERROR,
            layout(&[ItemKind::Int, ItemKind::Object]),
            items.as_ptr().cast(),
        );
        let written = typy_rt_os_error_str(error.cast());
        assert!(!written.exception.is_null());
        typy_rt_decrement(written.exception.cast());
        typy_rt_decrement(error.cast());
        crate::refcount::typy_rt_int_decrement(large);
        typy_rt_decrement(gone.cast());
    }
}
