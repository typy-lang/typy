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
mod tests;
