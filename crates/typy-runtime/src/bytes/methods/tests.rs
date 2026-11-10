//! Tests of the methods of `bytes`, against results CPython 3.15 gives.

use super::{
    typy_rt_bytes_case, typy_rt_bytes_hex, typy_rt_bytes_pad, typy_rt_bytes_search,
    typy_rt_bytes_split, typy_rt_bytes_splitlines, typy_rt_bytes_strip, typy_rt_bytes_test,
};
use crate::bytes::new_bytes;
use crate::header::BYTES_TYPE_ID;
use crate::string::{StrObject, code_point, length, set_code_point};
use crate::word::inline;

fn word(value: i64) -> i64 {
    inline(value).unwrap()
}

fn bytes(values: &[u8]) -> *mut StrObject {
    let data = new_bytes(values.len());
    for (index, value) in values.iter().enumerate() {
        // SAFETY: the new object has room for every byte.
        unsafe { set_code_point(data, index, u32::from(*value)) };
    }
    data
}

fn values(data: *const StrObject) -> Vec<u8> {
    // SAFETY: the data is live and every index is in bounds.
    unsafe {
        assert_eq!((*data).header.type_id, BYTES_TYPE_ID);
        (0..length(data))
            .map(|index| u8::try_from(code_point(data, index)).unwrap())
            .collect()
    }
}

fn text(string: *const StrObject) -> String {
    // SAFETY: the string is live and every index is in bounds.
    (0..unsafe { length(string) })
        .map(|index| char::from_u32(unsafe { code_point(string, index) }).unwrap())
        .collect()
}

/// Joins the `bytes` items of a list, each followed by `|`.
fn joined(list: *const crate::ListObject) -> Vec<u8> {
    // SAFETY: the list is live and holds live `bytes`.
    (0..unsafe { (*list).length })
        .flat_map(|index| {
            // SAFETY: the index is below the length.
            let item = unsafe { (*list).item(index) };
            let mut part = values(core::ptr::with_exposed_provenance(
                usize::try_from(item).unwrap(),
            ));
            part.push(b'|');
            part
        })
        .collect()
}

/// The bytes, `split()` and `splitlines()` joined with `|` after each part, and `strip()`.
type Spaces = (&'static [u8], &'static [u8], &'static [u8], &'static [u8]);

const SPACES: &[Spaces] = &[
    (&[], &[], &[], &[]),
    (
        &[97, 32, 98, 9, 99],
        &[97, 124, 98, 124, 99, 124],
        &[97, 32, 98, 9, 99, 124],
        &[97, 32, 98, 9, 99],
    ),
    (
        &[32, 133, 97, 160, 98, 28, 32],
        &[133, 97, 160, 98, 28, 124],
        &[32, 133, 97, 160, 98, 28, 32, 124],
        &[133, 97, 160, 98, 28],
    ),
    (
        &[72, 101, 108, 108, 111, 32, 87, 111, 114, 108, 100],
        &[72, 101, 108, 108, 111, 124, 87, 111, 114, 108, 100, 124],
        &[72, 101, 108, 108, 111, 32, 87, 111, 114, 108, 100, 124],
        &[72, 101, 108, 108, 111, 32, 87, 111, 114, 108, 100],
    ),
    (
        &[
            116, 104, 101, 121, 39, 114, 101, 32, 98, 105, 108, 108, 39, 115,
        ],
        &[
            116, 104, 101, 121, 39, 114, 101, 124, 98, 105, 108, 108, 39, 115, 124,
        ],
        &[
            116, 104, 101, 121, 39, 114, 101, 32, 98, 105, 108, 108, 39, 115, 124,
        ],
        &[
            116, 104, 101, 121, 39, 114, 101, 32, 98, 105, 108, 108, 39, 115,
        ],
    ),
    (
        &[97, 10, 98, 13, 10, 99, 13, 100, 11],
        &[97, 124, 98, 124, 99, 124, 100, 124],
        &[97, 124, 98, 124, 99, 124, 100, 11, 124],
        &[97, 10, 98, 13, 10, 99, 13, 100],
    ),
    (
        &[65, 66, 67, 32, 100, 101, 102, 32, 49, 50, 51],
        &[65, 66, 67, 124, 100, 101, 102, 124, 49, 50, 51, 124],
        &[65, 66, 67, 32, 100, 101, 102, 32, 49, 50, 51, 124],
        &[65, 66, 67, 32, 100, 101, 102, 32, 49, 50, 51],
    ),
    (&[233, 65], &[233, 65, 124], &[233, 65, 124], &[233, 65]),
    (&[49, 50], &[49, 50, 124], &[49, 50, 124], &[49, 50]),
    (&[32], &[], &[32, 124], &[]),
];

