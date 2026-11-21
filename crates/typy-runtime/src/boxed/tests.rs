//! Tests of the values a value of type `object` holds boxed (TYP-23).

use super::{
    numbers_equal, typy_rt_box_bool, typy_rt_box_float, typy_rt_box_int, typy_rt_box_none,
    typy_rt_unbox_bool, typy_rt_unbox_float, typy_rt_unbox_int,
};
use crate::header::{IMMORTAL, ObjectHeader};
use crate::refcount::typy_rt_decrement;
use crate::string::{StrObject, from_str};
use crate::type_object::{
    type_of, typy_rt_object_is_true, typy_rt_object_repr, typy_rt_objects_equal,
};
use crate::word::inline;

fn text(string: *mut StrObject) -> String {
    // SAFETY: the string is live.
    unsafe {
        (0..crate::string::length(string))
            .map(|index| char::from_u32(crate::string::code_point(string, index)).unwrap())
            .collect()
    }
}

/// Returns `repr(object)` as a Rust string, releasing the text.
fn repr(object: *mut ObjectHeader) -> String {
    // SAFETY: the object is live.
    let written = unsafe { typy_rt_object_repr(object) }.value;
    let result = text(written);
    // SAFETY: the text is owned here.
    unsafe { typy_rt_decrement(written.cast()) };
    result
}

/// Returns the name `type()` gives the object.
fn type_name(object: *mut ObjectHeader) -> String {
    // SAFETY: the object is live.
    unsafe { type_of(object) }.name().to_owned()
}

/// Returns the word of a large integer written in decimal.
fn large(digits: &str) -> i64 {
    let written = from_str(digits);
    // SAFETY: the text is live, and base 10 is an inline word.
    let word = unsafe { crate::int::typy_rt_int_from_str(written, inline(10).unwrap()) }.value;
    // SAFETY: the text is owned here.
    unsafe { typy_rt_decrement(written.cast()) };
    word
}

#[test]
fn typ_23_an_int_boxed_is_written_typed_and_unboxed_as_the_int() {
    let seven = inline(7).unwrap();
    // SAFETY: an inline word has no large integer.
    let boxed = unsafe { typy_rt_box_int(seven) };
    assert_eq!(repr(boxed), "7");
    assert_eq!(type_name(boxed), "int");
    // SAFETY: the box is live.
    unsafe {
        assert_eq!(typy_rt_unbox_int(boxed), seven);
        assert!(typy_rt_object_is_true(boxed));
        typy_rt_decrement(boxed);
    }
    // SAFETY: as above.
    let zero = unsafe { typy_rt_box_int(inline(0).unwrap()) };
    // SAFETY: the box is live.
    unsafe {
        assert!(!typy_rt_object_is_true(zero));
        typy_rt_decrement(zero);
    }
}

#[test]
fn typ_23_a_boxed_large_int_holds_a_reference_of_its_own() {
    let word = large("123456789012345678901234567890");
    let object = crate::int::object_of(word).cast::<ObjectHeader>();
    // SAFETY: the word is a live large integer.
    unsafe {
        let boxed = typy_rt_box_int(word);
        assert_eq!((*object).reference_count, 2);
        assert_eq!(repr(boxed), "123456789012345678901234567890");
        let unboxed = typy_rt_unbox_int(boxed);
        assert_eq!(unboxed, word);
        assert_eq!((*object).reference_count, 3);
        crate::refcount::typy_rt_int_decrement(unboxed);
        typy_rt_decrement(boxed);
        assert_eq!((*object).reference_count, 1);
        crate::refcount::typy_rt_int_decrement(word);
    }
}

#[test]
fn typ_23_a_float_boxed_is_written_typed_and_unboxed_as_the_float() {
    let boxed = typy_rt_box_float(1.5);
    assert_eq!(repr(boxed), "1.5");
    assert_eq!(type_name(boxed), "float");
    // SAFETY: the box is live.
    unsafe {
        assert!((typy_rt_unbox_float(boxed) - 1.5).abs() < f64::EPSILON);
        typy_rt_decrement(boxed);
    }
    let zero = typy_rt_box_float(0.0);
    // SAFETY: the box is live.
    unsafe {
        assert!(!typy_rt_object_is_true(zero));
        typy_rt_decrement(zero);
    }
}

#[test]
fn typ_23_true_false_and_none_are_immortal_objects_of_their_types() {
    let (yes, no, none) = (
        typy_rt_box_bool(true),
        typy_rt_box_bool(false),
        typy_rt_box_none(),
    );
    assert_eq!(
        (repr(yes), repr(no), repr(none)),
        ("True".to_owned(), "False".to_owned(), "None".to_owned())
    );
    assert_eq!(type_name(yes), "bool");
    assert_eq!(type_name(none), "NoneType");
    assert!(typy_rt_unbox_bool(yes) && !typy_rt_unbox_bool(no));
    assert_eq!(yes, typy_rt_box_bool(true));
    // SAFETY: each object is immortal and live.
    unsafe {
        assert_eq!((*yes).reference_count, IMMORTAL);
        assert!(typy_rt_object_is_true(yes));
        assert!(!typy_rt_object_is_true(no));
        assert!(!typy_rt_object_is_true(none));
        assert_eq!(typy_rt_unbox_int(yes), inline(1).unwrap());
    }
}

#[test]
fn typ_23_an_int_a_float_and_a_bool_compare_by_their_numbers() {
    // SAFETY: each box is live until it is released at the end.
    unsafe {
        let one = typy_rt_box_int(inline(1).unwrap());
        let one_float = typy_rt_box_float(1.0);
        let half = typy_rt_box_float(0.5);
        let yes = typy_rt_box_bool(true);
        let text = from_str("1").cast::<ObjectHeader>();
        assert!(typy_rt_objects_equal(one, one_float));
        assert!(typy_rt_objects_equal(one_float, yes));
        assert!(typy_rt_objects_equal(yes, one));
        assert!(!typy_rt_objects_equal(one, half));
        assert!(!typy_rt_objects_equal(one, text));
        assert!(!typy_rt_objects_equal(text, one));
        assert!(!numbers_equal(one, text));
        let hash = |object: *mut ObjectHeader| type_of(object).hash.unwrap()(object);
        assert_eq!(hash(one), hash(one_float));
        assert_eq!(hash(one), hash(yes));
        assert_eq!(hash(half), 1_152_921_504_606_846_976);
        for each in [one, one_float, half, text] {
            typy_rt_decrement(each);
        }
    }
}

#[test]
fn typ_23_a_large_int_equals_a_float_only_when_the_values_are_equal() {
    // 2**53 + 1 has no `float`; 2**53 has.
    let above = large("9007199254740993");
    let exact = large("9007199254740992");
    // SAFETY: each word is a live large integer, and each box is released at the end.
    unsafe {
        let (above_box, exact_box) = (typy_rt_box_int(above), typy_rt_box_int(exact));
        #[allow(clippy::cast_precision_loss, reason = "2**53 is a float exactly")]
        let float = typy_rt_box_float(9_007_199_254_740_992_i64 as f64);
        assert!(!typy_rt_objects_equal(above_box, float));
        assert!(typy_rt_objects_equal(exact_box, float));
        for each in [above_box, exact_box, float] {
            typy_rt_decrement(each);
        }
        crate::refcount::typy_rt_int_decrement(above);
        crate::refcount::typy_rt_int_decrement(exact);
    }
}
