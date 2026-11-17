//! The `sys` module (LIB-04): the program's arguments, `input`, and the exit status an
//! uncaught `SystemExit` gives.

use crate::Raising;
use crate::cell::ProgramCell;
use crate::exception::{ExceptionKind, ExceptionObject, raise};
use crate::list::ListObject;
use crate::string::{StrBuilder, StrObject};

/// The program's arguments as the C library gave them to `main`, and the list `sys.argv`
/// made of them once read.
struct Arguments {
    count: usize,
    values: *const *const u8,
    list: *mut ListObject,
}

static ARGUMENTS: ProgramCell<Arguments> = ProgramCell::new(Arguments {
    count: 0,
    values: core::ptr::null(),
    list: core::ptr::null_mut(),
});

/// Keeps the arguments `main` receives, for `sys.argv`.
pub(crate) fn keep_arguments(count: i32, values: *const *const u8) {
    ARGUMENTS.with(|arguments| {
        arguments.count = usize::try_from(count).unwrap_or(0);
        arguments.values = values;
    });
}

/// Returns `bytes` decoded as UTF-8, each byte that is not part of a character as U+DC80 to
/// U+DCFF, as CPython decodes its arguments and standard input in UTF-8 mode.
pub(crate) fn decode(bytes: &[u8]) -> *mut StrObject {
    let mut text = StrBuilder::with_capacity(bytes.len());
    let mut at = 0;
    let byte = |index: usize| bytes.get(index).map(|byte| u32::from(*byte));
    while at < bytes.len() {
        if let Ok((count, point)) = crate::codec::utf8_sequence(byte, at) {
            text.push(point);
            at += count;
        } else {
            text.push(0xdc00 + u32::from(bytes[at]));
            at += 1;
        }
    }
    text.finish()
}

/// Returns `sys.argv`, a list of the program's arguments made the first time it is read and
/// kept for the whole run, so a change to it shows at the next read.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_sys_argv() -> *mut ListObject {
    ARGUMENTS.with(|arguments| {
        if arguments.list.is_null() {
            let list = crate::list::new_list(crate::list::ItemKind::Object, arguments.count);
            for index in 0..arguments.count {
                // SAFETY: the C library passes `count` arguments, each a string ended by a
                // zero byte.
                let bytes = unsafe {
                    let start = *arguments.values.add(index);
                    let mut end = 0;
                    while *start.add(end) != 0 {
                        end += 1;
                    }
                    core::slice::from_raw_parts(start, end)
                };
                let text = decode(bytes);
                // SAFETY: the list is live with room for the item, and takes a reference of
                // its own, so this one is dropped.
                unsafe {
                    crate::list::push_retained(list, text as u64);
                    crate::refcount::typy_rt_decrement(text.cast());
                }
            }
            arguments.list = list;
        }
        arguments.list
    })
}

/// The limit `sys.getrecursionlimit` gives, CPython's default until the program sets one.
static RECURSION_LIMIT: ProgramCell<i64> = ProgramCell::new(1000);

/// Returns `sys.getrecursionlimit()`, an `int` word.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_sys_getrecursionlimit() -> i64 {
    RECURSION_LIMIT.with(|limit| crate::word::inline(*limit).unwrap_or(1))
}

/// Sets the limit `sys.setrecursionlimit(limit)` gives, after CPython's checks: a C `int`,
/// then at least 1. TyPy's recursion stops at the stack's own limit (FUN-02), so the number
/// is kept for `getrecursionlimit` only.
///
/// # Safety
///
/// `limit` is an `int` word whose large integer is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_sys_setrecursionlimit(limit: i64) -> *mut ExceptionObject {
    let value = if crate::word::is_inline(limit) {
        Some(i128::from(crate::word::inline_value(limit)))
    } else {
        // SAFETY: the caller's guarantee.
        unsafe { crate::int::value::Integer::from_word(limit) }.to_i128()
    };
    let Some(value) = value.and_then(|value| i32::try_from(value).ok()) else {
        return raise(
            ExceptionKind::OverflowError,
            "Python int too large to convert to C int",
        );
    };
    if value < 1 {
        return raise(
            ExceptionKind::ValueError,
            "recursion limit must be greater or equal than 1",
        );
    }
    RECURSION_LIMIT.with(|kept| *kept = i64::from(value));
    core::ptr::null_mut()
}

