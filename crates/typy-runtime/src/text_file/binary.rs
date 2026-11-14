//! Binary files (LIB-14): `BufferedReader`, `BufferedWriter` and `BufferedRandom`, which read
//! through the file's `Reader` and write through its `Writer`, and `FileIO`, which calls the
//! descriptor for each read and write. Each refuses what CPython's class refuses, with its
//! message: a buffered file names the operation (`read of closed file`), a `FileIO` does not.

use super::{BINARY, CLOSED, RAW, READABLE, TextFile, WRITABLE, flags_of};
use crate::Raising;
use crate::byte_buffer::ByteBuffer;
use crate::exception::{ExceptionKind, ExceptionObject, raise, raise_class};
use crate::list::ListObject;
use crate::string::StrObject;

/// Returns `io.UnsupportedOperation` with `message`.
fn unsupported(message: &str) -> *mut ExceptionObject {
    raise_class(
        &crate::exception::classes::UNSUPPORTED_OPERATION,
        crate::string::from_str(message),
    )
}

/// Returns `ValueError` with `message`.
fn value_error(message: &str) -> *mut ExceptionObject {
    raise(ExceptionKind::ValueError, message)
}

/// Returns the `OSError` of `errno`, which names no file.
fn os_error(errno: i32) -> *mut ExceptionObject {
    // SAFETY: a null file name names no file.
    unsafe { super::os_error::os_error(errno, core::ptr::null()) }
}

/// Returns whether a file is a `FileIO`, from its flags.
const fn is_raw(flags: u32) -> bool {
    flags & RAW != 0
}

/// Returns the error a read raises before reading, if any: a buffered file that does not
/// read refuses first and is then checked open, as `closed` names its operation; a `FileIO`
/// is checked open first.
fn read_refused(flags: u32, operation: &str) -> Option<*mut ExceptionObject> {
    let closed = flags & CLOSED != 0;
    let readable = flags & READABLE != 0;
    if is_raw(flags) {
        return if closed {
            Some(value_error("I/O operation on closed file"))
        } else if !readable {
            Some(unsupported("File not open for reading"))
        } else {
            None
        };
    }
    if !readable {
        return Some(unsupported("read"));
    }
    closed.then(|| value_error(operation))
}

/// Returns the error a write raises before writing, if any, as `read_refused` orders them.
fn write_refused(flags: u32) -> Option<*mut ExceptionObject> {
    let closed = flags & CLOSED != 0;
    let writable = flags & WRITABLE != 0;
    if is_raw(flags) {
        return if closed {
            Some(value_error("I/O operation on closed file"))
        } else if !writable {
            Some(unsupported("File not open for writing"))
        } else {
            None
        };
    }
    if !writable {
        return Some(unsupported("write"));
    }
    closed.then(|| value_error("write to closed file"))
}

/// Returns the size a read takes: `None` for all of the rest, from a negative `int` word.
fn size_of(word: i64) -> Option<usize> {
    usize::try_from(crate::word::inline_value(word)).ok()
}

/// Reads from a `FileIO`: `limit` bytes at most, or to the end, ending after a `\n` when
/// `line`. A line is read a byte at a time, as CPython's `RawIOBase.readline` reads it, so
/// the descriptor stops after the line.
fn raw_read(
    descriptor: i32,
    limit: Option<usize>,
    line: bool,
) -> Result<*mut StrObject, *mut ExceptionObject> {
    let mut out = ByteBuffer::new();
    let mut chunk = [0_u8; 8192];
    loop {
        let left = limit.map_or(chunk.len(), |limit| limit - out.bytes().len());
        let step = if line { 1 } else { left.min(chunk.len()) };
        if step == 0 {
            break;
        }
        match crate::libc::read_retrying(descriptor, &mut chunk[..step]) {
            Ok(0) => break,
            Ok(count) => out.extend(&chunk[..count]),
            Err(errno) => {
                out.discard();
                return Err(os_error(errno));
            }
        }
        if line && out.bytes().last() == Some(&b'\n') {
            break;
        }
    }
    let made = crate::bytes::from_slice(out.bytes());
    out.discard();
    Ok(made)
}

