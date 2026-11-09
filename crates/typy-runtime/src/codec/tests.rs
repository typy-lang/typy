//! Tests of `str.encode` and `bytes.decode`, against results CPython 3.15 gives.

use super::{typy_rt_bytes_decode, typy_rt_str_encode};
use crate::bytes::new_bytes;
use crate::string::{StrKind, StrObject, code_point, length, new_string, set_code_point};
use crate::word::inline;

fn text(string: *const StrObject) -> String {
    // SAFETY: the string is live and every index is in bounds.
    (0..unsafe { length(string) })
        .map(|index| char::from_u32(unsafe { code_point(string, index) }).unwrap_or('\u{fffd}'))
        .collect()
}

fn points(string: *const StrObject) -> Vec<u32> {
    // SAFETY: the string is live and every index is in bounds.
    (0..unsafe { length(string) })
        .map(|index| unsafe { code_point(string, index) })
        .collect()
}

/// Returns a string of code points, lone surrogates included.
fn string_of(points: &[u32]) -> *mut StrObject {
    let kind = points
        .iter()
        .map(|point| StrKind::of(*point))
        .max()
        .unwrap_or(StrKind::Latin1);
    let text = new_string(points.len(), kind);
    for (index, point) in points.iter().enumerate() {
        // SAFETY: the new string has room for every code point of this kind.
        unsafe { set_code_point(text, index, *point) };
    }
    text
}

fn bytes(values: &[u8]) -> *mut StrObject {
    let data = new_bytes(values.len());
    for (index, value) in values.iter().enumerate() {
        // SAFETY: the new object has room for every byte.
        unsafe { set_code_point(data, index, u32::from(*value)) };
    }
    data
}

fn message(exception: *mut crate::ExceptionObject) -> String {
    // SAFETY: the exception is live; its message is a live string.
    text(unsafe { (*exception).message })
}

/// The text, the codec, the handler, and the bytes or the message of the error.
type Encoding = (
    &'static [u32],
    i64,
    i64,
    Result<&'static [u8], &'static str>,
);