/// Returns `input(prompt)`: the prompt written to standard output, which is then flushed,
/// and the next line of standard input without its `\n`, or `EOFError` at its end. It reads
/// through `sys.stdin`, so the two share what has been read.
///
/// # Safety
///
/// `prompt` is null or points to a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_input(prompt: *const StrObject) -> Raising<*mut StrObject> {
    // CPython writes the prompt to `sys.stdout` and flushes it, which raises once it is closed.
    if crate::io::standard_closed(crate::libc::STDOUT) {
        return Raising::error(core::ptr::null_mut(), crate::text_file::closed_error());
    }
    if !prompt.is_null() {
        // SAFETY: the caller's guarantee.
        unsafe { crate::io::typy_rt_write_stdout(prompt) };
    }
    crate::io::flush_stdout();
    match crate::text_file::input_line() {
        Ok(Some(line)) => Raising::ok(line),
        Ok(None) => Raising::error(
            core::ptr::null_mut(),
            raise(ExceptionKind::EOFError, "EOF when reading a line"),
        ),
        Err(error) => Raising::error(core::ptr::null_mut(), error),
    }
}

/// Returns the exit status an uncaught exception gives, and whether it is reported: a
/// `SystemExit`, or an instance of a subclass, without an argument or with `None` ends with
/// 0, with an `int` with that number modulo 256 (255 for one outside `i128`, as CPython 3.15
/// gives), and with anything else, which is written to standard error, with 1; any other
/// exception is reported and ends with 1.
///
/// # Safety
///
/// `exception` points to a live exception.
pub(crate) unsafe fn exit_status(exception: *mut ExceptionObject) -> (i32, bool) {
    // SAFETY: the caller's guarantee.
    let object = unsafe { &*exception };
    if !is_system_exit(object.class()) {
        return (1, true);
    }
    match crate::exception::exit_code(object) {
        crate::exception::ExitCode::None => (0, false),
        crate::exception::ExitCode::Int(word) => {
            let low = if crate::word::is_inline(word) {
                i128::from(crate::word::inline_value(word))
            } else {
                // SAFETY: the word is the exception's own, live while it is.
                unsafe { crate::int::value::Integer::from_word(word) }
                    .to_i128()
                    .unwrap_or(-1)
            };
            #[allow(
                clippy::cast_possible_truncation,
                reason = "the status is the low byte, as the operating system keeps it"
            )]
            let status = (low & 0xff) as i32;
            (status, false)
        }
        crate::exception::ExitCode::Other => {
            if let Some(str) = object.class().str {
                // SAFETY: the exception is live; its class's `str` gives the `str` of its
                // argument.
                unsafe {
                    let text = str(exception.cast());
                    if text.exception.is_null() {
                        // CPython writes only the line ending when `sys.stderr` is closed.
                        if !crate::io::standard_closed(crate::libc::STDERR) {
                            crate::string::write_utf8(
                                text.value,
                                crate::string::Surrogates::Backslash,
                                crate::io::write_stderr,
                            );
                        }
                        crate::io::write_stderr(b"\n");
                        crate::refcount::typy_rt_decrement(text.value.cast());
                    } else {
                        crate::refcount::typy_rt_decrement(text.exception.cast());
                    }
                }
            }
            (1, false)
        }
    }
}

/// Returns whether `class` is `SystemExit` or derives from it. `SystemExit` is found by its
/// name among the built-in classes, whose module is empty, and not by its address, so `main`
/// does not link the exception classes and their text into a program that never raises one.
pub(crate) fn is_system_exit(class: &crate::type_object::TypeObject) -> bool {
    let mut current: *const crate::type_object::TypeObject = class;
    while !current.is_null() {
        // SAFETY: a base is null or a live, immortal type object.
        let ancestor = unsafe { &*current };
        if ancestor.name() == "SystemExit" && ancestor.module().is_empty() {
            return true;
        }
        current = ancestor.base;
    }
    false
}

mod version_info;

pub(crate) use version_info::VERSION_INFO_TYPE;

#[cfg(test)]
mod tests;
