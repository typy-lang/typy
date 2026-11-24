//! Tests of the report of an exception nothing caught.

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