const ENCODINGS: &[Encoding] = &[
    (&[], 0, 0, Ok(&[])),
    (&[], 0, 1, Ok(&[])),
    (&[], 0, 2, Ok(&[])),
    (&[], 0, 3, Ok(&[])),
    (&[], 0, 4, Ok(&[])),
    (&[], 0, 5, Ok(&[])),
    (&[], 1, 0, Ok(&[])),
    (&[], 1, 1, Ok(&[])),
    (&[], 1, 2, Ok(&[])),
    (&[], 1, 3, Ok(&[])),
    (&[], 1, 4, Ok(&[])),
    (&[], 1, 5, Ok(&[])),
    (&[], 2, 0, Ok(&[])),
    (&[], 2, 1, Ok(&[])),
    (&[], 2, 2, Ok(&[])),
    (&[], 2, 3, Ok(&[])),
    (&[], 2, 4, Ok(&[])),
    (&[], 2, 5, Ok(&[])),
    (&[0x61, 0x62, 0x63], 0, 0, Ok(&[97, 98, 99])),
    (&[0x61, 0x62, 0x63], 0, 1, Ok(&[97, 98, 99])),
    (&[0x61, 0x62, 0x63], 0, 2, Ok(&[97, 98, 99])),
    (&[0x61, 0x62, 0x63], 0, 3, Ok(&[97, 98, 99])),
    (&[0x61, 0x62, 0x63], 0, 4, Ok(&[97, 98, 99])),
    (&[0x61, 0x62, 0x63], 0, 5, Ok(&[97, 98, 99])),
    (&[0x61, 0x62, 0x63], 1, 0, Ok(&[97, 98, 99])),
    (&[0x61, 0x62, 0x63], 1, 1, Ok(&[97, 98, 99])),
    (&[0x61, 0x62, 0x63], 1, 2, Ok(&[97, 98, 99])),
    (&[0x61, 0x62, 0x63], 1, 3, Ok(&[97, 98, 99])),
    (&[0x61, 0x62, 0x63], 1, 4, Ok(&[97, 98, 99])),
    (&[0x61, 0x62, 0x63], 1, 5, Ok(&[97, 98, 99])),
    (&[0x61, 0x62, 0x63], 2, 0, Ok(&[97, 98, 99])),
    (&[0x61, 0x62, 0x63], 2, 1, Ok(&[97, 98, 99])),
    (&[0x61, 0x62, 0x63], 2, 2, Ok(&[97, 98, 99])),
    (&[0x61, 0x62, 0x63], 2, 3, Ok(&[97, 98, 99])),
    (&[0x61, 0x62, 0x63], 2, 4, Ok(&[97, 98, 99])),
    (&[0x61, 0x62, 0x63], 2, 5, Ok(&[97, 98, 99])),
    (&[0x68, 0xe9], 0, 0, Ok(&[104, 195, 169])),
    (&[0x68, 0xe9], 0, 1, Ok(&[104, 195, 169])),
    (&[0x68, 0xe9], 0, 2, Ok(&[104, 195, 169])),
    (&[0x68, 0xe9], 0, 3, Ok(&[104, 195, 169])),
    (&[0x68, 0xe9], 0, 4, Ok(&[104, 195, 169])),
    (&[0x68, 0xe9], 0, 5, Ok(&[104, 195, 169])),
    (
        &[0x68, 0xe9],
        1,
        0,
        Err(
            "'ascii' codec can't encode character '\u{5c}xe9' in position 1: ordinal not in range(128)",
        ),
    ),
    (&[0x68, 0xe9], 1, 1, Ok(&[104])),
    (&[0x68, 0xe9], 1, 2, Ok(&[104, 63])),
    (&[0x68, 0xe9], 1, 3, Ok(&[104, 92, 120, 101, 57])),
    (&[0x68, 0xe9], 1, 4, Ok(&[104, 38, 35, 50, 51, 51, 59])),
    (
        &[0x68, 0xe9],
        1,
        5,
        Err(
            "'ascii' codec can't encode character '\u{5c}xe9' in position 1: ordinal not in range(128)",
        ),
    ),
    (&[0x68, 0xe9], 2, 0, Ok(&[104, 233])),
    (&[0x68, 0xe9], 2, 1, Ok(&[104, 233])),
    (&[0x68, 0xe9], 2, 2, Ok(&[104, 233])),
    (&[0x68, 0xe9], 2, 3, Ok(&[104, 233])),
    (&[0x68, 0xe9], 2, 4, Ok(&[104, 233])),
    (&[0x68, 0xe9], 2, 5, Ok(&[104, 233])),
    (&[0xe9, 0xe8, 0x78], 0, 0, Ok(&[195, 169, 195, 168, 120])),
    (&[0xe9, 0xe8, 0x78], 0, 1, Ok(&[195, 169, 195, 168, 120])),
    (&[0xe9, 0xe8, 0x78], 0, 2, Ok(&[195, 169, 195, 168, 120])),
    (&[0xe9, 0xe8, 0x78], 0, 3, Ok(&[195, 169, 195, 168, 120])),
    (&[0xe9, 0xe8, 0x78], 0, 4, Ok(&[195, 169, 195, 168, 120])),
    (&[0xe9, 0xe8, 0x78], 0, 5, Ok(&[195, 169, 195, 168, 120])),
    (
        &[0xe9, 0xe8, 0x78],
        1,
        0,
        Err("'ascii' codec can't encode characters in position 0-1: ordinal not in range(128)"),
    ),
    (&[0xe9, 0xe8, 0x78], 1, 1, Ok(&[120])),
    (&[0xe9, 0xe8, 0x78], 1, 2, Ok(&[63, 63, 120])),
    (
        &[0xe9, 0xe8, 0x78],
        1,
        3,
        Ok(&[92, 120, 101, 57, 92, 120, 101, 56, 120]),
    ),
    (
        &[0xe9, 0xe8, 0x78],
        1,
        4,
        Ok(&[38, 35, 50, 51, 51, 59, 38, 35, 50, 51, 50, 59, 120]),
    ),
    (
        &[0xe9, 0xe8, 0x78],
        1,
        5,
        Err("'ascii' codec can't encode characters in position 0-1: ordinal not in range(128)"),
    ),
    (&[0xe9, 0xe8, 0x78], 2, 0, Ok(&[233, 232, 120])),
    (&[0xe9, 0xe8, 0x78], 2, 1, Ok(&[233, 232, 120])),
    (&[0xe9, 0xe8, 0x78], 2, 2, Ok(&[233, 232, 120])),
    (&[0xe9, 0xe8, 0x78], 2, 3, Ok(&[233, 232, 120])),
    (&[0xe9, 0xe8, 0x78], 2, 4, Ok(&[233, 232, 120])),
    (&[0xe9, 0xe8, 0x78], 2, 5, Ok(&[233, 232, 120])),
    (&[0x100], 0, 0, Ok(&[196, 128])),
    (&[0x100], 0, 1, Ok(&[196, 128])),
    (&[0x100], 0, 2, Ok(&[196, 128])),
    (&[0x100], 0, 3, Ok(&[196, 128])),
    (&[0x100], 0, 4, Ok(&[196, 128])),
    (&[0x100], 0, 5, Ok(&[196, 128])),
    (
        &[0x100],
        1,
        0,
        Err(
            "'ascii' codec can't encode character '\u{5c}u0100' in position 0: ordinal not in range(128)",
        ),
    ),
    (&[0x100], 1, 1, Ok(&[])),
    (&[0x100], 1, 2, Ok(&[63])),
    (&[0x100], 1, 3, Ok(&[92, 117, 48, 49, 48, 48])),
    (&[0x100], 1, 4, Ok(&[38, 35, 50, 53, 54, 59])),
    (
        &[0x100],
        1,
        5,
        Err(
            "'ascii' codec can't encode character '\u{5c}u0100' in position 0: ordinal not in range(128)",
        ),
    ),
    (
        &[0x100],
        2,
        0,
        Err(
            "'latin-1' codec can't encode character '\u{5c}u0100' in position 0: ordinal not in range(256)",
        ),
    ),
    (&[0x100], 2, 1, Ok(&[])),
    (&[0x100], 2, 2, Ok(&[63])),
    (&[0x100], 2, 3, Ok(&[92, 117, 48, 49, 48, 48])),
    (&[0x100], 2, 4, Ok(&[38, 35, 50, 53, 54, 59])),
    (
        &[0x100],
        2,
        5,
        Err(
            "'latin-1' codec can't encode character '\u{5c}u0100' in position 0: ordinal not in range(256)",
        ),
    ),
    (
        &[0x20ac, 0x1f600],
        0,
        0,
        Ok(&[226, 130, 172, 240, 159, 152, 128]),
    ),
    (
        &[0x20ac, 0x1f600],
        0,
        1,
        Ok(&[226, 130, 172, 240, 159, 152, 128]),
    ),
    (
        &[0x20ac, 0x1f600],
        0,
        2,
        Ok(&[226, 130, 172, 240, 159, 152, 128]),
    ),
    (
        &[0x20ac, 0x1f600],
        0,
        3,
        Ok(&[226, 130, 172, 240, 159, 152, 128]),
    ),
    (
        &[0x20ac, 0x1f600],
        0,
        4,
        Ok(&[226, 130, 172, 240, 159, 152, 128]),
    ),
    (
        &[0x20ac, 0x1f600],
        0,
        5,
        Ok(&[226, 130, 172, 240, 159, 152, 128]),
    ),
    (
        &[0x20ac, 0x1f600],
        1,
        0,
        Err("'ascii' codec can't encode characters in position 0-1: ordinal not in range(128)"),
    ),
    (&[0x20ac, 0x1f600], 1, 1, Ok(&[])),
    (&[0x20ac, 0x1f600], 1, 2, Ok(&[63, 63])),
    (
        &[0x20ac, 0x1f600],
        1,
        3,
        Ok(&[
            92, 117, 50, 48, 97, 99, 92, 85, 48, 48, 48, 49, 102, 54, 48, 48,
        ]),
    ),
    (
        &[0x20ac, 0x1f600],
        1,
        4,
        Ok(&[
            38, 35, 56, 51, 54, 52, 59, 38, 35, 49, 50, 56, 53, 49, 50, 59,
        ]),
    ),
    (
        &[0x20ac, 0x1f600],
        1,
        5,
        Err("'ascii' codec can't encode characters in position 0-1: ordinal not in range(128)"),
    ),
    (
        &[0x20ac, 0x1f600],
        2,
        0,
        Err("'latin-1' codec can't encode characters in position 0-1: ordinal not in range(256)"),
    ),
    (&[0x20ac, 0x1f600], 2, 1, Ok(&[])),
    (&[0x20ac, 0x1f600], 2, 2, Ok(&[63, 63])),
    (
        &[0x20ac, 0x1f600],
        2,
        3,
        Ok(&[
            92, 117, 50, 48, 97, 99, 92, 85, 48, 48, 48, 49, 102, 54, 48, 48,
        ]),
    ),
    (
        &[0x20ac, 0x1f600],
        2,
        4,
        Ok(&[
            38, 35, 56, 51, 54, 52, 59, 38, 35, 49, 50, 56, 53, 49, 50, 59,
        ]),
    ),
    (
        &[0x20ac, 0x1f600],
        2,
        5,
        Err("'latin-1' codec can't encode characters in position 0-1: ordinal not in range(256)"),
    ),
    (
        &[0xd800],
        0,
        0,
        Err(
            "'utf-8' codec can't encode character '\u{5c}ud800' in position 0: surrogates not allowed",
        ),
    ),
    (&[0xd800], 0, 1, Ok(&[])),
    (&[0xd800], 0, 2, Ok(&[63])),
    (&[0xd800], 0, 3, Ok(&[92, 117, 100, 56, 48, 48])),
    (&[0xd800], 0, 4, Ok(&[38, 35, 53, 53, 50, 57, 54, 59])),
    (
        &[0xd800],
        0,
        5,
        Err(
            "'utf-8' codec can't encode character '\u{5c}ud800' in position 0: surrogates not allowed",
        ),
    ),
    (
        &[0xd800],
        1,
        0,
        Err(
            "'ascii' codec can't encode character '\u{5c}ud800' in position 0: ordinal not in range(128)",
        ),
    ),
    (&[0xd800], 1, 1, Ok(&[])),
    (&[0xd800], 1, 2, Ok(&[63])),
    (&[0xd800], 1, 3, Ok(&[92, 117, 100, 56, 48, 48])),
    (&[0xd800], 1, 4, Ok(&[38, 35, 53, 53, 50, 57, 54, 59])),
    (
        &[0xd800],
        1,
        5,
        Err(
            "'ascii' codec can't encode character '\u{5c}ud800' in position 0: ordinal not in range(128)",
        ),
    ),
    (
        &[0xd800],
        2,
        0,
        Err(
            "'latin-1' codec can't encode character '\u{5c}ud800' in position 0: ordinal not in range(256)",
        ),
    ),
    (&[0xd800], 2, 1, Ok(&[])),
    (&[0xd800], 2, 2, Ok(&[63])),
    (&[0xd800], 2, 3, Ok(&[92, 117, 100, 56, 48, 48])),
    (&[0xd800], 2, 4, Ok(&[38, 35, 53, 53, 50, 57, 54, 59])),
    (
        &[0xd800],
        2,
        5,
        Err(
            "'latin-1' codec can't encode character '\u{5c}ud800' in position 0: ordinal not in range(256)",
        ),
    ),
    (&[0x10000, 0x61], 0, 0, Ok(&[240, 144, 128, 128, 97])),
    (&[0x10000, 0x61], 0, 1, Ok(&[240, 144, 128, 128, 97])),
    (&[0x10000, 0x61], 0, 2, Ok(&[240, 144, 128, 128, 97])),
    (&[0x10000, 0x61], 0, 3, Ok(&[240, 144, 128, 128, 97])),
    (&[0x10000, 0x61], 0, 4, Ok(&[240, 144, 128, 128, 97])),
    (&[0x10000, 0x61], 0, 5, Ok(&[240, 144, 128, 128, 97])),
    (
        &[0x10000, 0x61],
        1,
        0,
        Err(
            "'ascii' codec can't encode character '\u{5c}U00010000' in position 0: ordinal not in range(128)",
        ),
    ),
    (&[0x10000, 0x61], 1, 1, Ok(&[97])),
    (&[0x10000, 0x61], 1, 2, Ok(&[63, 97])),
    (
        &[0x10000, 0x61],
        1,
        3,
        Ok(&[92, 85, 48, 48, 48, 49, 48, 48, 48, 48, 97]),
    ),
    (
        &[0x10000, 0x61],
        1,
        4,
        Ok(&[38, 35, 54, 53, 53, 51, 54, 59, 97]),
    ),
    (
        &[0x10000, 0x61],
        1,
        5,
        Err(
            "'ascii' codec can't encode character '\u{5c}U00010000' in position 0: ordinal not in range(128)",
        ),
    ),
    (
        &[0x10000, 0x61],
        2,
        0,
        Err(
            "'latin-1' codec can't encode character '\u{5c}U00010000' in position 0: ordinal not in range(256)",
        ),
    ),
    (&[0x10000, 0x61], 2, 1, Ok(&[97])),
    (&[0x10000, 0x61], 2, 2, Ok(&[63, 97])),
    (
        &[0x10000, 0x61],
        2,
        3,
        Ok(&[92, 85, 48, 48, 48, 49, 48, 48, 48, 48, 97]),
    ),
    (
        &[0x10000, 0x61],
        2,
        4,
        Ok(&[38, 35, 54, 53, 53, 51, 54, 59, 97]),
    ),
    (
        &[0x10000, 0x61],
        2,
        5,
        Err(
            "'latin-1' codec can't encode character '\u{5c}U00010000' in position 0: ordinal not in range(256)",
        ),
    ),
    (
        &[0xdcff, 0xdc80],
        0,
        0,
        Err("'utf-8' codec can't encode characters in position 0-1: surrogates not allowed"),
    ),
    (&[0xdcff, 0xdc80], 0, 1, Ok(&[])),
    (&[0xdcff, 0xdc80], 0, 2, Ok(&[63, 63])),
    (
        &[0xdcff, 0xdc80],
        0,
        3,
        Ok(&[92, 117, 100, 99, 102, 102, 92, 117, 100, 99, 56, 48]),
    ),
    (
        &[0xdcff, 0xdc80],
        0,
        4,
        Ok(&[
            38, 35, 53, 54, 53, 55, 53, 59, 38, 35, 53, 54, 52, 52, 56, 59,
        ]),
    ),
    (&[0xdcff, 0xdc80], 0, 5, Ok(&[255, 128])),
    (
        &[0xdcff, 0xdc80],
        1,
        0,
        Err("'ascii' codec can't encode characters in position 0-1: ordinal not in range(128)"),
    ),
    (&[0xdcff, 0xdc80], 1, 1, Ok(&[])),
    (&[0xdcff, 0xdc80], 1, 2, Ok(&[63, 63])),
    (
        &[0xdcff, 0xdc80],
        1,
        3,
        Ok(&[92, 117, 100, 99, 102, 102, 92, 117, 100, 99, 56, 48]),
    ),
    (
        &[0xdcff, 0xdc80],
        1,
        4,
        Ok(&[
            38, 35, 53, 54, 53, 55, 53, 59, 38, 35, 53, 54, 52, 52, 56, 59,
        ]),
    ),
    (&[0xdcff, 0xdc80], 1, 5, Ok(&[255, 128])),
    (
        &[0xdcff, 0xdc80],
        2,
        0,
        Err("'latin-1' codec can't encode characters in position 0-1: ordinal not in range(256)"),
    ),
    (&[0xdcff, 0xdc80], 2, 1, Ok(&[])),
    (&[0xdcff, 0xdc80], 2, 2, Ok(&[63, 63])),
    (
        &[0xdcff, 0xdc80],
        2,
        3,
        Ok(&[92, 117, 100, 99, 102, 102, 92, 117, 100, 99, 56, 48]),
    ),
    (
        &[0xdcff, 0xdc80],
        2,
        4,
        Ok(&[
            38, 35, 53, 54, 53, 55, 53, 59, 38, 35, 53, 54, 52, 52, 56, 59,
        ]),
    ),
    (&[0xdcff, 0xdc80], 2, 5, Ok(&[255, 128])),
    (
        &[0x61, 0xff, 0x100, 0x62],
        0,
        0,
        Ok(&[97, 195, 191, 196, 128, 98]),
    ),
    (
        &[0x61, 0xff, 0x100, 0x62],
        0,
        1,
        Ok(&[97, 195, 191, 196, 128, 98]),
    ),
    (
        &[0x61, 0xff, 0x100, 0x62],
        0,
        2,
        Ok(&[97, 195, 191, 196, 128, 98]),
    ),
    (
        &[0x61, 0xff, 0x100, 0x62],
        0,
        3,
        Ok(&[97, 195, 191, 196, 128, 98]),
    ),
    (
        &[0x61, 0xff, 0x100, 0x62],
        0,
        4,
        Ok(&[97, 195, 191, 196, 128, 98]),
    ),
    (
        &[0x61, 0xff, 0x100, 0x62],
        0,
        5,
        Ok(&[97, 195, 191, 196, 128, 98]),
    ),
    (
        &[0x61, 0xff, 0x100, 0x62],
        1,
        0,
        Err("'ascii' codec can't encode characters in position 1-2: ordinal not in range(128)"),
    ),
    (&[0x61, 0xff, 0x100, 0x62], 1, 1, Ok(&[97, 98])),
    (&[0x61, 0xff, 0x100, 0x62], 1, 2, Ok(&[97, 63, 63, 98])),
    (
        &[0x61, 0xff, 0x100, 0x62],
        1,
        3,
        Ok(&[97, 92, 120, 102, 102, 92, 117, 48, 49, 48, 48, 98]),
    ),
    (
        &[0x61, 0xff, 0x100, 0x62],
        1,
        4,
        Ok(&[97, 38, 35, 50, 53, 53, 59, 38, 35, 50, 53, 54, 59, 98]),
    ),
    (
        &[0x61, 0xff, 0x100, 0x62],
        1,
        5,
        Err("'ascii' codec can't encode characters in position 1-2: ordinal not in range(128)"),
    ),
    (
        &[0x61, 0xff, 0x100, 0x62],
        2,
        0,
        Err(
            "'latin-1' codec can't encode character '\u{5c}u0100' in position 2: ordinal not in range(256)",
        ),
    ),
    (&[0x61, 0xff, 0x100, 0x62], 2, 1, Ok(&[97, 255, 98])),
    (&[0x61, 0xff, 0x100, 0x62], 2, 2, Ok(&[97, 255, 63, 98])),
    (
        &[0x61, 0xff, 0x100, 0x62],
        2,
        3,
        Ok(&[97, 255, 92, 117, 48, 49, 48, 48, 98]),
    ),
    (
        &[0x61, 0xff, 0x100, 0x62],
        2,
        4,
        Ok(&[97, 255, 38, 35, 50, 53, 54, 59, 98]),
    ),
    (
        &[0x61, 0xff, 0x100, 0x62],
        2,
        5,
        Err(
            "'latin-1' codec can't encode character '\u{5c}u0100' in position 2: ordinal not in range(256)",
        ),
    ),
];

