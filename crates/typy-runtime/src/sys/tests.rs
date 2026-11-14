//! Tests of `sys`: decoding the arguments, the list `sys.argv`, and the exit status an
//! uncaught exception gives.

use super::{
    decode, exit_status, keep_arguments, typy_rt_sys_argv, typy_rt_sys_getrecursionlimit,
    typy_rt_sys_setrecursionlimit,
};
use crate::exception::classes::{EXCEPTION, SYSTEM_EXIT};
use crate::exception::{ExceptionKind, ExceptionObject, raise, typy_rt_exception_new};
use crate::int::limbs::Limbs;
use crate::int::value::Integer;
use crate::list::ItemKind;
use crate::refcount::typy_rt_decrement;
use crate::string::{StrObject, code_point, from_str, length};
use crate::tuple::{Field, TupleType};
use crate::type_object::TypeObject;

/// Returns the code points of a `str`.
fn points(text: *const StrObject) -> Vec<u32> {
    // SAFETY: the string is live.
    unsafe {
        (0..length(text))
            .map(|index| code_point(text, index))
            .collect()
    }
}

#[test]
fn arguments_decode_as_utf_8_with_each_stray_byte_as_a_surrogate() {
    let cases: [(&[u8], &[u32]); 5] = [
        (b"", &[]),
        (b"two words", &[116, 119, 111, 32, 119, 111, 114, 100, 115]),
        ("\u{e9}\u{1f600}".as_bytes(), &[0xe9, 0x1f600]),
        (b"a\xffb", &[0x61, 0xdcff, 0x62]),
        (b"\xc3", &[0xdcc3]),
    ];
    for (bytes, expected) in cases {
        let text = decode(bytes);
        assert_eq!(points(text), expected, "{bytes:?}");
        // SAFETY: the string is owned here.
        unsafe { typy_rt_decrement(text.cast()) };
    }
}

#[test]
fn sys_argv_is_built_once_from_the_arguments_main_receives() {
    static FIRST: &[u8] = b"./program\0";
    static SECOND: &[u8] = b"caf\xc3\xa9\0";
    let values = [FIRST.as_ptr(), SECOND.as_ptr()];
    keep_arguments(2, values.as_ptr());
    let list = typy_rt_sys_argv();
    assert!(core::ptr::eq(list, typy_rt_sys_argv()));
    // SAFETY: the list is live and holds two strings.
    unsafe {
        assert_eq!((*list).length, 2);
        let second = (*list).item(1) as *const StrObject;
        assert_eq!(points(second), [0x63, 0x61, 0x66, 0xe9]);
        assert_eq!((*second).header.reference_count, 1);
    }
}

/// Returns a `SystemExit`, or an instance of `class`, with one argument of `kind`.
fn with_argument(class: &TypeObject, kind: ItemKind, word: u64) -> *mut ExceptionObject {
    let fields = Box::leak(Box::new([Field::plain(kind, 0)]));
    let layout = Box::leak(Box::new(TupleType {
        size: 8,
        length: 1,
        fields: fields.as_ptr(),
    }));
    let words = [word];
    // SAFETY: the word is a live item of the layout; the exception retains its copy, and the
    // test's reference is released after.
    unsafe {
        let exception = typy_rt_exception_new(class, layout, words.as_ptr().cast());
        crate::tuple::release(layout, words.as_ptr().expose_provenance() as u64);
        exception
    }
}

/// Returns the `int` word of `limbs`, a large integer the caller owns.
fn large(limbs: &[u64]) -> u64 {
    Integer {
        negative: false,
        magnitude: Limbs::from_slice(limbs),
    }
    .into_word()
    .cast_unsigned()
}

fn inline(value: i64) -> u64 {
    crate::word::inline(value).unwrap().cast_unsigned()
}

#[test]
fn system_exit_gives_its_code_as_the_exit_status_without_a_report() {
    let cases = [
        (ItemKind::NoneValue, 0, 0),
        (ItemKind::Int, inline(3), 3),
        (ItemKind::Int, inline(258), 2),
        (ItemKind::Int, inline(-1), 255),
        (ItemKind::Bool, 1, 1),
        (ItemKind::Bool, 0, 0),
        (ItemKind::Int, large(&[5, 1]), 5),
        (ItemKind::Int, large(&[0, 0, 1]), 255),
    ];
    for (kind, word, status) in cases {
        let exception = with_argument(&SYSTEM_EXIT, kind, word);
        // SAFETY: the exception is live and owned here.
        unsafe {
            assert_eq!(exit_status(exception), (status, false), "{kind:?} {word}");
            typy_rt_decrement(exception.cast());
        }
    }
}

