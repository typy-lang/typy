//! What a text file says about itself (LIB-14): `name`, `mode`, `closed`, `fileno()`,
//! `isatty()`, `readable()`, `writable()` and `seekable()`, with CPython's errors once the
//! file is closed.

use super::{CLOSED, READABLE, TextFile, WRITABLE};
use crate::Raising;
use crate::exception::{ExceptionKind, ExceptionObject, raise};
use crate::refcount::typy_rt_increment;
use crate::string::StrObject;

/// What `typy_rt_text_state` answers, numbered as the compiler passes it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Question {
    /// `isatty()`.
    Terminal = 0,
    /// `readable()`.
    Readable = 1,
    /// `writable()`.
    Writable = 2,
    /// `seekable()`.
    Seekable = 3,
    /// `closed`.
    Closed = 4,
}

impl Question {
    fn of(word: i64) -> Self {
        match crate::word::inline_value(word) {
            0 => Self::Terminal,
            1 => Self::Readable,
            2 => Self::Writable,
            3 => Self::Seekable,
            _ => Self::Closed,
        }
    }
}

/// Returns `ValueError: I/O operation on closed file`, without the full stop of the methods
/// that read or write, as CPython's raw file words it.
fn closed_raw_error() -> *mut ExceptionObject {
    raise(ExceptionKind::ValueError, "I/O operation on closed file")
}

/// Returns `file.name`: the path it was opened with, or `<stdin>`, `<stdout>` or `<stderr>`.
///
/// # Safety
///
/// `file` points to a live text file.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_text_name(file: *const TextFile) -> *mut StrObject {
    // SAFETY: the caller's guarantee; a file `open` made owns its path.
    unsafe {
        let path = (*file).path;
        if path.is_null() {
            return crate::string::from_str((*file).name);
        }
        typy_rt_increment(path.cast());
        path
    }
}

/// Returns `file.mode`: the mode as written to `open`, or `r` or `w` for a standard file.
///
/// # Safety
///
/// `file` points to a live text file.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_text_mode(file: *const TextFile) -> *mut StrObject {
    // SAFETY: the caller's guarantee; a file `open` made owns its mode.
    unsafe {
        let mode = (*file).mode;
        if mode.is_null() {
            let readable = (*file).flags & READABLE != 0;
            return crate::string::from_str(if readable { "r" } else { "w" });
        }
        typy_rt_increment(mode.cast());
        mode
    }
}

/// Returns `file.fileno()`, the file's descriptor as an `int` word.
///
/// # Safety
///
/// `file` points to a live text file.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_text_fileno(file: *const TextFile) -> Raising<i64> {
    // SAFETY: the caller's guarantee.
    let (flags, descriptor) = unsafe { (super::flags_of(file), (*file).descriptor) };
    if flags & CLOSED != 0 {
        return Raising::error(0, closed_raw_error());
    }
    Raising::ok(crate::word::small(usize::try_from(descriptor).unwrap_or(0)))
}

/// Returns the answer to `question`, a `Question` number, about `file`. Once the file is
/// closed, `closed` is true, `readable()` and `writable()` are false for the direction the
/// file does not have, and every other question raises, as in CPython.
///
/// # Safety
///
/// `file` points to a live text file, and `question` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_text_state(file: *const TextFile, question: i64) -> Raising<bool> {
    let question = Question::of(question);
    // SAFETY: the caller's guarantee.
    let (flags, descriptor) = unsafe { (super::flags_of(file), (*file).descriptor) };
    let closed = flags & CLOSED != 0;
    let direction = match question {
        Question::Readable => Some(flags & READABLE != 0),
        Question::Writable => Some(flags & WRITABLE != 0),
        _ => None,
    };
    if question == Question::Closed {
        return Raising::ok(closed);
    }
    if closed && direction != Some(false) {
        return Raising::error(false, closed_raw_error());
    }
    Raising::ok(match question {
        Question::Terminal => crate::libc::is_terminal(descriptor),
        Question::Seekable => crate::libc::seek(descriptor, 0, crate::libc::SEEK_CUR).is_ok(),
        _ => direction.unwrap_or(false),
    })
}
