//! Encodes a string as UTF-8 for output.

use super::{StrObject, code_point, length};

/// Writes the string as UTF-8 to `sink`, one chunk at a time. A lone surrogate, which UTF-8 cannot hold, is written as `?`.
///
/// # Safety
///
/// `string` points to a live `str` object.
pub(crate) unsafe fn write_utf8(string: *const StrObject, mut sink: impl FnMut(&[u8])) {
    let mut buffer = [0_u8; 256];
    let mut used = 0;
    // SAFETY: the caller passes a live string.
    for index in 0..unsafe { length(string) } {
        // SAFETY: the index is below the length.
        let point = unsafe { code_point(string, index) };
        let character = char::from_u32(point).unwrap_or('?');
        if used + 4 > buffer.len() {
            sink(&buffer[..used]);
            used = 0;
        }
        used += character.encode_utf8(&mut buffer[used..]).len();
    }
    if used > 0 {
        sink(&buffer[..used]);
    }
}

#[cfg(test)]
mod tests {
    use super::write_utf8;
    use crate::string::from_str;

    #[test]
    fn strings_are_written_as_utf8_across_chunks() {
        let text: String = "a\u{e9}\u{3b1}\u{1f600}".repeat(100);
        let string = from_str(&text);
        let mut out = Vec::new();
        // SAFETY: the string is live.
        unsafe { write_utf8(string, |bytes| out.extend_from_slice(bytes)) };
        assert_eq!(String::from_utf8(out).unwrap(), text);
    }
}