static NO_FIELDS: [Field; 0] = [];
static NOTHING: TupleType = TupleType {
    size: 8,
    length: 0,
    fields: NO_FIELDS.as_ptr(),
};

#[test]
fn system_exit_without_arguments_ends_with_zero() {
    // SAFETY: no arguments are read; the exception is owned here.
    unsafe {
        let exception = typy_rt_exception_new(
            &raw const SYSTEM_EXIT,
            &raw const NOTHING,
            core::ptr::null(),
        );
        assert_eq!(exit_status(exception), (0, false));
        typy_rt_decrement(exception.cast());
    }
}

#[test]
fn system_exit_with_another_value_ends_with_one_after_writing_it() {
    let exception = with_argument(
        &SYSTEM_EXIT,
        ItemKind::Object,
        from_str("bye").expose_provenance() as u64,
    );
    // SAFETY: the exception is live and owned here.
    unsafe {
        assert_eq!(exit_status(exception), (1, false));
        typy_rt_decrement(exception.cast());
    }
}

#[test]
fn any_other_exception_is_reported_and_ends_with_one() {
    let error = raise(ExceptionKind::ValueError, "bad");
    let built = with_argument(&EXCEPTION, ItemKind::Int, inline(0));
    // SAFETY: the exceptions are live and owned here.
    unsafe {
        assert_eq!(exit_status(error), (1, true));
        assert_eq!(exit_status(built), (1, true));
        typy_rt_decrement(error.cast());
        typy_rt_decrement(built.cast());
    }
}

static TWO_INT_FIELDS: [Field; 2] = [
    Field::plain(ItemKind::Int, 0),
    Field::plain(ItemKind::Int, 8),
];
static TWO_INTS: TupleType = TupleType {
    size: 16,
    length: 2,
    fields: TWO_INT_FIELDS.as_ptr(),
};

#[test]
fn system_exit_with_several_arguments_ends_with_one_after_writing_them() {
    let words = [inline(1), inline(2)];
    // SAFETY: the words are inline `int` items of the layout; the exception is owned here.
    unsafe {
        let exception = typy_rt_exception_new(
            &raw const SYSTEM_EXIT,
            &raw const TWO_INTS,
            words.as_ptr().cast(),
        );
        assert_eq!(exit_status(exception), (1, false));
        typy_rt_decrement(exception.cast());
    }
}

/// A class of a program's own named `SystemExit`, which is not the built-in one.
static OWN_SYSTEM_EXIT: TypeObject = TypeObject {
    module: "app".as_ptr(),
    module_length: 3,
    ..TypeObject::new("SystemExit", &raw const EXCEPTION)
};

#[test]
fn a_class_of_a_program_named_system_exit_is_reported() {
    let exception = with_argument(&OWN_SYSTEM_EXIT, ItemKind::Int, inline(0));
    // SAFETY: the exception is live and owned here.
    unsafe {
        assert_eq!(exit_status(exception), (1, true));
        typy_rt_decrement(exception.cast());
    }
}

#[test]
fn the_recursion_limit_keeps_a_c_int_of_at_least_one() {
    let overflow = Some("Python int too large to convert to C int");
    let too_small = Some("recursion limit must be greater or equal than 1");
    let cases = [
        (inline(50), None, 50),
        (inline(0), too_small, 50),
        (inline(-5), too_small, 50),
        (inline(2_147_483_648), overflow, 50),
        (inline(2_147_483_647), None, 2_147_483_647),
        (large(&[0, 1]), overflow, 2_147_483_647),
    ];
    for (word, message, limit) in cases {
        // SAFETY: the word is an inline `int` or a live large one.
        let error = unsafe { typy_rt_sys_setrecursionlimit(word.cast_signed()) };
        let found = (!error.is_null()).then(|| {
            // SAFETY: the exception and its message are live; the exception is owned here.
            unsafe {
                let text = points((*error).message);
                typy_rt_decrement(error.cast());
                String::from_iter(text.into_iter().filter_map(char::from_u32))
            }
        });
        assert_eq!(found.as_deref(), message, "{word}");
        assert_eq!(
            typy_rt_sys_getrecursionlimit(),
            crate::word::inline(limit).unwrap()
        );
    }
}
