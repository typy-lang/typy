//! The error `zip(..., strict=True)` raises when its arguments have different lengths.

use crate::exception::{ExceptionKind, ExceptionObject};
use crate::message::MessageBuffer;
use crate::word::inline_value;

/// Raises `ValueError` for argument `index`, counted from 0, which ended before the arguments
/// before it, or with `longer`, an `int` word that is not 0, had items left after the first
/// argument ended. The message is CPython's: `zip() argument 2 is shorter than argument 1`, or
/// `zip() argument 3 is longer than arguments 1-2`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_zip_unequal(index: i64, longer: i64) -> *mut ExceptionObject {
    let index = inline_value(index);
    let mut message = MessageBuffer::new();
    message.push_str("zip() argument ");
    message.push_number(index + 1);
    message.push_str(if inline_value(longer) == 0 {
        " is shorter than argument"
    } else {
        " is longer than argument"
    });
    message.push_str(if index == 1 { " " } else { "s 1-" });
    message.push_number(index);
    message.raise(ExceptionKind::ValueError)
}

#[cfg(test)]
mod tests {
    use super::typy_rt_zip_unequal;
    use crate::word::inline;

    fn message(index: i64, longer: i64) -> String {
        let exception = typy_rt_zip_unequal(inline(index).unwrap(), inline(longer).unwrap());
        // SAFETY: the exception is live; its message is a live string.
        let text = unsafe { (*exception).message };
        // SAFETY: the message is live and every index is in bounds.
        (0..unsafe { crate::string::length(text) })
            .map(|position| {
                // SAFETY: the message is live and the position is in bounds.
                char::from_u32(unsafe { crate::string::code_point(text, position) }).unwrap()
            })
            .collect()
    }

    #[test]
    fn messages_name_the_arguments_as_cpython_does() {
        assert_eq!(message(1, 0), "zip() argument 2 is shorter than argument 1");
        assert_eq!(
            message(2, 0),
            "zip() argument 3 is shorter than arguments 1-2"
        );
        assert_eq!(message(1, 1), "zip() argument 2 is longer than argument 1");
        assert_eq!(
            message(3, 1),
            "zip() argument 4 is longer than arguments 1-3"
        );
    }
}