/// Reads from a binary file that is open and readable, after writing out what it holds.
///
/// # Safety
///
/// `file` points to a live binary file `open` made.
unsafe fn read_open(
    file: *mut TextFile,
    limit: Option<usize>,
    line: bool,
) -> Result<*mut StrObject, *mut ExceptionObject> {
    // SAFETY: the caller's guarantee.
    unsafe {
        let descriptor = (*file).descriptor;
        if is_raw((*file).flags) {
            return raw_read(descriptor, limit, line);
        }
        let flushed = super::disk::flush(&mut *file);
        if !flushed.is_null() {
            return Err(flushed);
        }
        let mut source = |buffer: &mut [u8]| crate::libc::read_retrying(descriptor, buffer);
        (*(*file).reader).read_bytes(limit, line, &mut source)
    }
}

/// Returns `file.read(size)` of a binary file: `size` bytes at most, or the rest when `size`
/// is negative or `None`.
///
/// # Safety
///
/// `file` points to a live binary file, and `size` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_binary_read(
    file: *mut TextFile,
    size: i64,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller's guarantee.
    if let Some(error) = read_refused(unsafe { flags_of(file) }, "read of closed file") {
        return Raising::error(core::ptr::null_mut(), error);
    }
    // SAFETY: the caller's guarantee.
    unsafe { read_open(file, size_of(size), false) }.into()
}

/// Returns `file.readline(size)` of a binary file: the next line with its `\n`, `size` bytes
/// at most when `size` is not negative.
///
/// # Safety
///
/// `file` points to a live binary file, and `size` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_binary_readline(
    file: *mut TextFile,
    size: i64,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller's guarantee.
    if let Some(error) = read_refused(unsafe { flags_of(file) }, "readline of closed file") {
        return Raising::error(core::ptr::null_mut(), error);
    }
    // SAFETY: the caller's guarantee.
    unsafe { read_open(file, size_of(size), true) }.into()
}

/// Returns the next line of a binary file for a `for` loop or `readlines`, or `None` at its
/// end; a closed file raises as CPython's iterator does.
///
/// # Safety
///
/// `file` points to a live binary file.
pub(crate) unsafe fn next_line(
    file: *mut TextFile,
) -> Result<Option<*mut StrObject>, *mut ExceptionObject> {
    // SAFETY: the caller's guarantee.
    let flags = unsafe { flags_of(file) };
    if flags & CLOSED != 0 {
        return Err(super::disk::closed_error());
    }
    if let Some(error) = read_refused(flags, "readline of closed file") {
        return Err(error);
    }
    // SAFETY: the caller's guarantee.
    let line = unsafe { read_open(file, None, true)? };
    // SAFETY: the line is a live `bytes`, released when it is empty.
    unsafe {
        if crate::string::length(line) == 0 {
            crate::refcount::typy_rt_decrement(line.cast());
            return Ok(None);
        }
    }
    Ok(Some(line))
}

/// Returns `file.readlines()` of a binary file: the list of the lines left.
///
/// # Safety
///
/// `file` points to a live binary file.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_binary_readlines(file: *mut TextFile) -> Raising<*mut ListObject> {
    let list = crate::list::new_list(crate::list::ItemKind::Object, 0);
    loop {
        // SAFETY: the caller's guarantee.
        match unsafe { next_line(file) } {
            Ok(Some(line)) => {
                // SAFETY: the list is live and takes a reference of its own to the line.
                unsafe {
                    crate::list::typy_rt_list_append(list, line as u64);
                    crate::refcount::typy_rt_decrement(line.cast());
                }
            }
            Ok(None) => return Raising::ok(list),
            Err(error) => {
                // SAFETY: the list is new and owned here.
                unsafe { crate::refcount::typy_rt_decrement(list.cast()) };
                return Raising::error(core::ptr::null_mut(), error);
            }
        }
    }
}

