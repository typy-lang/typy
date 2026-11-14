//! `seek`, `tell` and `truncate` of a text file (LIB-14), as CPython's `TextIOWrapper` gives
//! them for UTF-8: a position is the byte offset in the file, which is the cookie CPython's
//! `tell` returns when no decoding is pending.

use super::{CLOSED, TextFile, WRITABLE, typy_rt_text_flush, with_reader};
use crate::Raising;
use crate::exception::{ExceptionKind, ExceptionObject, raise, raise_class};
use crate::int::value::Integer;
use crate::libc::{EINVAL, SEEK_CUR, SEEK_END, SEEK_SET};
use crate::message::MessageBuffer;

/// Returns `io.UnsupportedOperation` with `message`.
fn unsupported(message: &str) -> *mut ExceptionObject {
    raise_class(
        &crate::exception::classes::UNSUPPORTED_OPERATION,
        crate::string::from_str(message),
    )
}

/// Returns the `OSError` of `errno`, which names no file.
fn os_error(errno: i32) -> *mut ExceptionObject {
    // SAFETY: a null file name names no file.
    unsafe { super::os_error::os_error(errno, core::ptr::null()) }
}

/// Returns the value of an `int` word, or `None` when it does not fit in an `i128`.
///
/// # Safety
///
/// `word` is an `int` word whose large integer is live.
unsafe fn value_of(word: i64) -> Option<i128> {
    if crate::word::is_inline(word) {
        return Some(i128::from(crate::word::inline_value(word)));
    }
    // SAFETY: the caller's guarantee.
    unsafe { Integer::from_word(word) }.to_i128()
}

/// Returns the `int` word of a position.
fn word_of(position: i64) -> i64 {
    Integer::from_i128(i128::from(position)).into_word()
}

/// Checks that the file is open and its descriptor can move, as `seek` and `tell` check.
///
/// # Safety
///
/// `file` points to a live text file.
unsafe fn seekable(file: *mut TextFile) -> Result<i32, *mut ExceptionObject> {
    // SAFETY: the caller's guarantee.
    let (flags, descriptor) = unsafe { ((*file).flags, (*file).descriptor) };
    if flags & CLOSED != 0 {
        return Err(super::disk::closed_error());
    }
    if crate::libc::seek(descriptor, 0, SEEK_CUR).is_err() {
        return Err(unsupported("underlying stream is not seekable"));
    }
    Ok(descriptor)
}

/// Writes out what the file holds, then returns where its descriptor is, less what its reader
/// has read and not given out.
///
/// # Safety
///
/// `file` points to a live text file that is open.
unsafe fn position(file: *mut TextFile, descriptor: i32) -> Result<i64, *mut ExceptionObject> {
    // SAFETY: the caller's guarantee.
    let error = unsafe { typy_rt_text_flush(file) };
    if !error.is_null() {
        return Err(error);
    }
    let at = crate::libc::seek(descriptor, 0, SEEK_CUR).map_err(os_error)?;
    // SAFETY: the caller's guarantee.
    let unread = unsafe { with_reader(file, |reader| reader.map_or(0, |reader| reader.unread())) };
    Ok(at - i64::try_from(unread).unwrap_or(0))
}

/// Returns `tell()`, or the error `tell` raises.
///
/// # Safety
///
/// `file` points to a live text file.
unsafe fn tell(file: *mut TextFile) -> Result<i64, *mut ExceptionObject> {
    // SAFETY: the caller's guarantee.
    unsafe {
        let descriptor = seekable(file)?;
        if with_reader(file, |reader| {
            reader.is_some_and(|reader| reader.iterating())
        }) {
            return Err(raise_class(
                &crate::exception::classes::OS_ERROR,
                crate::string::from_str("telling position disabled by next() call"),
            ));
        }
        position(file, descriptor)
    }
}

/// Returns `file.tell()`: the file's position as an `int` word.
///
/// # Safety
///
/// `file` points to a live text file.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_text_tell(file: *mut TextFile) -> Raising<i64> {
    // SAFETY: the caller's guarantee.
    match unsafe { tell(file) } {
        Ok(at) => Raising::ok(word_of(at)),
        Err(error) => Raising::error(0, error),
    }
}

/// Returns `ValueError` with `prefix` and the `int` word's value, as `seek` words it.
///
/// # Safety
///
/// `word` is an `int` word whose large integer is live.
unsafe fn value_error(prefix: &str, word: i64, suffix: &str) -> *mut ExceptionObject {
    // SAFETY: the caller's guarantee.
    let shown = unsafe { crate::int::typy_rt_int_to_str(word) };
    if !shown.exception.is_null() {
        return shown.exception;
    }
    let text = shown.value;
    let mut message = MessageBuffer::new();
    message.push_str(prefix);
    // SAFETY: the text is a live string, released once it is copied.
    unsafe {
        message.push_text(text);
        crate::refcount::typy_rt_decrement(text.cast());
    }
    message.push_str(suffix);
    message.raise(ExceptionKind::ValueError)
}

