//! Tests of lists: growth, items of each kind, positions, equality, membership, text and freeing.

use super::operations::{
    typy_rt_list_append, typy_rt_list_contains, typy_rt_list_equal, typy_rt_list_get,
    typy_rt_list_length, typy_rt_list_new, typy_rt_list_repr, typy_rt_list_set,
};
use super::{ItemKind, grown_capacity, new_list};
use crate::int::value::Integer;
use crate::string::{code_point, from_str, length};
use crate::word::{inline, inline_value};

fn text(string: *const crate::StrObject) -> String {
    // SAFETY: the string is live and every index is in bounds.
    (0..unsafe { length(string) })
        .map(|index| char::from_u32(unsafe { code_point(string, index) }).unwrap())
        .collect()
}

fn word(value: i64) -> i64 {
    inline(value).unwrap()
}

fn ints(values: &[i64]) -> *mut super::ListObject {
    let list = new_list(ItemKind::Int, 0);
    for value in values {
        // SAFETY: the list is live, and an inline word needs no reference.
        unsafe { typy_rt_list_append(list, word(*value).cast_unsigned()) };
    }
    list
}

fn repr(list: *const super::ListObject) -> String {
    // SAFETY: the list is live; its text is a new string.
    let result = unsafe { typy_rt_list_repr(list) };
    assert!(result.exception.is_null());
    text(result.value)
}

#[test]
fn a_million_ints_built_one_append_at_a_time_fit_in_eight_and_a_half_megabytes() {
    let list = new_list(ItemKind::Int, 0);
    for value in 0..1_000_000 {
        // SAFETY: the list is live, and an inline word needs no reference.
        unsafe { typy_rt_list_append(list, word(value).cast_unsigned()) };
    }
    // SAFETY: the list is live.
    let (length, capacity) = unsafe { ((*list).length, (*list).capacity) };
    let bytes = core::mem::size_of::<super::ListObject>() + capacity * 8;
    assert_eq!(length, 1_000_000);
    assert!(bytes <= 8_500_000, "{bytes} bytes");
}

#[test]
fn capacity_grows_geometrically() {
    assert_eq!(grown_capacity(0), 4);
    assert_eq!(grown_capacity(64), 76);
    assert_eq!(grown_capacity(1 << 20), (1 << 20) + (1 << 15) + 4);
}

#[test]
fn items_are_read_and_written_at_positions_counted_from_either_end() {
    let list = ints(&[10, 20, 30]);
    // SAFETY: the list is live, and the items are inline words.
    unsafe {
        assert_eq!(inline_value(typy_rt_list_length(list)), 3);
        assert_eq!(
            inline_value(typy_rt_list_get(list, word(-1)).value.cast_signed()),
            30
        );
        assert!(typy_rt_list_set(list, word(0), word(5).cast_unsigned()).is_null());
    }
    assert_eq!(repr(list), "[5, 20, 30]");
    for index in [3, -4] {
        // SAFETY: the list is live; the exceptions are live.
        unsafe {
            let read = typy_rt_list_get(list, word(index));
            assert_eq!(text((*read.exception).message), "list index out of range");
            let written = typy_rt_list_set(list, word(index), word(1).cast_unsigned());
            assert_eq!(
                text((*written).message),
                "list assignment index out of range"
            );
        }
    }
    let huge = Integer::from_i128(1 << 70).into_word();
    // SAFETY: the list is live; the word points to a live large integer.
    let read = unsafe { typy_rt_list_get(list, huge) };
    // SAFETY: the exception is live.
    let message = unsafe { (*read.exception).message };
    assert_eq!(
        text(message),
        "cannot fit 'int' into an index-sized integer"
    );
}

#[test]
fn equality_and_membership_compare_items_by_value() {
    let (first, second, other) = (ints(&[1, 2]), ints(&[1, 2]), ints(&[1, 3]));
    // SAFETY: the lists are live, and the item is an inline word.
    unsafe {
        assert!(typy_rt_list_equal(first, second));
        assert!(!typy_rt_list_equal(first, other));
        assert!(!typy_rt_list_equal(first, ints(&[1])));
        assert!(typy_rt_list_contains(first, word(2).cast_unsigned()));
        assert!(!typy_rt_list_contains(first, word(9).cast_unsigned()));
    }
}

