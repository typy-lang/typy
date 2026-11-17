//! Encodes a string as UTF-8 for the standard streams, whose error handlers are those of
//! CPython's UTF-8 mode: `surrogateescape` for standard output and `backslashreplace` for
//! standard error (LEX-10).

use super::{StrObject, code_point, length};

/// How a stream writes a surrogate, which UTF-8 cannot hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Surrogates {
    /// `surrogateescape`: U+DC80 to U+DCFF as the bytes 0x80 to 0xFF. A program that can
    /// hold any other surrogate checks for it with `unescapable` first and raises.
    Escape,
    /// `backslashreplace`: `\ud800`.
    Backslash,
}

/// Writes the string as UTF-8 to `sink`, one chunk at a time, each surrogate as `surrogates`
/// says.
///
/// # Safety
///
/// `string` points to a live `str` object.
pub(crate) unsafe fn write_utf8(
    string: *const StrObject,
    surrogates: Surrogates,
    mut sink: impl FnMut(&[u8]),
) {
    let mut buffer = [0_u8; 256];
    let mut used = 0;
    // SAFETY: the caller passes a live string.
    for index in 0..unsafe { length(string) } {
        // A backslash escape takes at most six bytes, and a character four.
        if used + 6 > buffer.len() {
            sink(&buffer[..used]);
            used = 0;
        }
        // SAFETY: the index is below the length.
        let point = unsafe { code_point(string, index) };
        if let Some(character) = char::from_u32(point) {
            used += character.encode_utf8(&mut buffer[used..]).len();
            continue;
        }
        match surrogates {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "U+DC80 to U+DCFF stand for the bytes 0x80 to 0xFF"
            )]
            Surrogates::Escape if (0xdc80..=0xdcff).contains(&point) => {
                buffer[used] = (point - 0xdc00) as u8;
                used += 1;
            }
            Surrogates::Escape => {
                buffer[used] = b'?';
                used += 1;
            }
            Surrogates::Backslash => crate::codec::escape(point, &mut |each| {
                #[allow(clippy::cast_possible_truncation, reason = "an escape is ASCII")]
                let byte = each as u8;
                buffer[used] = byte;
                used += 1;
            }),
        }
    }
    if used > 0 {
        sink(&buffer[..used]);
    }
}

/// Returns where `surrogateescape` fails to write the string as UTF-8, as CPython's encoder
/// reports it: from the first surrogate outside U+DC80 to U+DCFF to the end of its run of
/// surrogates.
///
/// # Safety
///
/// `string` points to a live `str` object.
pub(crate) unsafe fn unescapable(string: *const StrObject) -> Option<(usize, usize)> {
    // SAFETY: the caller passes a live string; every index stays below its length.
    unsafe {
        let count = length(string);
        let surrogate = |index: usize| (0xd800..=0xdfff).contains(&code_point(string, index));
        let start = (0..count).find(|index| {
            let point = code_point(string, *index);
            (0xd800..=0xdfff).contains(&point) && !(0xdc80..=0xdcff).contains(&point)
        })?;
        let end = (start..count)
            .find(|index| !surrogate(*index))
            .unwrap_or(count);
        Some((start, end))
    }
}

#[cfg(test)]
mod tests {
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
}
