//! Prints an exception nothing caught, as CPython does, ending with its class and message (STD-05).

use super::ExceptionObject;
use crate::io::{flush_stdout, write_stderr};
use crate::string::write_utf8;

/// Writes the traceback and the last line, `Class: message`, or `Class` alone when the message is empty.
///
/// # Safety
///
/// `exception` points to a live exception.
pub(crate) unsafe fn report_uncaught(exception: *const ExceptionObject) {
    flush_stdout();
    write_stderr(b"Traceback (most recent call last):\n");
    // SAFETY: the caller passes a live exception, whose entries hold static, null-terminated names.
    unsafe {
        let traceback = &(*exception).traceback;
        for index in (0..traceback.len()).rev() {
            let entry = traceback.get(index);
            write_stderr(b"  File \"");
            write_stderr(c_bytes(entry.file));
            write_stderr(b"\", line ");
            write_number(entry.line);
            write_stderr(b", in ");
            write_stderr(c_bytes(entry.function));
            write_stderr(b"\n");
        }
        write_stderr((*exception).kind.name().as_bytes());
        let message = (*exception).message;
        if crate::string::length(message) > 0 {
            write_stderr(b": ");
            write_utf8(message, write_stderr);
        }
    }
    write_stderr(b"\n");
}

/// Returns the bytes of a null-terminated string, without the terminator.
///
/// # Safety
///
/// `text` points to a null-terminated string that lives for the whole program.
unsafe fn c_bytes(text: *const u8) -> &'static [u8] {
    let mut length = 0;
    // SAFETY: the string is null-terminated, so every byte up to the terminator is readable.
    unsafe {
        while *text.add(length) != 0 {
            length += 1;
        }
        core::slice::from_raw_parts(text, length)
    }
}

fn write_number(mut value: u32) {
    let mut digits = [0_u8; 10];
    let mut start = digits.len();
    loop {
        start -= 1;
        digits[start] = b'0' + u8::try_from(value % 10).unwrap_or(0);
        value /= 10;
        if value == 0 {
            break;
        }
    }
    write_stderr(&digits[start..]);
}

#[cfg(test)]
mod tests {
    use super::report_uncaught;
    use crate::exception::{ExceptionKind, raise, typy_rt_traceback_add};

    #[test]
    fn reporting_an_exception_with_frames_and_an_empty_message_completes() {
        for message in ["division by zero", ""] {
            let exception = raise(ExceptionKind::ZeroDivisionError, message);
            // SAFETY: the exception is live, the names are static, and its reference is dropped at the end.
            unsafe {
                typy_rt_traceback_add(
                    exception,
                    c"f".as_ptr().cast(),
                    c"app.py".as_ptr().cast(),
                    12,
                );
                report_uncaught(exception);
                crate::refcount::typy_rt_decrement(exception.cast());
            }
        }
    }
}
