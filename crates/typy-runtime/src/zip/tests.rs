//! Tests of the error of `zip(..., strict=True)` for arguments of different lengths.

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
