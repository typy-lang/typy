//! Tests of `sys`: decoding the arguments, the list `sys.argv`, and the exit status an
//! uncaught exception gives.

use super::{INPUT_BLOCK, Input, decode, exit_status, keep_arguments, read_line, typy_rt_sys_argv};
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

/// Returns each line `read_line` gives from `chunks`, read one chunk at a time, until
/// `EOFError`, with the text of the `EOFError`.
fn lines_of(chunks: &[&[u8]]) -> (Vec<Vec<u32>>, Vec<u32>) {
    let mut input = Input {
        bytes: [0; INPUT_BLOCK],
        start: 0,
        end: 0,
    };
    let mut next = chunks.iter();
    let mut lines = Vec::new();
    loop {
        let line = read_line(&mut input, |buffer| {
            let Some(chunk) = next.next() else {
                return 0;
            };
            buffer[..chunk.len()].copy_from_slice(chunk);
            chunk.len()
        });
        if !line.exception.is_null() {
            // SAFETY: the exception is live and owned here.
            unsafe {
                let str = (*line.exception).class().str.unwrap();
                let text = str(line.exception.cast());
                let found = points(text.value);
                typy_rt_decrement(text.value.cast());
                typy_rt_decrement(line.exception.cast());
                return (lines, found);
            }
        }
        lines.push(points(line.value));
        // SAFETY: the line is owned here.
        unsafe { typy_rt_decrement(line.value.cast()) };
    }
}

fn text_points(text: &str) -> Vec<u32> {
    text.chars().map(u32::from).collect()
}

#[test]
fn input_gives_each_line_without_its_newline_then_eof_error() {
    let end = text_points("EOF when reading a line");
    let cases: [(&[&[u8]], &[&str]); 6] = [
        (&[], &[]),
        (&[b"one\n"], &["one"]),
        (&[b"one\n\ntwo\r\n"], &["one", "", "two\r"]),
        (&[b"last"], &["last"]),
        (
            &[b"sp", b"lit\nacross", b" chunks\n"],
            &["split", "across chunks"],
        ),
        (&[b"caf\xc3", b"\xa9\n"], &["caf\u{e9}"]),
    ];
    for (chunks, expected) in cases {
        let (lines, error) = lines_of(chunks);
        let expected: Vec<Vec<u32>> = expected.iter().map(|line| text_points(line)).collect();
        assert_eq!(lines, expected, "{chunks:?}");
        assert_eq!(error, end, "{chunks:?}");
    }
}

#[test]
fn input_grows_its_line_past_the_first_allocation() {
    let long = [b'x'; INPUT_BLOCK];
    let (lines, _) = lines_of(&[&long, &long[..100], b"\n"]);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].len(), INPUT_BLOCK + 100);
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