/// The bytes, `upper`, `lower`, `swapcase`, `capitalize` and `title`, the `is` tests in the
/// order of `typy_rt_bytes_test`, and `hex`.
type Cases = (&'static [u8], [&'static [u8]; 5], [bool; 7], &'static str);

const CASES: &[Cases] = &[
    (
        &[],
        [&[], &[], &[], &[], &[]],
        [false, false, false, false, false, false, false],
        "",
    ),
    (
        &[97, 32, 98, 9, 99],
        [
            &[65, 32, 66, 9, 67],
            &[97, 32, 98, 9, 99],
            &[65, 32, 66, 9, 67],
            &[65, 32, 98, 9, 99],
            &[65, 32, 66, 9, 67],
        ],
        [false, false, false, false, false, true, false],
        "6120620963",
    ),
    (
        &[32, 133, 97, 160, 98, 28, 32],
        [
            &[32, 133, 65, 160, 66, 28, 32],
            &[32, 133, 97, 160, 98, 28, 32],
            &[32, 133, 65, 160, 66, 28, 32],
            &[32, 133, 97, 160, 98, 28, 32],
            &[32, 133, 65, 160, 66, 28, 32],
        ],
        [false, false, false, false, false, true, false],
        "208561a0621c20",
    ),
    (
        &[72, 101, 108, 108, 111, 32, 87, 111, 114, 108, 100],
        [
            &[72, 69, 76, 76, 79, 32, 87, 79, 82, 76, 68],
            &[104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100],
            &[104, 69, 76, 76, 79, 32, 119, 79, 82, 76, 68],
            &[72, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100],
            &[72, 101, 108, 108, 111, 32, 87, 111, 114, 108, 100],
        ],
        [false, false, false, false, false, false, true],
        "48656c6c6f20576f726c64",
    ),
    (
        &[
            116, 104, 101, 121, 39, 114, 101, 32, 98, 105, 108, 108, 39, 115,
        ],
        [
            &[84, 72, 69, 89, 39, 82, 69, 32, 66, 73, 76, 76, 39, 83],
            &[
                116, 104, 101, 121, 39, 114, 101, 32, 98, 105, 108, 108, 39, 115,
            ],
            &[84, 72, 69, 89, 39, 82, 69, 32, 66, 73, 76, 76, 39, 83],
            &[
                84, 104, 101, 121, 39, 114, 101, 32, 98, 105, 108, 108, 39, 115,
            ],
            &[
                84, 104, 101, 121, 39, 82, 101, 32, 66, 105, 108, 108, 39, 83,
            ],
        ],
        [false, false, false, false, false, true, false],
        "746865792772652062696c6c2773",
    ),
    (
        &[97, 10, 98, 13, 10, 99, 13, 100, 11],
        [
            &[65, 10, 66, 13, 10, 67, 13, 68, 11],
            &[97, 10, 98, 13, 10, 99, 13, 100, 11],
            &[65, 10, 66, 13, 10, 67, 13, 68, 11],
            &[65, 10, 98, 13, 10, 99, 13, 100, 11],
            &[65, 10, 66, 13, 10, 67, 13, 68, 11],
        ],
        [false, false, false, false, false, true, false],
        "610a620d0a630d640b",
    ),
    (
        &[65, 66, 67, 32, 100, 101, 102, 32, 49, 50, 51],
        [
            &[65, 66, 67, 32, 68, 69, 70, 32, 49, 50, 51],
            &[97, 98, 99, 32, 100, 101, 102, 32, 49, 50, 51],
            &[97, 98, 99, 32, 68, 69, 70, 32, 49, 50, 51],
            &[65, 98, 99, 32, 100, 101, 102, 32, 49, 50, 51],
            &[65, 98, 99, 32, 68, 101, 102, 32, 49, 50, 51],
        ],
        [false, false, false, false, false, false, false],
        "4142432064656620313233",
    ),
    (
        &[233, 65],
        [&[233, 65], &[233, 97], &[233, 97], &[233, 97], &[233, 65]],
        [false, false, false, false, true, false, true],
        "e941",
    ),
    (
        &[49, 50],
        [&[49, 50], &[49, 50], &[49, 50], &[49, 50], &[49, 50]],
        [false, true, true, false, false, false, false],
        "3132",
    ),
    (
        &[32],
        [&[32], &[32], &[32], &[32], &[32]],
        [false, false, false, true, false, false, false],
        "20",
    ),
];