/// Returns `file.write(data)` of a binary file: the number of bytes, as an `int` word. A
/// buffered file holds them; one that has read moves its descriptor back to where the reading
/// stopped first, as `BufferedRandom` writes at the position `tell` gives. A `FileIO` writes
/// them at once.
///
/// # Safety
///
/// `file` points to a live binary file, and `data` to a live `bytes`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_binary_write(
    file: *mut TextFile,
    data: *const StrObject,
) -> Raising<i64> {
    // SAFETY: the caller's guarantee.
    unsafe {
        if let Some(error) = write_refused(flags_of(file)) {
            return Raising::error(0, error);
        }
        let count = crate::string::length(data);
        let descriptor = (*file).descriptor;
        let written = if is_raw((*file).flags) {
            let mut bytes = ByteBuffer::new();
            for index in 0..count {
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "an item of a bytes is below 256"
                )]
                bytes.extend(&[crate::string::code_point(data, index) as u8]);
            }
            let result = crate::libc::write_retrying(descriptor, bytes.bytes());
            bytes.discard();
            result.map_err(os_error)
        } else {
            let reader = (*file).reader;
            if !reader.is_null() && (*reader).unread() > 0 {
                let back = -i64::try_from((*reader).unread()).unwrap_or(0);
                if let Err(errno) = crate::libc::seek(descriptor, back, crate::libc::SEEK_CUR) {
                    return Raising::error(0, os_error(errno));
                }
                (*reader).reset();
            }
            let mut sink = |bytes: &[u8]| crate::libc::write_retrying(descriptor, bytes);
            (*(*file).writer).add(data, &mut sink)
        };
        match written {
            Ok(()) => Raising::ok(crate::word::small(count)),
            Err(error) => Raising::error(0, error),
        }
    }
}

/// Returns whether `file` is a binary file.
///
/// # Safety
///
/// `file` points to a live text file.
pub(crate) unsafe fn is_binary(file: *const TextFile) -> bool {
    // SAFETY: the caller's guarantee.
    !file.is_null() && unsafe { (*file).flags } & BINARY != 0
}

/// Returns the value of an `int` word that fits in an `i64`, or `None`.
///
/// # Safety
///
/// `word` is an `int` word whose large integer is live.
unsafe fn i64_of(word: i64) -> Option<i64> {
    // SAFETY: the caller's guarantee.
    unsafe { super::position::value_of(word) }.and_then(|value| i64::try_from(value).ok())
}

/// Returns `tell()` of a binary file: the descriptor's position, less what the reader holds
/// and with what the writer holds, which a `BufferedWriter` counts without writing it out.
///
/// # Safety
///
/// `file` points to a live binary file.
pub(crate) unsafe fn tell(file: *mut TextFile) -> Result<i64, *mut ExceptionObject> {
    // SAFETY: the caller's guarantee; an open file's reader and writer are live or null.
    unsafe {
        if flags_of(file) & CLOSED != 0 {
            return Err(value_error("I/O operation on closed file"));
        }
        let at =
            crate::libc::seek((*file).descriptor, 0, crate::libc::SEEK_CUR).map_err(os_error)?;
        let (reader, writer) = ((*file).reader, (*file).writer);
        let unread = if reader.is_null() {
            0
        } else {
            (*reader).unread()
        };
        let pending = if writer.is_null() {
            0
        } else {
            (*writer).pending()
        };
        Ok(at - i64::try_from(unread).unwrap_or(0) + i64::try_from(pending).unwrap_or(0))
    }
}

