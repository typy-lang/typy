//! Prints an exception nothing caught, as CPython does, ending with its class and message (STD-05).

use super::ExceptionObject;
use crate::io::{flush_stdout, write_stderr};
use crate::string::write_utf8;

/// The number of identical entries in a row a traceback shows before it counts the rest, as
/// CPython's `TB_RECURSIVE_CUTOFF`; a deep recursion then prints a few lines, not one per frame.
const REPEATED_CUTOFF: u64 = 3;

/// Writes the chain of the exception and the last line, `Class: message`, or `Class` alone
/// when the message is empty.
///
/// # Safety
///
/// `exception` points to a live exception.
pub(crate) unsafe fn report_uncaught(exception: *const ExceptionObject) {
    flush_stdout();
    // SAFETY: the caller's guarantee.
    unsafe { report_into(exception, &mut |bytes| write_stderr(bytes)) };
}

/// The line between an exception and the one raised from it.
const CAUSE: &[u8] = b"\nThe above exception was the direct cause of the following exception:\n\n";
/// The line between an exception and one raised while it was handled.
const CONTEXT: &[u8] = b"\nDuring handling of the above exception, another exception occurred:\n\n";

/// Writes the report of an uncaught exception through `write`, as CPython does: the oldest
/// exception of its chain first, each followed by the line that links it to the next, then
/// each exception's traceback and its last line (EXC-04, STD-05).
///
/// # Safety
///
/// `exception` points to a live exception.
unsafe fn report_into(exception: *const ExceptionObject, write: &mut dyn FnMut(&[u8])) {
    // SAFETY: the caller's guarantee; every exception in the chain is live.
    unsafe {
        let length = chain_length(exception);
        for depth in (0..length).rev() {
            let link = nth(exception, depth);
            report_one(link, write);
            if depth > 0 {
                let newer = nth(exception, depth - 1);
                write(if core::ptr::eq((*newer).cause, link) {
                    CAUSE
                } else {
                    CONTEXT
                });
            }
        }
    }
}

/// Returns the exception the report shows before `exception`: its cause, or its context
/// unless `from` left it out, or null.
///
/// # Safety
///
/// `exception` points to a live exception.
unsafe fn older(exception: *const ExceptionObject) -> *const ExceptionObject {
    // SAFETY: the caller's guarantee.
    unsafe {
        if !(*exception).cause.is_null() {
            (*exception).cause
        } else if (*exception).suppress_context == 0 {
            (*exception).context
        } else {
            core::ptr::null()
        }
    }
}

/// Returns the exception `depth` links older than `exception`.
///
/// # Safety
///
/// The chain of `exception` has more than `depth` links.
unsafe fn nth(exception: *const ExceptionObject, depth: usize) -> *const ExceptionObject {
    let mut link = exception;
    for _ in 0..depth {
        // SAFETY: the caller's guarantee.
        link = unsafe { older(link) };
    }
    link
}

/// Returns the number of exceptions in the chain of `exception`, stopping before one already
/// in it, as CPython shows each exception of a chain once.
///
/// # Safety
///
/// `exception` points to a live exception.
unsafe fn chain_length(exception: *const ExceptionObject) -> usize {
    let mut length = 0;
    let mut link = exception;
    // SAFETY: the caller's guarantee; every link of the chain is live.
    unsafe {
        while !link.is_null()
            && !(0..length).any(|depth| core::ptr::eq(nth(exception, depth), link))
        {
            length += 1;
            link = older(link);
        }
    }
    length
}

/// Writes the traceback of one exception, if it has been raised, then its last line. An
/// entry identical to the one before it, the same file, line and function, is shown at most
/// three times in a row; the rest are counted on one line, `[Previous line repeated 996 more
/// times]`, as CPython does.
///
/// # Safety
///
/// `exception` points to a live exception.
unsafe fn report_one(exception: *const ExceptionObject, write: &mut dyn FnMut(&[u8])) {
    // SAFETY: the caller passes a live exception, whose entries hold static, null-terminated names.
    unsafe {
        let traceback = &(*exception).traceback;
        if traceback.len() > 0 {
            write(b"Traceback (most recent call last):\n");
        }
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
        write_last_line(exception, write);
    }
}

