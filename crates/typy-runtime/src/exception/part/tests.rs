//! Tests of the arguments of an exception and the code of a `SystemExit`.

use super::{
    typy_rt_exception_args_length, typy_rt_exception_arguments_address,
    typy_rt_exception_arguments_match, typy_rt_exception_part_equal, typy_rt_exception_part_text,
    typy_rt_exception_part_truth,
};
use crate::exception::classes::{FILE_NOT_FOUND_ERROR, SYSTEM_EXIT, VALUE_ERROR};
use crate::exception::{ExceptionKind, ExceptionObject, raise, typy_rt_exception_new};
use crate::list::ItemKind;
use crate::refcount::typy_rt_decrement;
use crate::string::{code_point, length};
use crate::tuple::{Field, TupleType};
use crate::type_object::TypeObject;
use crate::word::{inline, inline_value};

/// The parts, as the `int` words the compiler passes.
fn args() -> i64 {
    inline(0).unwrap()
}

fn code() -> i64 {
    inline(1).unwrap()
}

/// Returns a layout of plain fields of these kinds, eight bytes apart.
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

/// Returns an exception of `class` built with these items of these kinds.
fn built(class: &TypeObject, kinds: &[ItemKind], words: &[u64]) -> *mut ExceptionObject {
    // SAFETY: the words are live items of the layout, and the exception keeps a copy.
    unsafe { typy_rt_exception_new(class, layout(kinds), words.as_ptr().cast()) }
}

fn text(object: *mut ExceptionObject, part: i64, repr: bool) -> String {
    let flag = inline(i64::from(repr)).unwrap();
    // SAFETY: the exception is live; the text is new and released once read.
    unsafe {
        let found = typy_rt_exception_part_text(object, part, flag).value;
        let written = (0..length(found))
            .map(|index| char::from_u32(code_point(found, index)).unwrap())
            .collect();
        typy_rt_decrement(found.cast());
        written
    }
}

/// Returns whether a part equals a value of `kind`, passed as a tuple of one.
fn equals(object: *mut ExceptionObject, part: i64, kind: ItemKind, word: u64) -> bool {
    let words = [word];
    // SAFETY: the exception and the tuple of one are live.
    unsafe {
        typy_rt_exception_part_equal(
            object,
            part,
            layout(&[kind]),
            words.as_ptr().expose_provenance() as u64,
        )
    }
}

fn release(object: *mut ExceptionObject) {
    // SAFETY: the test holds the only reference.
    unsafe { typy_rt_decrement(object.cast()) };
}

#[test]
fn args_is_the_tuple_of_the_arguments_or_of_the_runtimes_text() {
    let three = inline(3).unwrap().cast_unsigned();
    let two = built(&VALUE_ERROR, &[ItemKind::Int, ItemKind::Bool], &[three, 1]);
    let none = built(&VALUE_ERROR, &[], &[]);
    let raised = raise(ExceptionKind::ValueError, "bad");
    // SAFETY: the exceptions are live.
    unsafe {
        assert_eq!(text(two, args(), false), "(3, True)");
        assert_eq!(inline_value(typy_rt_exception_args_length(two)), 2);
        assert!(typy_rt_exception_part_truth(two, args()));
        assert_eq!(text(none, args(), true), "()");
        assert_eq!(inline_value(typy_rt_exception_args_length(none)), 0);
        assert!(!typy_rt_exception_part_truth(none, args()));
        assert_eq!(text(raised, args(), true), "('bad',)");
        assert_eq!(inline_value(typy_rt_exception_args_length(raised)), 1);
        assert!(typy_rt_exception_part_truth(raised, args()));
    }
    assert!(!equals(two, args(), ItemKind::Int, three));
    for exception in [two, none, raised] {
        release(exception);
    }
}

#[test]
fn code_is_none_the_one_argument_or_the_tuple_of_several() {
    let three = inline(3).unwrap().cast_unsigned();
    let zero = inline(0).unwrap().cast_unsigned();
    let bare = built(&SYSTEM_EXIT, &[], &[]);
    let one = built(&SYSTEM_EXIT, &[ItemKind::Int], &[three]);
    let falsy = built(&SYSTEM_EXIT, &[ItemKind::Int], &[zero]);
    let several = built(
        &SYSTEM_EXIT,
        &[ItemKind::Int, ItemKind::Int],
        &[three, zero],
    );
    assert_eq!(text(bare, code(), false), "None");
    assert_eq!(text(one, code(), true), "3");
    assert_eq!(text(one, code(), false), "3");
    assert_eq!(text(several, code(), false), "(3, 0)");
    // SAFETY: the exceptions are live.
    unsafe {
        assert!(!typy_rt_exception_part_truth(bare, code()));
        assert!(typy_rt_exception_part_truth(one, code()));
        assert!(!typy_rt_exception_part_truth(falsy, code()));
        assert!(typy_rt_exception_part_truth(several, code()));
    }
    assert!(equals(bare, code(), ItemKind::NoneValue, 0));
    assert!(!equals(one, code(), ItemKind::NoneValue, 0));
    assert!(equals(one, code(), ItemKind::Int, three));
    assert!(equals(one, code(), ItemKind::Float, 3.0_f64.to_bits()));
    assert!(!equals(one, code(), ItemKind::Int, zero));
    for exception in [bare, one, falsy, several] {
        release(exception);
    }
}

