//! Tests of building the messages of exceptions.

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