/// The bytes, the codec, the handler, and the text or the message of the error.
type Decoding = (
    &'static [u8],
    i64,
    i64,
    Result<&'static [u32], &'static str>,
);

const DECODINGS: &[Decoding] = &[
    (&[], 0, 0, Ok(&[])),
    (&[], 0, 1, Ok(&[])),
    (&[], 0, 2, Ok(&[])),
    (&[], 0, 3, Ok(&[])),
    (&[], 0, 5, Ok(&[])),
    (&[], 1, 0, Ok(&[])),
    (&[], 1, 1, Ok(&[])),
    (&[], 1, 2, Ok(&[])),
    (&[], 1, 3, Ok(&[])),
    (&[], 1, 5, Ok(&[])),
    (&[], 2, 0, Ok(&[])),
    (&[], 2, 1, Ok(&[])),
    (&[], 2, 2, Ok(&[])),
    (&[], 2, 3, Ok(&[])),
    (&[], 2, 5, Ok(&[])),
    (&[97, 98, 99], 0, 0, Ok(&[0x61, 0x62, 0x63])),
    (&[97, 98, 99], 0, 1, Ok(&[0x61, 0x62, 0x63])),
    (&[97, 98, 99], 0, 2, Ok(&[0x61, 0x62, 0x63])),
    (&[97, 98, 99], 0, 3, Ok(&[0x61, 0x62, 0x63])),
    (&[97, 98, 99], 0, 5, Ok(&[0x61, 0x62, 0x63])),
    (&[97, 98, 99], 1, 0, Ok(&[0x61, 0x62, 0x63])),
    (&[97, 98, 99], 1, 1, Ok(&[0x61, 0x62, 0x63])),
    (&[97, 98, 99], 1, 2, Ok(&[0x61, 0x62, 0x63])),
    (&[97, 98, 99], 1, 3, Ok(&[0x61, 0x62, 0x63])),
    (&[97, 98, 99], 1, 5, Ok(&[0x61, 0x62, 0x63])),
    (&[97, 98, 99], 2, 0, Ok(&[0x61, 0x62, 0x63])),
    (&[97, 98, 99], 2, 1, Ok(&[0x61, 0x62, 0x63])),
    (&[97, 98, 99], 2, 2, Ok(&[0x61, 0x62, 0x63])),
    (&[97, 98, 99], 2, 3, Ok(&[0x61, 0x62, 0x63])),
    (&[97, 98, 99], 2, 5, Ok(&[0x61, 0x62, 0x63])),
    (
        &[255],
        0,
        0,
        Err("'utf-8' codec can't decode byte 0xff in position 0: invalid start byte"),
    ),
    (&[255], 0, 1, Ok(&[])),
    (&[255], 0, 2, Ok(&[0xfffd])),
    (&[255], 0, 3, Ok(&[0x5c, 0x78, 0x66, 0x66])),
    (&[255], 0, 5, Ok(&[0xdcff])),
    (
        &[255],
        1,
        0,
        Err("'ascii' codec can't decode byte 0xff in position 0: ordinal not in range(128)"),
    ),
    (&[255], 1, 1, Ok(&[])),
    (&[255], 1, 2, Ok(&[0xfffd])),
    (&[255], 1, 3, Ok(&[0x5c, 0x78, 0x66, 0x66])),
    (&[255], 1, 5, Ok(&[0xdcff])),
    (&[255], 2, 0, Ok(&[0xff])),
    (&[255], 2, 1, Ok(&[0xff])),
    (&[255], 2, 2, Ok(&[0xff])),
    (&[255], 2, 3, Ok(&[0xff])),
    (&[255], 2, 5, Ok(&[0xff])),
    (
        &[97, 195],
        0,
        0,
        Err("'utf-8' codec can't decode byte 0xc3 in position 1: unexpected end of data"),
    ),
    (&[97, 195], 0, 1, Ok(&[0x61])),
    (&[97, 195], 0, 2, Ok(&[0x61, 0xfffd])),
    (&[97, 195], 0, 3, Ok(&[0x61, 0x5c, 0x78, 0x63, 0x33])),
    (&[97, 195], 0, 5, Ok(&[0x61, 0xdcc3])),
    (
        &[97, 195],
        1,
        0,
        Err("'ascii' codec can't decode byte 0xc3 in position 1: ordinal not in range(128)"),
    ),
    (&[97, 195], 1, 1, Ok(&[0x61])),
    (&[97, 195], 1, 2, Ok(&[0x61, 0xfffd])),
    (&[97, 195], 1, 3, Ok(&[0x61, 0x5c, 0x78, 0x63, 0x33])),
    (&[97, 195], 1, 5, Ok(&[0x61, 0xdcc3])),
    (&[97, 195], 2, 0, Ok(&[0x61, 0xc3])),
    (&[97, 195], 2, 1, Ok(&[0x61, 0xc3])),
    (&[97, 195], 2, 2, Ok(&[0x61, 0xc3])),
    (&[97, 195], 2, 3, Ok(&[0x61, 0xc3])),
    (&[97, 195], 2, 5, Ok(&[0x61, 0xc3])),
    (
        &[226, 130],
        0,
        0,
        Err("'utf-8' codec can't decode bytes in position 0-1: unexpected end of data"),
    ),
    (&[226, 130], 0, 1, Ok(&[])),
    (&[226, 130], 0, 2, Ok(&[0xfffd])),
    (
        &[226, 130],
        0,
        3,
        Ok(&[0x5c, 0x78, 0x65, 0x32, 0x5c, 0x78, 0x38, 0x32]),
    ),
    (&[226, 130], 0, 5, Ok(&[0xdce2, 0xdc82])),
    (
        &[226, 130],
        1,
        0,
        Err("'ascii' codec can't decode byte 0xe2 in position 0: ordinal not in range(128)"),
    ),
    (&[226, 130], 1, 1, Ok(&[])),
    (&[226, 130], 1, 2, Ok(&[0xfffd, 0xfffd])),
    (
        &[226, 130],
        1,
        3,
        Ok(&[0x5c, 0x78, 0x65, 0x32, 0x5c, 0x78, 0x38, 0x32]),
    ),
    (&[226, 130], 1, 5, Ok(&[0xdce2, 0xdc82])),
    (&[226, 130], 2, 0, Ok(&[0xe2, 0x82])),
    (&[226, 130], 2, 1, Ok(&[0xe2, 0x82])),
    (&[226, 130], 2, 2, Ok(&[0xe2, 0x82])),
    (&[226, 130], 2, 3, Ok(&[0xe2, 0x82])),
    (&[226, 130], 2, 5, Ok(&[0xe2, 0x82])),
    (
        &[226, 40, 161],
        0,
        0,
        Err("'utf-8' codec can't decode byte 0xe2 in position 0: invalid continuation byte"),
    ),
    (&[226, 40, 161], 0, 1, Ok(&[0x28])),
    (&[226, 40, 161], 0, 2, Ok(&[0xfffd, 0x28, 0xfffd])),
    (
        &[226, 40, 161],
        0,
        3,
        Ok(&[0x5c, 0x78, 0x65, 0x32, 0x28, 0x5c, 0x78, 0x61, 0x31]),
    ),
    (&[226, 40, 161], 0, 5, Ok(&[0xdce2, 0x28, 0xdca1])),
    (
        &[226, 40, 161],
        1,
        0,
        Err("'ascii' codec can't decode byte 0xe2 in position 0: ordinal not in range(128)"),
    ),
    (&[226, 40, 161], 1, 1, Ok(&[0x28])),
    (&[226, 40, 161], 1, 2, Ok(&[0xfffd, 0x28, 0xfffd])),
    (
        &[226, 40, 161],
        1,
        3,
        Ok(&[0x5c, 0x78, 0x65, 0x32, 0x28, 0x5c, 0x78, 0x61, 0x31]),
    ),
    (&[226, 40, 161], 1, 5, Ok(&[0xdce2, 0x28, 0xdca1])),
    (&[226, 40, 161], 2, 0, Ok(&[0xe2, 0x28, 0xa1])),
    (&[226, 40, 161], 2, 1, Ok(&[0xe2, 0x28, 0xa1])),
    (&[226, 40, 161], 2, 2, Ok(&[0xe2, 0x28, 0xa1])),
    (&[226, 40, 161], 2, 3, Ok(&[0xe2, 0x28, 0xa1])),
    (&[226, 40, 161], 2, 5, Ok(&[0xe2, 0x28, 0xa1])),
    (
        &[237, 160, 128],
        0,
        0,
        Err("'utf-8' codec can't decode byte 0xed in position 0: invalid continuation byte"),
    ),
    (&[237, 160, 128], 0, 1, Ok(&[])),
    (&[237, 160, 128], 0, 2, Ok(&[0xfffd, 0xfffd, 0xfffd])),
    (
        &[237, 160, 128],
        0,
        3,
        Ok(&[
            0x5c, 0x78, 0x65, 0x64, 0x5c, 0x78, 0x61, 0x30, 0x5c, 0x78, 0x38, 0x30,
        ]),
    ),
    (&[237, 160, 128], 0, 5, Ok(&[0xdced, 0xdca0, 0xdc80])),
    (
        &[237, 160, 128],
        1,
        0,
        Err("'ascii' codec can't decode byte 0xed in position 0: ordinal not in range(128)"),
    ),
    (&[237, 160, 128], 1, 1, Ok(&[])),
    (&[237, 160, 128], 1, 2, Ok(&[0xfffd, 0xfffd, 0xfffd])),
    (
        &[237, 160, 128],
        1,
        3,
        Ok(&[
            0x5c, 0x78, 0x65, 0x64, 0x5c, 0x78, 0x61, 0x30, 0x5c, 0x78, 0x38, 0x30,
        ]),
    ),
    (&[237, 160, 128], 1, 5, Ok(&[0xdced, 0xdca0, 0xdc80])),
    (&[237, 160, 128], 2, 0, Ok(&[0xed, 0xa0, 0x80])),
    (&[237, 160, 128], 2, 1, Ok(&[0xed, 0xa0, 0x80])),
    (&[237, 160, 128], 2, 2, Ok(&[0xed, 0xa0, 0x80])),
    (&[237, 160, 128], 2, 3, Ok(&[0xed, 0xa0, 0x80])),
    (&[237, 160, 128], 2, 5, Ok(&[0xed, 0xa0, 0x80])),
    (
        &[240, 144, 128],
        0,
        0,
        Err("'utf-8' codec can't decode bytes in position 0-2: unexpected end of data"),
    ),
    (&[240, 144, 128], 0, 1, Ok(&[])),
    (&[240, 144, 128], 0, 2, Ok(&[0xfffd])),
    (
        &[240, 144, 128],
        0,
        3,
        Ok(&[
            0x5c, 0x78, 0x66, 0x30, 0x5c, 0x78, 0x39, 0x30, 0x5c, 0x78, 0x38, 0x30,
        ]),
    ),
    (&[240, 144, 128], 0, 5, Ok(&[0xdcf0, 0xdc90, 0xdc80])),
    (
        &[240, 144, 128],
        1,
        0,
        Err("'ascii' codec can't decode byte 0xf0 in position 0: ordinal not in range(128)"),
    ),
    (&[240, 144, 128], 1, 1, Ok(&[])),
    (&[240, 144, 128], 1, 2, Ok(&[0xfffd, 0xfffd, 0xfffd])),
    (
        &[240, 144, 128],
        1,
        3,
        Ok(&[
            0x5c, 0x78, 0x66, 0x30, 0x5c, 0x78, 0x39, 0x30, 0x5c, 0x78, 0x38, 0x30,
        ]),
    ),
    (&[240, 144, 128], 1, 5, Ok(&[0xdcf0, 0xdc90, 0xdc80])),
    (&[240, 144, 128], 2, 0, Ok(&[0xf0, 0x90, 0x80])),
    (&[240, 144, 128], 2, 1, Ok(&[0xf0, 0x90, 0x80])),
    (&[240, 144, 128], 2, 2, Ok(&[0xf0, 0x90, 0x80])),
    (&[240, 144, 128], 2, 3, Ok(&[0xf0, 0x90, 0x80])),
    (&[240, 144, 128], 2, 5, Ok(&[0xf0, 0x90, 0x80])),
    (
        &[244, 144, 128, 128],
        0,
        0,
        Err("'utf-8' codec can't decode byte 0xf4 in position 0: invalid continuation byte"),
    ),
    (&[244, 144, 128, 128], 0, 1, Ok(&[])),
    (
        &[244, 144, 128, 128],
        0,
        2,
        Ok(&[0xfffd, 0xfffd, 0xfffd, 0xfffd]),
    ),
    (
        &[244, 144, 128, 128],
        0,
        3,
        Ok(&[
            0x5c, 0x78, 0x66, 0x34, 0x5c, 0x78, 0x39, 0x30, 0x5c, 0x78, 0x38, 0x30, 0x5c, 0x78,
            0x38, 0x30,
        ]),
    ),
    (
        &[244, 144, 128, 128],
        0,
        5,
        Ok(&[0xdcf4, 0xdc90, 0xdc80, 0xdc80]),
    ),
    (
        &[244, 144, 128, 128],
        1,
        0,
        Err("'ascii' codec can't decode byte 0xf4 in position 0: ordinal not in range(128)"),
    ),
    (&[244, 144, 128, 128], 1, 1, Ok(&[])),
    (
        &[244, 144, 128, 128],
        1,
        2,
        Ok(&[0xfffd, 0xfffd, 0xfffd, 0xfffd]),
    ),
    (
        &[244, 144, 128, 128],
        1,
        3,
        Ok(&[
            0x5c, 0x78, 0x66, 0x34, 0x5c, 0x78, 0x39, 0x30, 0x5c, 0x78, 0x38, 0x30, 0x5c, 0x78,
            0x38, 0x30,
        ]),
    ),
    (
        &[244, 144, 128, 128],
        1,
        5,
        Ok(&[0xdcf4, 0xdc90, 0xdc80, 0xdc80]),
    ),
    (&[244, 144, 128, 128], 2, 0, Ok(&[0xf4, 0x90, 0x80, 0x80])),
    (&[244, 144, 128, 128], 2, 1, Ok(&[0xf4, 0x90, 0x80, 0x80])),
    (&[244, 144, 128, 128], 2, 2, Ok(&[0xf4, 0x90, 0x80, 0x80])),
    (&[244, 144, 128, 128], 2, 3, Ok(&[0xf4, 0x90, 0x80, 0x80])),
    (&[244, 144, 128, 128], 2, 5, Ok(&[0xf4, 0x90, 0x80, 0x80])),
    (
        &[192, 128],
        0,
        0,
        Err("'utf-8' codec can't decode byte 0xc0 in position 0: invalid start byte"),
    ),
    (&[192, 128], 0, 1, Ok(&[])),
    (&[192, 128], 0, 2, Ok(&[0xfffd, 0xfffd])),
    (
        &[192, 128],
        0,
        3,
        Ok(&[0x5c, 0x78, 0x63, 0x30, 0x5c, 0x78, 0x38, 0x30]),
    ),
    (&[192, 128], 0, 5, Ok(&[0xdcc0, 0xdc80])),
    (
        &[192, 128],
        1,
        0,
        Err("'ascii' codec can't decode byte 0xc0 in position 0: ordinal not in range(128)"),
    ),
    (&[192, 128], 1, 1, Ok(&[])),
    (&[192, 128], 1, 2, Ok(&[0xfffd, 0xfffd])),
    (
        &[192, 128],
        1,
        3,
        Ok(&[0x5c, 0x78, 0x63, 0x30, 0x5c, 0x78, 0x38, 0x30]),
    ),
    (&[192, 128], 1, 5, Ok(&[0xdcc0, 0xdc80])),
    (&[192, 128], 2, 0, Ok(&[0xc0, 0x80])),
    (&[192, 128], 2, 1, Ok(&[0xc0, 0x80])),
    (&[192, 128], 2, 2, Ok(&[0xc0, 0x80])),
    (&[192, 128], 2, 3, Ok(&[0xc0, 0x80])),
    (&[192, 128], 2, 5, Ok(&[0xc0, 0x80])),
    (
        &[128, 97, 98, 99],
        0,
        0,
        Err("'utf-8' codec can't decode byte 0x80 in position 0: invalid start byte"),
    ),
    (&[128, 97, 98, 99], 0, 1, Ok(&[0x61, 0x62, 0x63])),
    (&[128, 97, 98, 99], 0, 2, Ok(&[0xfffd, 0x61, 0x62, 0x63])),
    (
        &[128, 97, 98, 99],
        0,
        3,
        Ok(&[0x5c, 0x78, 0x38, 0x30, 0x61, 0x62, 0x63]),
    ),
    (&[128, 97, 98, 99], 0, 5, Ok(&[0xdc80, 0x61, 0x62, 0x63])),
    (
        &[128, 97, 98, 99],
        1,
        0,
        Err("'ascii' codec can't decode byte 0x80 in position 0: ordinal not in range(128)"),
    ),
    (&[128, 97, 98, 99], 1, 1, Ok(&[0x61, 0x62, 0x63])),
    (&[128, 97, 98, 99], 1, 2, Ok(&[0xfffd, 0x61, 0x62, 0x63])),
    (
        &[128, 97, 98, 99],
        1,
        3,
        Ok(&[0x5c, 0x78, 0x38, 0x30, 0x61, 0x62, 0x63]),
    ),
    (&[128, 97, 98, 99], 1, 5, Ok(&[0xdc80, 0x61, 0x62, 0x63])),
    (&[128, 97, 98, 99], 2, 0, Ok(&[0x80, 0x61, 0x62, 0x63])),
    (&[128, 97, 98, 99], 2, 1, Ok(&[0x80, 0x61, 0x62, 0x63])),
    (&[128, 97, 98, 99], 2, 2, Ok(&[0x80, 0x61, 0x62, 0x63])),
    (&[128, 97, 98, 99], 2, 3, Ok(&[0x80, 0x61, 0x62, 0x63])),
    (&[128, 97, 98, 99], 2, 5, Ok(&[0x80, 0x61, 0x62, 0x63])),
    (
        &[226, 130, 172, 240, 159, 152, 128],
        0,
        0,
        Ok(&[0x20ac, 0x1f600]),
    ),
    (
        &[226, 130, 172, 240, 159, 152, 128],
        0,
        1,
        Ok(&[0x20ac, 0x1f600]),
    ),
    (
        &[226, 130, 172, 240, 159, 152, 128],
        0,
        2,
        Ok(&[0x20ac, 0x1f600]),
    ),
    (
        &[226, 130, 172, 240, 159, 152, 128],
        0,
        3,
        Ok(&[0x20ac, 0x1f600]),
    ),
    (
        &[226, 130, 172, 240, 159, 152, 128],
        0,
        5,
        Ok(&[0x20ac, 0x1f600]),
    ),
    (
        &[226, 130, 172, 240, 159, 152, 128],
        1,
        0,
        Err("'ascii' codec can't decode byte 0xe2 in position 0: ordinal not in range(128)"),
    ),
    (&[226, 130, 172, 240, 159, 152, 128], 1, 1, Ok(&[])),
    (
        &[226, 130, 172, 240, 159, 152, 128],
        1,
        2,
        Ok(&[0xfffd, 0xfffd, 0xfffd, 0xfffd, 0xfffd, 0xfffd, 0xfffd]),
    ),
    (
        &[226, 130, 172, 240, 159, 152, 128],
        1,
        3,
        Ok(&[
            0x5c, 0x78, 0x65, 0x32, 0x5c, 0x78, 0x38, 0x32, 0x5c, 0x78, 0x61, 0x63, 0x5c, 0x78,
            0x66, 0x30, 0x5c, 0x78, 0x39, 0x66, 0x5c, 0x78, 0x39, 0x38, 0x5c, 0x78, 0x38, 0x30,
        ]),
    ),
    (
        &[226, 130, 172, 240, 159, 152, 128],
        1,
        5,
        Ok(&[0xdce2, 0xdc82, 0xdcac, 0xdcf0, 0xdc9f, 0xdc98, 0xdc80]),
    ),
    (
        &[226, 130, 172, 240, 159, 152, 128],
        2,
        0,
        Ok(&[0xe2, 0x82, 0xac, 0xf0, 0x9f, 0x98, 0x80]),
    ),
    (
        &[226, 130, 172, 240, 159, 152, 128],
        2,
        1,
        Ok(&[0xe2, 0x82, 0xac, 0xf0, 0x9f, 0x98, 0x80]),
    ),
    (
        &[226, 130, 172, 240, 159, 152, 128],
        2,
        2,
        Ok(&[0xe2, 0x82, 0xac, 0xf0, 0x9f, 0x98, 0x80]),
    ),
    (
        &[226, 130, 172, 240, 159, 152, 128],
        2,
        3,
        Ok(&[0xe2, 0x82, 0xac, 0xf0, 0x9f, 0x98, 0x80]),
    ),
    (
        &[226, 130, 172, 240, 159, 152, 128],
        2,
        5,
        Ok(&[0xe2, 0x82, 0xac, 0xf0, 0x9f, 0x98, 0x80]),
    ),
    (
        &[97, 255, 98, 226, 130],
        0,
        0,
        Err("'utf-8' codec can't decode byte 0xff in position 1: invalid start byte"),
    ),
    (&[97, 255, 98, 226, 130], 0, 1, Ok(&[0x61, 0x62])),
    (
        &[97, 255, 98, 226, 130],
        0,
        2,
        Ok(&[0x61, 0xfffd, 0x62, 0xfffd]),
    ),
    (
        &[97, 255, 98, 226, 130],
        0,
        3,
        Ok(&[
            0x61, 0x5c, 0x78, 0x66, 0x66, 0x62, 0x5c, 0x78, 0x65, 0x32, 0x5c, 0x78, 0x38, 0x32,
        ]),
    ),
    (
        &[97, 255, 98, 226, 130],
        0,
        5,
        Ok(&[0x61, 0xdcff, 0x62, 0xdce2, 0xdc82]),
    ),
    (
        &[97, 255, 98, 226, 130],
        1,
        0,
        Err("'ascii' codec can't decode byte 0xff in position 1: ordinal not in range(128)"),
    ),
    (&[97, 255, 98, 226, 130], 1, 1, Ok(&[0x61, 0x62])),
    (
        &[97, 255, 98, 226, 130],
        1,
        2,
        Ok(&[0x61, 0xfffd, 0x62, 0xfffd, 0xfffd]),
    ),
    (
        &[97, 255, 98, 226, 130],
        1,
        3,
        Ok(&[
            0x61, 0x5c, 0x78, 0x66, 0x66, 0x62, 0x5c, 0x78, 0x65, 0x32, 0x5c, 0x78, 0x38, 0x32,
        ]),
    ),
    (
        &[97, 255, 98, 226, 130],
        1,
        5,
        Ok(&[0x61, 0xdcff, 0x62, 0xdce2, 0xdc82]),
    ),
    (
        &[97, 255, 98, 226, 130],
        2,
        0,
        Ok(&[0x61, 0xff, 0x62, 0xe2, 0x82]),
    ),
    (
        &[97, 255, 98, 226, 130],
        2,
        1,
        Ok(&[0x61, 0xff, 0x62, 0xe2, 0x82]),
    ),
    (
        &[97, 255, 98, 226, 130],
        2,
        2,
        Ok(&[0x61, 0xff, 0x62, 0xe2, 0x82]),
    ),
    (
        &[97, 255, 98, 226, 130],
        2,
        3,
        Ok(&[0x61, 0xff, 0x62, 0xe2, 0x82]),
    ),
    (
        &[97, 255, 98, 226, 130],
        2,
        5,
        Ok(&[0x61, 0xff, 0x62, 0xe2, 0x82]),
    ),
    (
        &[224, 128, 128],
        0,
        0,
        Err("'utf-8' codec can't decode byte 0xe0 in position 0: invalid continuation byte"),
    ),
    (&[224, 128, 128], 0, 1, Ok(&[])),
    (&[224, 128, 128], 0, 2, Ok(&[0xfffd, 0xfffd, 0xfffd])),
    (
        &[224, 128, 128],
        0,
        3,
        Ok(&[
            0x5c, 0x78, 0x65, 0x30, 0x5c, 0x78, 0x38, 0x30, 0x5c, 0x78, 0x38, 0x30,
        ]),
    ),
    (&[224, 128, 128], 0, 5, Ok(&[0xdce0, 0xdc80, 0xdc80])),
    (
        &[224, 128, 128],
        1,
        0,
        Err("'ascii' codec can't decode byte 0xe0 in position 0: ordinal not in range(128)"),
    ),
    (&[224, 128, 128], 1, 1, Ok(&[])),
    (&[224, 128, 128], 1, 2, Ok(&[0xfffd, 0xfffd, 0xfffd])),
    (
        &[224, 128, 128],
        1,
        3,
        Ok(&[
            0x5c, 0x78, 0x65, 0x30, 0x5c, 0x78, 0x38, 0x30, 0x5c, 0x78, 0x38, 0x30,
        ]),
    ),
    (&[224, 128, 128], 1, 5, Ok(&[0xdce0, 0xdc80, 0xdc80])),
    (&[224, 128, 128], 2, 0, Ok(&[0xe0, 0x80, 0x80])),
    (&[224, 128, 128], 2, 1, Ok(&[0xe0, 0x80, 0x80])),
    (&[224, 128, 128], 2, 2, Ok(&[0xe0, 0x80, 0x80])),
    (&[224, 128, 128], 2, 3, Ok(&[0xe0, 0x80, 0x80])),
    (&[224, 128, 128], 2, 5, Ok(&[0xe0, 0x80, 0x80])),
    (
        &[240, 128, 128, 128],
        0,
        0,
        Err("'utf-8' codec can't decode byte 0xf0 in position 0: invalid continuation byte"),
    ),
    (&[240, 128, 128, 128], 0, 1, Ok(&[])),
    (
        &[240, 128, 128, 128],
        0,
        2,
        Ok(&[0xfffd, 0xfffd, 0xfffd, 0xfffd]),
    ),
    (
        &[240, 128, 128, 128],
        0,
        3,
        Ok(&[
            0x5c, 0x78, 0x66, 0x30, 0x5c, 0x78, 0x38, 0x30, 0x5c, 0x78, 0x38, 0x30, 0x5c, 0x78,
            0x38, 0x30,
        ]),
    ),
    (
        &[240, 128, 128, 128],
        0,
        5,
        Ok(&[0xdcf0, 0xdc80, 0xdc80, 0xdc80]),
    ),
    (
        &[240, 128, 128, 128],
        1,
        0,
        Err("'ascii' codec can't decode byte 0xf0 in position 0: ordinal not in range(128)"),
    ),
    (&[240, 128, 128, 128], 1, 1, Ok(&[])),
    (
        &[240, 128, 128, 128],
        1,
        2,
        Ok(&[0xfffd, 0xfffd, 0xfffd, 0xfffd]),
    ),
    (
        &[240, 128, 128, 128],
        1,
        3,
        Ok(&[
            0x5c, 0x78, 0x66, 0x30, 0x5c, 0x78, 0x38, 0x30, 0x5c, 0x78, 0x38, 0x30, 0x5c, 0x78,
            0x38, 0x30,
        ]),
    ),
    (
        &[240, 128, 128, 128],
        1,
        5,
        Ok(&[0xdcf0, 0xdc80, 0xdc80, 0xdc80]),
    ),
    (&[240, 128, 128, 128], 2, 0, Ok(&[0xf0, 0x80, 0x80, 0x80])),
    (&[240, 128, 128, 128], 2, 1, Ok(&[0xf0, 0x80, 0x80, 0x80])),
    (&[240, 128, 128, 128], 2, 2, Ok(&[0xf0, 0x80, 0x80, 0x80])),
    (&[240, 128, 128, 128], 2, 3, Ok(&[0xf0, 0x80, 0x80, 0x80])),
    (&[240, 128, 128, 128], 2, 5, Ok(&[0xf0, 0x80, 0x80, 0x80])),
    (
        &[245],
        0,
        0,
        Err("'utf-8' codec can't decode byte 0xf5 in position 0: invalid start byte"),
    ),
    (&[245], 0, 1, Ok(&[])),
    (&[245], 0, 2, Ok(&[0xfffd])),
    (&[245], 0, 3, Ok(&[0x5c, 0x78, 0x66, 0x35])),
    (&[245], 0, 5, Ok(&[0xdcf5])),
    (
        &[245],
        1,
        0,
        Err("'ascii' codec can't decode byte 0xf5 in position 0: ordinal not in range(128)"),
    ),
    (&[245], 1, 1, Ok(&[])),
    (&[245], 1, 2, Ok(&[0xfffd])),
    (&[245], 1, 3, Ok(&[0x5c, 0x78, 0x66, 0x35])),
    (&[245], 1, 5, Ok(&[0xdcf5])),
    (&[245], 2, 0, Ok(&[0xf5])),
    (&[245], 2, 1, Ok(&[0xf5])),
    (&[245], 2, 2, Ok(&[0xf5])),
    (&[245], 2, 3, Ok(&[0xf5])),
    (&[245], 2, 5, Ok(&[0xf5])),
    (
        &[195, 169, 195],
        0,
        0,
        Err("'utf-8' codec can't decode byte 0xc3 in position 2: unexpected end of data"),
    ),
    (&[195, 169, 195], 0, 1, Ok(&[0xe9])),
    (&[195, 169, 195], 0, 2, Ok(&[0xe9, 0xfffd])),
    (&[195, 169, 195], 0, 3, Ok(&[0xe9, 0x5c, 0x78, 0x63, 0x33])),
    (&[195, 169, 195], 0, 5, Ok(&[0xe9, 0xdcc3])),
    (
        &[195, 169, 195],
        1,
        0,
        Err("'ascii' codec can't decode byte 0xc3 in position 0: ordinal not in range(128)"),
    ),
    (&[195, 169, 195], 1, 1, Ok(&[])),
    (&[195, 169, 195], 1, 2, Ok(&[0xfffd, 0xfffd, 0xfffd])),
    (
        &[195, 169, 195],
        1,
        3,
        Ok(&[
            0x5c, 0x78, 0x63, 0x33, 0x5c, 0x78, 0x61, 0x39, 0x5c, 0x78, 0x63, 0x33,
        ]),
    ),
    (&[195, 169, 195], 1, 5, Ok(&[0xdcc3, 0xdca9, 0xdcc3])),
    (&[195, 169, 195], 2, 0, Ok(&[0xc3, 0xa9, 0xc3])),
    (&[195, 169, 195], 2, 1, Ok(&[0xc3, 0xa9, 0xc3])),
    (&[195, 169, 195], 2, 2, Ok(&[0xc3, 0xa9, 0xc3])),
    (&[195, 169, 195], 2, 3, Ok(&[0xc3, 0xa9, 0xc3])),
    (&[195, 169, 195], 2, 5, Ok(&[0xc3, 0xa9, 0xc3])),
    (&[239, 191, 189], 0, 0, Ok(&[0xfffd])),
    (&[239, 191, 189], 0, 1, Ok(&[0xfffd])),
    (&[239, 191, 189], 0, 2, Ok(&[0xfffd])),
    (&[239, 191, 189], 0, 3, Ok(&[0xfffd])),
    (&[239, 191, 189], 0, 5, Ok(&[0xfffd])),
    (
        &[239, 191, 189],
        1,
        0,
        Err("'ascii' codec can't decode byte 0xef in position 0: ordinal not in range(128)"),
    ),
    (&[239, 191, 189], 1, 1, Ok(&[])),
    (&[239, 191, 189], 1, 2, Ok(&[0xfffd, 0xfffd, 0xfffd])),
    (
        &[239, 191, 189],
        1,
        3,
        Ok(&[
            0x5c, 0x78, 0x65, 0x66, 0x5c, 0x78, 0x62, 0x66, 0x5c, 0x78, 0x62, 0x64,
        ]),
    ),
    (&[239, 191, 189], 1, 5, Ok(&[0xdcef, 0xdcbf, 0xdcbd])),
    (&[239, 191, 189], 2, 0, Ok(&[0xef, 0xbf, 0xbd])),
    (&[239, 191, 189], 2, 1, Ok(&[0xef, 0xbf, 0xbd])),
    (&[239, 191, 189], 2, 2, Ok(&[0xef, 0xbf, 0xbd])),
    (&[239, 191, 189], 2, 3, Ok(&[0xef, 0xbf, 0xbd])),
    (&[239, 191, 189], 2, 5, Ok(&[0xef, 0xbf, 0xbd])),
    (&[241, 128, 128, 128], 0, 0, Ok(&[0x40000])),
    (&[241, 128, 128, 128], 0, 1, Ok(&[0x40000])),
    (&[241, 128, 128, 128], 0, 2, Ok(&[0x40000])),
    (&[241, 128, 128, 128], 0, 3, Ok(&[0x40000])),
    (&[241, 128, 128, 128], 0, 5, Ok(&[0x40000])),
    (
        &[241, 128, 128, 128],
        1,
        0,
        Err("'ascii' codec can't decode byte 0xf1 in position 0: ordinal not in range(128)"),
    ),
    (&[241, 128, 128, 128], 1, 1, Ok(&[])),
    (
        &[241, 128, 128, 128],
        1,
        2,
        Ok(&[0xfffd, 0xfffd, 0xfffd, 0xfffd]),
    ),
    (
        &[241, 128, 128, 128],
        1,
        3,
        Ok(&[
            0x5c, 0x78, 0x66, 0x31, 0x5c, 0x78, 0x38, 0x30, 0x5c, 0x78, 0x38, 0x30, 0x5c, 0x78,
            0x38, 0x30,
        ]),
    ),
    (
        &[241, 128, 128, 128],
        1,
        5,
        Ok(&[0xdcf1, 0xdc80, 0xdc80, 0xdc80]),
    ),
    (&[241, 128, 128, 128], 2, 0, Ok(&[0xf1, 0x80, 0x80, 0x80])),
    (&[241, 128, 128, 128], 2, 1, Ok(&[0xf1, 0x80, 0x80, 0x80])),
    (&[241, 128, 128, 128], 2, 2, Ok(&[0xf1, 0x80, 0x80, 0x80])),
    (&[241, 128, 128, 128], 2, 3, Ok(&[0xf1, 0x80, 0x80, 0x80])),
    (&[241, 128, 128, 128], 2, 5, Ok(&[0xf1, 0x80, 0x80, 0x80])),
];