/// Returns whether a part equals the tuple of these items of these kinds.
fn equals_tuple(
    object: *mut ExceptionObject,
    part: i64,
    kinds: &[ItemKind],
    words: &[u64],
) -> bool {
    let inner = layout(kinds);
    let outer = Box::leak(Box::new(TupleType {
        size: inner.size.max(8),
        length: 1,
        fields: Box::leak(Box::new([Field {
            kind: ItemKind::Tuple as u32,
            offset: 0,
            tuple: inner,
        }]))
        .as_ptr(),
    }));
    let words = if words.is_empty() { &[0][..] } else { words };
    // SAFETY: the exception and the tuple are live.
    unsafe {
        typy_rt_exception_part_equal(
            object,
            part,
            outer,
            words.as_ptr().expose_provenance() as u64,
        )
    }
}

#[test]
fn args_equals_only_a_tuple_of_the_same_arguments() {
    let three = inline(3).unwrap().cast_unsigned();
    let none = built(&VALUE_ERROR, &[], &[]);
    let raised = raise(ExceptionKind::ValueError, "bad");
    let texts = [
        crate::string::from_str("bad"),
        crate::string::from_str("other"),
    ];
    let text = ItemKind::Object;
    let [bad, other] = texts.map(|each| each.expose_provenance() as u64);
    assert!(equals_tuple(none, args(), &[], &[]));
    assert!(!equals_tuple(none, args(), &[ItemKind::Int], &[three]));
    assert!(!equals(none, args(), ItemKind::Int, three));
    assert!(equals_tuple(raised, args(), &[text], &[bad]));
    assert!(!equals_tuple(raised, args(), &[text], &[other]));
    assert!(!equals_tuple(raised, args(), &[text, text], &[bad, bad]));
    assert!(!equals(raised, args(), text, bad));
    for exception in [none, raised] {
        release(exception);
    }
    for each in texts {
        // SAFETY: the test holds the only reference.
        unsafe { typy_rt_decrement(each.cast()) };
    }
}

/// Returns a `ValueError` built with no arguments.
fn built_none() -> *mut ExceptionObject {
    built(&VALUE_ERROR, &[], &[])
}

/// Returns whether `args` of `object` is laid out as a tuple of these kinds, as `args`
/// shows it, or as the exception holds it when `raw`.
fn laid_out(object: *mut ExceptionObject, kinds: &[ItemKind], raw: bool) -> bool {
    let raw = inline(i64::from(raw)).unwrap();
    // SAFETY: the exception and the layout are live.
    unsafe { typy_rt_exception_arguments_match(object, layout(kinds), raw) }
}

#[test]
fn args_matches_the_layout_of_its_arguments_its_text_or_no_items() {
    let three = inline(3).unwrap().cast_unsigned();
    let built = built(&VALUE_ERROR, &[ItemKind::Int, ItemKind::Bool], &[three, 1]);
    let raised = raise(ExceptionKind::ValueError, "bad");
    let none = built_none();
    let text = ItemKind::Object;
    for raw in [false, true] {
        assert!(laid_out(built, &[ItemKind::Int, ItemKind::Bool], raw));
        assert!(!laid_out(built, &[ItemKind::Int], raw));
        assert!(laid_out(raised, &[text], raw));
        assert!(!laid_out(raised, &[], raw));
        assert!(laid_out(none, &[], raw));
        assert!(!laid_out(none, &[text], raw));
    }
    for exception in [built, raised, none] {
        release(exception);
    }
}

#[test]
fn an_os_error_with_a_file_name_matches_two_items_unless_raw() {
    let two = inline(2).unwrap().cast_unsigned();
    let names = crate::string::from_str("missing.txt");
    let reason = crate::string::from_str("No such file or directory");
    let text = ItemKind::Object;
    let words = [
        two,
        reason.expose_provenance() as u64,
        names.expose_provenance() as u64,
    ];
    let error = built(&FILE_NOT_FOUND_ERROR, &[ItemKind::Int, text, text], &words);
    assert!(laid_out(error, &[ItemKind::Int, text], false));
    assert!(!laid_out(error, &[ItemKind::Int, text, text], false));
    assert!(laid_out(error, &[ItemKind::Int, text, text], true));
    assert!(!laid_out(error, &[ItemKind::Int, text], true));
    release(error);
    for each in [names, reason] {
        // SAFETY: the test holds its own reference.
        unsafe { typy_rt_decrement(each.cast()) };
    }
}

#[test]
fn the_items_of_args_are_the_arguments_the_field_of_the_text_or_none() {
    let three = inline(3).unwrap().cast_unsigned();
    let built = built(&VALUE_ERROR, &[ItemKind::Int], &[three]);
    let raised = raise(ExceptionKind::ValueError, "bad");
    let none = built_none();
    // SAFETY: the exceptions are live.
    unsafe {
        assert_eq!(
            typy_rt_exception_arguments_address(built),
            (*built).arguments.expose_provenance() as u64
        );
        assert_eq!(
            typy_rt_exception_arguments_address(raised),
            (&raw const (*raised).message).expose_provenance() as u64
        );
        assert_eq!(typy_rt_exception_arguments_address(none), 0);
    }
    for exception in [built, raised, none] {
        release(exception);
    }
}
