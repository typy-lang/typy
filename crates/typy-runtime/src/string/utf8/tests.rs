//! Tests of encoding strings as UTF-8 for the standard streams.

use super::{Surrogates, unescapable, write_utf8};
use crate::string::{StrBuilder, from_str};

fn with_points(points: &[u32]) -> *mut crate::string::StrObject {
    let mut builder = StrBuilder::with_capacity(points.len());
    for point in points {
        builder.push(*point);
    }
    builder.finish()
}

fn written(points: &[u32], surrogates: Surrogates) -> Vec<u8> {
    let mut out = Vec::new();
    // SAFETY: the string is live.
    unsafe {
        write_utf8(with_points(points), surrogates, |bytes| {
            out.extend_from_slice(bytes);
        });
    };
    out
}

#[test]
fn surrogates_are_written_as_each_standard_stream_writes_them() {
    let text = [0x61, 0xdc80, 0xdcff, 0xd800, 0xe9];
    assert_eq!(written(&text, Surrogates::Escape), b"a\x80\xff?\xc3\xa9");
    assert_eq!(
        written(&text, Surrogates::Backslash),
        b"a\\udc80\\udcff\\ud800\xc3\xa9"
    );
    let long: Vec<u32> = std::iter::repeat_n(0xd800, 100).collect();
    assert_eq!(
        written(&long, Surrogates::Backslash),
        b"\\ud800".repeat(100)
    );
}

#[test]
fn the_unescapable_surrogates_run_from_the_first_to_the_end_of_their_run() {
    type Case = (&'static [u32], Option<(usize, usize)>);
    let cases: &[Case] = &[
        (&[0x61, 0xdc80, 0xdcff], None),
        (&[0xd800, 0xdc80], Some((0, 2))),
        (&[0xdc80, 0xd800, 0x78], Some((1, 2))),
        (&[0xdc80, 0xd800, 0xdc81, 0x78, 0xdfff], Some((1, 3))),
    ];
    for (points, expected) in cases {
        // SAFETY: the string is live.
        let found = unsafe { unescapable(with_points(points)) };
        assert_eq!(found, *expected, "{points:x?}");
    }
}

#[test]
fn strings_are_written_as_utf8_across_chunks() {
    let text: String = "a\u{e9}\u{3b1}\u{1f600}".repeat(100);
    let string = from_str(&text);
    let mut out = Vec::new();
    // SAFETY: the string is live.
    unsafe {
        write_utf8(string, Surrogates::Escape, |bytes| {
            out.extend_from_slice(bytes);
        });
    };
    assert_eq!(String::from_utf8(out).unwrap(), text);
}