#[test]
fn encoding_gives_cpython_s_bytes_and_errors() {
    for (whole, codec, handler, expected) in ENCODINGS {
        // SAFETY: the string is live.
        let encoded = unsafe {
            typy_rt_str_encode(
                string_of(whole),
                inline(*codec).unwrap(),
                inline(*handler).unwrap(),
            )
        };
        let result = if encoded.exception.is_null() {
            Ok(points(encoded.value)
                .into_iter()
                .map(|point| u8::try_from(point).unwrap())
                .collect::<Vec<u8>>())
        } else {
            Err(message(encoded.exception))
        };
        let wanted = expected.map(<[u8]>::to_vec).map_err(str::to_owned);
        assert_eq!(result, wanted, "{whole:?} codec {codec} handler {handler}");
    }
}

#[test]
fn decoding_gives_cpython_s_text_and_errors() {
    for (data, codec, handler, expected) in DECODINGS {
        // SAFETY: the bytes are live.
        let decoded = unsafe {
            typy_rt_bytes_decode(
                bytes(data),
                inline(*codec).unwrap(),
                inline(*handler).unwrap(),
            )
        };
        let result = if decoded.exception.is_null() {
            Ok(points(decoded.value))
        } else {
            Err(message(decoded.exception))
        };
        let wanted = expected.map(<[u32]>::to_vec).map_err(str::to_owned);
        assert_eq!(result, wanted, "{data:?} codec {codec} handler {handler}");
    }
}
