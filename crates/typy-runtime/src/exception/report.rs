//! Prints an exception nothing caught, as CPython does, ending with its class and message (STD-05).

use super::ExceptionObject;
use crate::io::{flush_stdout, write_stderr};
use crate::string::write_utf8;

/// The number of identical entries in a row a traceback shows before it counts the rest, as
/// CPython's `TB_RECURSIVE_CUTOFF`; a deep recursion then prints a few lines, not one per frame.
const REPEATED_CUTOFF: u64 = 3;

/// Writes the traceback and the last line, `Class: message`, or `Class` alone when the message is empty.
///
/// # Safety
///
/// `exception` points to a live exception.
pub(crate) unsafe fn report_uncaught(exception: *const ExceptionObject) {
    flush_stdout();
    // SAFETY: the caller's guarantee.
    unsafe { report_into(exception, &mut |bytes| write_stderr(bytes)) };
}

/// Writes the report of an uncaught exception through `write`. An entry identical to the one
/// before it, the same file, line and function, is shown at most three times in a row; the
/// rest are counted on one line, `[Previous line repeated 996 more times]`, as CPython does.
///
/// # Safety
///
/// `exception` points to a live exception.
unsafe fn report_into(exception: *const ExceptionObject, write: &mut dyn FnMut(&[u8])) {
    write(b"Traceback (most recent call last):\n");
    // SAFETY: the caller passes a live exception, whose entries hold static, null-terminated names.
    unsafe {
        let traceback = &(*exception).traceback;
        let mut last: Option<(&[u8], u32, &[u8])> = None;
        let mut count = 0;
        for index in (0..traceback.len()).rev() {
            let entry = traceback.get(index);
            let current = (c_bytes(entry.file), entry.line, c_bytes(entry.function));
            if last != Some(current) {
                write_repeated(count, write);
                last = Some(current);
                count = 0;
            }
            count += 1;
            if count > REPEATED_CUTOFF {
                continue;
            }
            write(b"  File \"");
            write(current.0);
            write(b"\", line ");
            write_number(u64::from(entry.line), write);
            write(b", in ");
            write(current.2);
            write(b"\n");
        }
        write_repeated(count, write);
        write((*exception).kind.name().as_bytes());
        let message = (*exception).message;
        if crate::string::length(message) > 0 {
            write(b": ");
            write_utf8(message, |bytes| write(bytes));
        }
    }
    write(b"\n");
}

/// Writes how many entries after the first three were not shown, if any.
fn write_repeated(count: u64, write: &mut dyn FnMut(&[u8])) {
    if count <= REPEATED_CUTOFF {
        return;
    }
    let hidden = count - REPEATED_CUTOFF;
    write(b"  [Previous line repeated ");
    write_number(hidden, write);
    write(if hidden > 1 {
        b" more times]\n"
    } else {
        b" more time]\n"
    });
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

fn write_number(mut value: u64, write: &mut dyn FnMut(&[u8])) {
    let mut digits = [0_u8; 20];
    let mut start = digits.len();
    loop {
        start -= 1;
        digits[start] = b'0' + u8::try_from(value % 10).unwrap_or(0);
        value /= 10;
        if value == 0 {
            break;
        }
    }
    write(&digits[start..]);
}

#[cfg(test)]
mod tests {
    use super::{report_into, report_uncaught};
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

    #[test]
    fn repeated_entries_are_counted_after_the_third_as_cpython_counts_them() {
        let cases: [(usize, &str); 3] = [
            (3, ""),
            (4, "  [Previous line repeated 1 more time]\n"),
            (7, "  [Previous line repeated 4 more times]\n"),
        ];
        for (repeats, summary) in cases {
            let exception = raise(ExceptionKind::RecursionError, "deep");
            let add = |function: &core::ffi::CStr, line: u32| {
                // SAFETY: the exception is live and the names are static.
                unsafe {
                    typy_rt_traceback_add(
                        exception,
                        function.as_ptr().cast(),
                        c"a.py".as_ptr().cast(),
                        line,
                    );
                }
            };
            add(c"down", 3);
            (0..repeats).for_each(|_| add(c"down", 4));
            add(c"<module>", 9);
            let mut bytes = Vec::new();
            // SAFETY: the exception is live, and its reference is dropped after the report.
            unsafe {
                report_into(exception, &mut |part| bytes.extend_from_slice(part));
                crate::refcount::typy_rt_decrement(exception.cast());
            }
            let shown = "  File \"a.py\", line 4, in down\n".repeat(repeats.min(3));
            let expected = format!(
                "Traceback (most recent call last):\n  File \"a.py\", line 9, in <module>\n{shown}{summary}  File \"a.py\", line 3, in down\nRecursionError: deep\n"
            );
            assert_eq!(
                String::from_utf8(bytes).unwrap(),
                expected,
                "{repeats} repeats"
            );
        }
    }
}
