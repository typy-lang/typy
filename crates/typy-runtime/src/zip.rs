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
mod tests;
