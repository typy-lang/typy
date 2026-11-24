//! Tests of `KeyError` as the runtime raises it.

use super::{key_error, key_error_text};
use crate::list::{ItemKind, ItemType};
use crate::refcount::typy_rt_decrement;
use crate::string::{StrObject, from_str};
use crate::tuple::{Field, TupleType};
use crate::word::inline;

/// Returns the text of a string, and releases it.
fn taken(string: *mut StrObject) -> String {
    // SAFETY: the string is live and owned here.
    unsafe {
        let text = (0..crate::string::length(string))
            .map(|index| char::from_u32(crate::string::code_point(string, index)).unwrap())
            .collect();
        typy_rt_decrement(string.cast());
        text
    }
}

/// Returns `str` and `repr` of an exception, and releases it.
fn shown(exception: *mut super::ExceptionObject) -> (String, String) {
    // SAFETY: the exception is live and owned here.
    unsafe {
        let text = crate::exception::typy_rt_exception_text(exception, inline(0).unwrap());
        let repr = crate::exception::typy_rt_exception_text(exception, inline(1).unwrap());
        typy_rt_decrement(exception.cast());
        (taken(text.value), taken(repr.value))
    }
}

#[test]
fn a_missing_string_key_is_the_argument_and_its_repr_the_text() {
    let key = from_str("k");
    // SAFETY: the key is live.
    let error = unsafe {
        key_error(
            ItemType::from(ItemKind::Object),
            key.expose_provenance() as u64,
        )
    };
    assert_eq!(shown(error), ("'k'".to_owned(), "KeyError('k')".to_owned()));
    // SAFETY: the key is live; the exception gave back the reference it took.
    unsafe {
        assert_eq!((*key).header.reference_count, 1);
        typy_rt_decrement(key.cast());
    }
}

#[test]
fn a_missing_tuple_key_is_kept_whole_as_the_argument() {
    let fields = Box::leak(Box::new([
        Field::plain(ItemKind::Object, 0),
        Field::plain(ItemKind::Int, 8),
    ]));
    let pair = Box::leak(Box::new(TupleType {
        size: 16,
        length: 2,
        fields: fields.as_ptr(),
    }));
    let name = from_str("a");
    let words = [
        name.expose_provenance() as u64,
        inline(1).unwrap().cast_unsigned(),
    ];
    // SAFETY: the words are a live tuple of the descriptor.
    let error = unsafe {
        key_error(
            ItemType::tuple(pair),
            words.as_ptr().expose_provenance() as u64,
        )
    };
    assert_eq!(
        shown(error),
        ("('a', 1)".to_owned(), "KeyError(('a', 1))".to_owned())
    );
    // SAFETY: the string is live; the exception gave back the reference it took.
    unsafe {
        assert_eq!((*name).header.reference_count, 1);
        typy_rt_decrement(name.cast());
    }
}

#[test]
fn a_key_error_of_a_text_shows_it_quoted_and_keeps_it_plain_as_the_argument() {
    assert_eq!(
        shown(key_error_text("pop from an empty set")),
        (
            "'pop from an empty set'".to_owned(),
            "KeyError('pop from an empty set')".to_owned()
        )
    );
}