/// Writes `Class: text`, with the module before a class a program module defines, and the
/// text the class's `str` gives; `Class` alone when the text is empty or the class writes
/// none.
///
/// # Safety
///
/// `exception` points to a live exception.
unsafe fn write_last_line(exception: *const ExceptionObject, write: &mut dyn FnMut(&[u8])) {
    // SAFETY: the caller's guarantee.
    let class = unsafe { (*exception).class() };
    let module = class.module();
    if !module.is_empty() && module != "__main__" {
        write(module.as_bytes());
        write(b".");
    }
    write(class.name().as_bytes());
    // Every exception class writes its `str`; the report calls it through the class, so a
    // program that raises nothing links none of the code that writes arguments (NFR-01).
    let Some(str) = class.str else {
        write(b"\n");
        return;
    };
    // SAFETY: the class's function takes an exception of the class.
    let text = unsafe { str(exception.cast_mut().cast()) };
    if text.exception.is_null() {
        // SAFETY: the text is a live `str` owned here.
        unsafe {
            if crate::string::length(text.value) > 0 {
                write(b": ");
                write_utf8(text.value, |bytes| write(bytes));
            }
            crate::refcount::typy_rt_decrement(text.value.cast());
        }
    } else {
        write(b": <exception str() failed>");
        // SAFETY: the exception that writing raised is owned here.
        unsafe { crate::refcount::typy_rt_decrement(text.exception.cast()) };
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
    use crate::Raising;
    use crate::exception::classes::VALUE_ERROR;
    use crate::exception::{
        ExceptionKind, ExceptionObject, raise, typy_rt_exception_enter, typy_rt_exception_leave,
        typy_rt_exception_new, typy_rt_exception_set_cause, typy_rt_traceback_add,
    };
    use crate::header::ObjectHeader;
    use crate::string::StrObject;
    use crate::tuple::{Field, TupleType};
    use crate::type_object::TypeObject;

    /// Returns the report of `exception` and releases it.
    fn report(exception: *mut ExceptionObject) -> String {
        let mut bytes = Vec::new();
        // SAFETY: the exception is live, and its reference is dropped after the report.
        unsafe {
            report_into(exception, &mut |part| bytes.extend_from_slice(part));
            crate::refcount::typy_rt_decrement(exception.cast());
        }
        String::from_utf8(bytes).unwrap()
    }

    /// Adds one frame of `app.py` to an exception.
    fn frame(exception: *mut ExceptionObject, line: u32) {
        // SAFETY: the exception is live and the names are static.
        unsafe {
            typy_rt_traceback_add(
                exception,
                c"<module>".as_ptr().cast(),
                c"app.py".as_ptr().cast(),
                line,
            );
        }
    }

    static NO_FIELDS: [Field; 0] = [];
    static NOTHING: TupleType = TupleType {
        size: 8,
        length: 0,
        fields: NO_FIELDS.as_ptr(),
    };

    /// A class a module named `app` defines.
    static APP_ERROR: TypeObject = TypeObject {
        module: "app".as_ptr(),
        module_length: 3,
        ..TypeObject::new("AppError", &raw const VALUE_ERROR)
    };

    unsafe extern "C" fn failing_str(_: *mut ObjectHeader) -> Raising<*mut StrObject> {
        Raising::error(
            core::ptr::null_mut(),
            raise(ExceptionKind::RuntimeError, "no text"),
        )
    }

    /// A class whose `str` raises.
    static SILENT_ERROR: TypeObject = TypeObject {
        str: Some(failing_str),
        ..TypeObject::new("SilentError", &raw const VALUE_ERROR)
    };

    #[test]
    fn a_cause_is_reported_first_with_cpython_s_line_between() {
        let cause = raise(ExceptionKind::KeyError, "'k'");
        frame(cause, 2);
        let exception = raise(ExceptionKind::ValueError, "bad");
        frame(exception, 4);
        // SAFETY: both exceptions are live; the exception takes a reference to its cause.
        unsafe {
            typy_rt_exception_set_cause(exception, cause);
            crate::refcount::typy_rt_decrement(cause.cast());
        }
        assert_eq!(
            report(exception),
            "Traceback (most recent call last):\n  File \"app.py\", line 2, in <module>\n\
             KeyError: 'k'\n\nThe above exception was the direct cause of the following \
             exception:\n\nTraceback (most recent call last):\n  File \"app.py\", line 4, in \
             <module>\nValueError: bad\n"
        );
    }

    #[test]
    fn a_context_is_reported_first_unless_from_suppresses_it() {
        for suppress in [false, true] {
            let handled = raise(ExceptionKind::KeyError, "'k'");
            // SAFETY: the handled exception outlives the clause.
            let previous = unsafe { typy_rt_exception_enter(handled) };
            let exception = raise(ExceptionKind::ValueError, "bad");
            // SAFETY: the exceptions are live.
            unsafe {
                typy_rt_exception_leave(previous);
                crate::refcount::typy_rt_decrement(handled.cast());
                if suppress {
                    typy_rt_exception_set_cause(exception, core::ptr::null_mut());
                }
            }
            let expected = if suppress {
                "ValueError: bad\n"
            } else {
                "KeyError: 'k'\n\nDuring handling of the above exception, another exception \
                 occurred:\n\nValueError: bad\n"
            };
            assert_eq!(report(exception), expected, "suppress {suppress}");
        }
    }

    /// A class the main module defines.
    static MAIN_ERROR: TypeObject = TypeObject {
        module: "__main__".as_ptr(),
        module_length: 8,
        ..TypeObject::new("MainError", &raw const VALUE_ERROR)
    };

    #[test]
    fn a_class_of_the_main_module_is_named_alone() {
        // SAFETY: the class is an exception class, and the layout holds no items.
        let exception = unsafe {
            typy_rt_exception_new(
                &raw const MAIN_ERROR,
                &raw const NOTHING,
                [0_u64].as_ptr().cast(),
            )
        };
        assert_eq!(report(exception), "MainError\n");
    }

    #[test]
    fn each_exception_of_a_cycle_of_causes_is_reported_once() {
        let first = raise(ExceptionKind::KeyError, "'a'");
        let second = raise(ExceptionKind::ValueError, "b");
        // SAFETY: both exceptions are live; each takes a reference to the other as its cause.
        unsafe {
            typy_rt_exception_set_cause(second, first);
            typy_rt_exception_set_cause(first, second);
        }
        let mut bytes = Vec::new();
        // SAFETY: the exceptions are live; the cycle is broken before they are released.
        unsafe {
            report_into(second, &mut |part| bytes.extend_from_slice(part));
            let cause = core::mem::replace(&mut (*first).cause, core::ptr::null_mut());
            crate::refcount::typy_rt_decrement(cause.cast());
            crate::refcount::typy_rt_decrement(first.cast());
            crate::refcount::typy_rt_decrement(second.cast());
        }
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "KeyError: 'a'\n\nThe above exception was the direct cause of the following \
             exception:\n\nValueError: b\n"
        );
    }

    #[test]
    fn a_class_of_another_module_is_named_with_its_module() {
        // SAFETY: the class is an exception class, and the layout holds no items.
        let exception = unsafe {
            typy_rt_exception_new(
                &raw const APP_ERROR,
                &raw const NOTHING,
                [0_u64].as_ptr().cast(),
            )
        };
        assert_eq!(report(exception), "app.AppError\n");
    }

    #[test]
    fn a_str_that_raises_is_reported_as_cpython_reports_it() {
        // SAFETY: the class is an exception class, and the layout holds no items.
        let exception = unsafe {
            typy_rt_exception_new(
                &raw const SILENT_ERROR,
                &raw const NOTHING,
                [0_u64].as_ptr().cast(),
            )
        };
        assert_eq!(report(exception), "SilentError: <exception str() failed>\n");
    }

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