#[test]
fn whitespace_and_line_breaks_are_ascii() {
    for (data, split, lines, stripped) in SPACES {
        // SAFETY: the bytes are live.
        unsafe {
            let parts = typy_rt_bytes_split(bytes(data), bytes(&[]), word(-1), word(0)).value;
            assert_eq!(joined(parts), *split, "{data:?}");
            assert_eq!(
                joined(typy_rt_bytes_splitlines(bytes(data), word(0))),
                *lines,
                "{data:?}"
            );
            assert_eq!(
                values(typy_rt_bytes_strip(bytes(data), bytes(&[]), word(6))),
                *stripped
            );
        }
    }
}

#[test]
fn case_tests_and_hex_are_ascii() {
    let questions = [0, 1, 3, 5, 6, 7, 8];
    for (data, cased, tests, hex) in CASES {
        for (mode, expected) in [0, 1, 3, 4, 5].iter().zip(cased) {
            // SAFETY: the bytes are live.
            let made = unsafe { typy_rt_bytes_case(bytes(data), word(*mode)) };
            assert_eq!(values(made), *expected, "{data:?} mode {mode}");
        }
        for (question, expected) in questions.iter().zip(tests) {
            // SAFETY: the bytes are live.
            let answer = unsafe { typy_rt_bytes_test(bytes(data), word(*question)) };
            assert_eq!(answer, *expected, "{data:?} test {question}");
        }
        // SAFETY: the bytes are live.
        assert_eq!(text(unsafe { typy_rt_bytes_hex(bytes(data)) }), *hex);
    }
}

#[test]
fn errors_name_bytes_as_cpython_does() {
    let message = |exception: *mut crate::ExceptionObject| {
        // SAFETY: the exception and its message are live.
        text(unsafe { (*exception).message })
    };
    // SAFETY: the bytes are live.
    unsafe {
        let missing = typy_rt_bytes_search(bytes(b"abc"), bytes(b"z"), word(0), word(0), word(8));
        assert_eq!(message(missing.exception), "subsection not found");
        let wide = typy_rt_bytes_pad(bytes(b"a"), word(5), bytes(b"ab"), word(0));
        assert_eq!(
            message(wide.exception),
            "center(): argument 2 must be a byte string of length 1, not a bytes object of length 2"
        );
        let padded = typy_rt_bytes_pad(bytes(b"a"), word(3), bytes(b"*"), word(2)).value;
        assert_eq!(values(padded), b"**a");
    }
}