#[test]
fn every_item_kind_is_written_as_cpython_writes_it() {
    let floats = typy_rt_list_new(word(ItemKind::Float as i64), word(2));
    let bools = new_list(ItemKind::Bool, 0);
    let strings = new_list(ItemKind::Object, 0);
    let nested = new_list(ItemKind::Object, 0);
    // SAFETY: the lists and items are live; the lists add references to the objects.
    unsafe {
        typy_rt_list_append(floats, 1.5_f64.to_bits());
        typy_rt_list_append(floats, (-0.0_f64).to_bits());
        typy_rt_list_append(bools, 1);
        typy_rt_list_append(bools, 0);
        typy_rt_list_append(strings, from_str("it's").expose_provenance() as u64);
        typy_rt_list_append(nested, ints(&[1, 2]).expose_provenance() as u64);
        typy_rt_list_append(
            nested,
            new_list(ItemKind::Int, 0).expose_provenance() as u64,
        );
    }
    assert_eq!(repr(floats), "[1.5, -0.0]");
    assert_eq!(repr(bools), "[True, False]");
    assert_eq!(repr(strings), "[\"it's\"]");
    assert_eq!(repr(nested), "[[1, 2], []]");
    assert_eq!(repr(new_list(ItemKind::Int, 0)), "[]");
    let copy = new_list(ItemKind::Object, 0);
    // SAFETY: the lists are live; the item is a live string.
    unsafe {
        typy_rt_list_append(copy, from_str("it's").expose_provenance() as u64);
        assert!(typy_rt_list_equal(strings, copy));
        assert!(typy_rt_list_equal(floats, floats));
    }
}

#[test]
fn freeing_a_list_releases_its_items() {
    let inner = ints(&[1]);
    let outer = new_list(ItemKind::Object, 0);
    let big = Integer::from_i128(1 << 70).into_word();
    let numbers = new_list(ItemKind::Int, 0);
    // SAFETY: the lists and items are live; each list adds a reference, and the last
    // references are dropped at the end.
    unsafe {
        typy_rt_list_append(outer, inner.expose_provenance() as u64);
        typy_rt_list_append(numbers, big.cast_unsigned());
        assert_eq!((*inner).header.reference_count, 2);
        crate::refcount::typy_rt_decrement(outer.cast());
        assert_eq!((*inner).header.reference_count, 1);
        crate::refcount::typy_rt_decrement(inner.cast());
        crate::refcount::typy_rt_decrement(numbers.cast());
        crate::refcount::typy_rt_int_decrement(big);
    }
}

#[test]
fn the_placeholder_is_an_immortal_empty_list() {
    let placeholder = (&raw const super::typy_rt_placeholder_list)
        .cast_mut()
        .cast::<crate::ObjectHeader>();
    // SAFETY: the placeholder is immortal, so counting it changes nothing.
    unsafe {
        crate::refcount::typy_rt_increment(placeholder);
        crate::refcount::typy_rt_decrement(placeholder);
        assert_eq!((*placeholder).reference_count, crate::header::IMMORTAL);
        assert_eq!(inline_value(typy_rt_list_length(placeholder.cast())), 0);
    }
}

#[test]
fn a_list_that_never_held_an_item_is_freed() {
    let list = new_list(ItemKind::Object, 0);
    // SAFETY: the list is live, and this drops its only reference.
    unsafe { crate::refcount::typy_rt_decrement(list.cast()) };
}

#[test]
#[cfg_attr(
    miri,
    ignore = "building a 4301-digit integer takes hours under Miri; the same code runs in the other tests"
)]
fn the_text_of_a_list_with_an_int_of_too_many_digits_raises() {
    let mut big = Integer::from_i128(1);
    let ten = Integer::from_i128(10);
    for _ in 0..4300 {
        big = big.multiply(&ten);
    }
    let list = ints(&[1]);
    // SAFETY: the list is live, and the word points to a live large integer.
    unsafe { typy_rt_list_append(list, big.into_word().cast_unsigned()) };
    // SAFETY: the list is live; the exception is live.
    unsafe {
        let result = typy_rt_list_repr(list);
        assert!(!result.exception.is_null());
        assert_eq!((*result.exception).kind, crate::ExceptionKind::ValueError);
    }
}