/// Returns `seek(offset, whence)` of a binary file, with the checks of CPython's class: a
/// buffered file allows `whence` 0 to 4 (4 and 3 are Linux's `SEEK_HOLE` and `SEEK_DATA`)
/// before it is checked open, and an offset past an `i64` is a `ValueError`; a `FileIO`
/// gives `whence` to the descriptor and refuses such an offset with `OverflowError`.
///
/// # Safety
///
/// `file` points to a live binary file, and `offset` and `whence` are `int` words whose
/// large integers are live.
pub(crate) unsafe fn seek(
    file: *mut TextFile,
    offset: i64,
    whence: i64,
) -> Result<i64, *mut ExceptionObject> {
    // SAFETY: the caller's guarantee.
    let whence_value = unsafe { super::position::value_of(whence) };
    let Some(whence) = whence_value.and_then(|value| i32::try_from(value).ok()) else {
        return Err(raise(
            ExceptionKind::OverflowError,
            "Python int too large to convert to C int",
        ));
    };
    // SAFETY: the caller's guarantee.
    let flags = unsafe { flags_of(file) };
    // SAFETY: the caller's guarantee.
    let descriptor = unsafe { (*file).descriptor };
    if is_raw(flags) {
        if flags & CLOSED != 0 {
            return Err(value_error("I/O operation on closed file"));
        }
        // SAFETY: the caller's guarantee.
        let Some(offset) = (unsafe { i64_of(offset) }) else {
            return Err(raise(
                ExceptionKind::OverflowError,
                "Python int too large to convert to C long",
            ));
        };
        return crate::libc::seek(descriptor, offset, whence).map_err(os_error);
    }
    if !(0..=4).contains(&whence) {
        let mut message = crate::message::MessageBuffer::new();
        message.push_str("whence value ");
        message.push_number(i64::from(whence));
        message.push_str(" unsupported");
        return Err(message.raise(ExceptionKind::ValueError));
    }
    if flags & CLOSED != 0 {
        return Err(value_error("seek of closed file"));
    }
    // SAFETY: the caller's guarantee.
    let Some(mut offset) = (unsafe { i64_of(offset) }) else {
        return Err(value_error("cannot fit 'int' into an offset-sized integer"));
    };
    // SAFETY: the caller's guarantee; the file is open.
    unsafe {
        let mut whence = whence;
        if whence == crate::libc::SEEK_CUR {
            // A move from the position `tell` gives, which counts what the buffers hold.
            offset = offset.saturating_add(tell(file)?);
            whence = crate::libc::SEEK_SET;
        }
        let flushed = super::disk::flush(&mut *file);
        if !flushed.is_null() {
            return Err(flushed);
        }
        if !(*file).reader.is_null() {
            (*(*file).reader).reset();
        }
        crate::libc::seek(descriptor, offset, whence).map_err(os_error)
    }
}

/// Returns `truncate(size)` of a binary file, or `truncate()` for `None`: the file cut or
/// extended to `size` bytes, or to its descriptor's position, which does not move.
///
/// # Safety
///
/// `file` points to a live binary file, and `size` is an `int` word whose large integer is
/// live when it is given.
pub(crate) unsafe fn truncate(
    file: *mut TextFile,
    size: Option<i64>,
) -> Result<i64, *mut ExceptionObject> {
    // SAFETY: the caller's guarantee.
    let flags = unsafe { flags_of(file) };
    // SAFETY: the caller's guarantee.
    let descriptor = unsafe { (*file).descriptor };
    if is_raw(flags) {
        if flags & CLOSED != 0 {
            return Err(value_error("I/O operation on closed file"));
        }
        if flags & WRITABLE == 0 {
            return Err(unsupported("File not open for writing"));
        }
    } else {
        if flags & CLOSED != 0 {
            return Err(value_error("truncate of closed file"));
        }
        if flags & WRITABLE == 0 {
            return Err(unsupported("truncate"));
        }
        // SAFETY: the caller's guarantee; the file is open.
        let flushed = unsafe { super::disk::flush(&mut *file) };
        if !flushed.is_null() {
            return Err(flushed);
        }
    }
    let size = match size {
        // SAFETY: the caller's guarantee.
        Some(word) => unsafe { i64_of(word) }.ok_or_else(|| {
            raise(
                ExceptionKind::OverflowError,
                "Python int too large to convert to C long",
            )
        })?,
        None => crate::libc::seek(descriptor, 0, crate::libc::SEEK_CUR).map_err(os_error)?,
    };
    crate::libc::truncate(descriptor, size).map_err(os_error)?;
    Ok(size)
}