/// Moves the file to `offset` from `whence`, after writing out what it holds and dropping
/// what its reader holds; returns the new position.
///
/// # Safety
///
/// `file` points to a live text file that is open.
unsafe fn move_to(
    file: *mut TextFile,
    descriptor: i32,
    offset: i64,
    whence: i32,
) -> Result<i64, *mut ExceptionObject> {
    // SAFETY: the caller's guarantee.
    unsafe {
        let error = typy_rt_text_flush(file);
        if !error.is_null() {
            return Err(error);
        }
        with_reader(file, |reader| {
            if let Some(reader) = reader {
                reader.reset();
            }
        });
    }
    crate::libc::seek(descriptor, offset, whence).map_err(os_error)
}

/// Returns `file.seek(cookie, whence)` with CPython's checks: `whence` a C `int` of 0, 1 or
/// 2, a relative move only by zero, and no negative position. A position from 2^63 on is
/// `EINVAL`, as the descriptor refuses it; CPython reads the bits from 2^64 on as decoder
/// state, which its documentation leaves undefined for a cookie `tell` did not return.
///
/// # Safety
///
/// `file` points to a live text file, and `cookie` and `whence` are `int` words whose large
/// integers are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_text_seek(
    file: *mut TextFile,
    cookie: i64,
    whence: i64,
) -> Raising<i64> {
    // SAFETY: the caller's guarantee.
    match unsafe { seek(file, cookie, whence) } {
        Ok(at) => Raising::ok(word_of(at)),
        Err(error) => Raising::error(0, error),
    }
}

/// Returns the position `seek` moves to, or its error.
///
/// # Safety
///
/// As for `typy_rt_text_seek`.
unsafe fn seek(file: *mut TextFile, cookie: i64, whence: i64) -> Result<i64, *mut ExceptionObject> {
    // SAFETY: the caller's guarantee.
    let (offset, whence_value) = unsafe { (value_of(cookie), value_of(whence)) };
    let Some(whence_value) = whence_value.and_then(|value| i32::try_from(value).ok()) else {
        return Err(raise(
            ExceptionKind::OverflowError,
            "Python int too large to convert to C int",
        ));
    };
    // SAFETY: the caller's guarantee.
    let descriptor = unsafe { seekable(file)? };
    let zero = offset == Some(0);
    let offset = match whence_value {
        SEEK_SET => offset,
        // SAFETY: the caller's guarantee.
        SEEK_CUR if zero => Some(i128::from(unsafe { tell(file)? })),
        SEEK_CUR => return Err(unsupported("can't do nonzero cur-relative seeks")),
        // SAFETY: the file is open.
        SEEK_END if zero => return unsafe { move_to(file, descriptor, 0, SEEK_END) },
        SEEK_END => return Err(unsupported("can't do nonzero end-relative seeks")),
        _ => {
            // SAFETY: the caller's guarantee.
            return Err(unsafe {
                value_error("invalid whence (", whence, ", should be 0, 1 or 2)")
            });
        }
    };
    // A value past `i128` has the sign of its word.
    // SAFETY: the caller's guarantee.
    let negative = offset.map_or_else(|| unsafe { crate::int::is_negative(cookie) }, |at| at < 0);
    if negative {
        // SAFETY: the caller's guarantee.
        return Err(unsafe { value_error("negative seek position ", cookie, "") });
    }
    let Some(offset) = offset.and_then(|at| i64::try_from(at).ok()) else {
        return Err(os_error(EINVAL));
    };
    // SAFETY: the file is open.
    unsafe { move_to(file, descriptor, offset, SEEK_SET) }
}

/// Returns `file.truncate(size)`, or `file.truncate()` when `given` is false: the file cut or
/// extended to `size` bytes, or to its position, which does not move; `size` is a C `long`.
///
/// # Safety
///
/// `file` points to a live text file, `size` is an `int` word whose large integer is live
/// when `given`, and `given` is the `int` word of 0 or 1.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_text_truncate(
    file: *mut TextFile,
    size: i64,
    given: i64,
) -> Raising<i64> {
    let given = crate::word::inline_value(given) != 0;
    // SAFETY: the caller's guarantee.
    match unsafe { truncate(file, given.then_some(size)) } {
        Ok(at) => Raising::ok(word_of(at)),
        Err(error) => Raising::error(0, error),
    }
}

/// Returns the size `truncate` gives the file, or its error.
///
/// # Safety
///
/// As for `typy_rt_text_truncate`.
unsafe fn truncate(file: *mut TextFile, size: Option<i64>) -> Result<i64, *mut ExceptionObject> {
    // SAFETY: the caller's guarantee.
    let (flags, descriptor) = unsafe { ((*file).flags, (*file).descriptor) };
    if flags & CLOSED != 0 {
        return Err(super::disk::closed_error());
    }
    // SAFETY: the caller's guarantee.
    let error = unsafe { typy_rt_text_flush(file) };
    if !error.is_null() {
        return Err(error);
    }
    if flags & WRITABLE == 0 {
        return Err(unsupported("truncate"));
    }
    let size = match size {
        // SAFETY: the caller's guarantee.
        Some(word) => unsafe { value_of(word) }
            .and_then(|value| i64::try_from(value).ok())
            .ok_or_else(|| {
                raise(
                    ExceptionKind::OverflowError,
                    "Python int too large to convert to C long",
                )
            })?,
        None => crate::libc::seek(descriptor, 0, SEEK_CUR).map_err(os_error)?,
    };
    crate::libc::truncate(descriptor, size).map_err(os_error)?;
    Ok(size)
}
