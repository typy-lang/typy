//! Builds the message of an exception from text, numbers and `repr` of a string.

use crate::exception::{ExceptionKind, ExceptionObject, raise_with_message};
use crate::string::{StrKind, StrObject, new_string, repr_into, set_code_point};

/// A message of up to 1024 code points, built without allocating until it is raised; the rest
/// is dropped, so an error about a very long input stays short.
pub(crate) struct MessageBuffer {
    points: [u32; 1024],
    used: usize,
}

impl MessageBuffer {
    /// Returns an empty message.
    pub(crate) const fn new() -> Self {
        Self {
            points: [0; 1024],
            used: 0,
        }
    }

    /// Appends one code point.
    pub(crate) fn push(&mut self, point: u32) {
        if self.used < self.points.len() {
            self.points[self.used] = point;
            self.used += 1;
        }
    }

    /// Appends text.
    pub(crate) fn push_str(&mut self, text: &str) {
        for character in text.chars() {
            self.push(u32::from(character));
        }
    }

    /// Appends a number in decimal.
    pub(crate) fn push_number(&mut self, value: i64) {
        let mut digits = [0_u8; 20];
        let mut start = digits.len();
        let mut rest = value.unsigned_abs();
        loop {
            start -= 1;
            digits[start] = b'0' + u8::try_from(rest % 10).unwrap_or(0);
            rest /= 10;
            if rest == 0 {
                break;
            }
        }
        if value < 0 {
            self.push(u32::from(b'-'));
        }
        for digit in &digits[start..] {
            self.push(u32::from(*digit));
        }
    }

    /// Appends `repr(text)`.
    ///
    /// # Safety
    ///
    /// `text` points to a live `str` object.
    pub(crate) unsafe fn push_repr(&mut self, text: *const StrObject) {
        // SAFETY: the caller passes a live string.
        unsafe { repr_into(text, |point| self.push(point)) };
    }

    /// Appends the code points of `text`.
    ///
    /// # Safety
    ///
    /// `text` points to a live `str` object.
    pub(crate) unsafe fn push_text(&mut self, text: *const StrObject) {
        // SAFETY: the caller passes a live string, and each index is inside it.
        unsafe {
            for index in 0..crate::string::length(text) {
                self.push(crate::string::code_point(text, index));
            }
        }
    }

    /// Returns a new exception of `kind` with the message, owned by the caller.
    pub(crate) fn raise(&self, kind: ExceptionKind) -> *mut ExceptionObject {
        raise_with_message(kind, self.text())
    }

    /// Returns the message as a new `str`, owned by the caller.
    pub(crate) fn text(&self) -> *mut StrObject {
        let used = &self.points[..self.used];
        let text_kind = used
            .iter()
            .map(|point| StrKind::of(*point))
            .max()
            .unwrap_or(StrKind::Latin1);
        let message = new_string(used.len(), text_kind);
        for (index, point) in used.iter().enumerate() {
            // SAFETY: the string is new, the index is in bounds, and its kind holds every point.
            unsafe { set_code_point(message, index, *point) };
        }
        message
    }
}

#[cfg(test)]
mod tests {
    use super::MessageBuffer;
    use crate::exception::ExceptionKind;
    use crate::string::{code_point, from_str, length};

    fn text(buffer: &MessageBuffer) -> String {
        let exception = buffer.raise(ExceptionKind::ValueError);
        // SAFETY: the exception and its message are live.
        unsafe {
            let message = (*exception).message;
            (0..length(message))
                .map(|index| char::from_u32(code_point(message, index)).unwrap())
                .collect()
        }
    }

    #[test]
    fn messages_join_text_numbers_and_quoted_strings() {
        let mut buffer = MessageBuffer::new();
        buffer.push_str("base ");
        buffer.push_number(-36);
        buffer.push_str(": ");
        // SAFETY: the string is live.
        unsafe { buffer.push_repr(from_str("\u{3b1}'")) };
        assert_eq!(text(&buffer), "base -36: \"\u{3b1}'\"");
    }

    #[test]
    fn long_messages_are_cut_at_1024_code_points() {
        let mut buffer = MessageBuffer::new();
        for _ in 0..2000 {
            buffer.push_str("x");
        }
        assert_eq!(text(&buffer).len(), 1024);
    }
}
