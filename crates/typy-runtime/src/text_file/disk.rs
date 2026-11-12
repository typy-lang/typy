//! The files `open` makes (ADR-0019): opening a path in a text mode, reading and writing
//! through the file's buffers, closing, and the list of open files the program closes at its
//! end.

use super::reader::Reader;
use super::writer::Writer;
use super::{CLOSED, FUNCTIONS, FileFunctions, READABLE, TextFile, WRITABLE};
use crate::Raising;
use crate::cell::ProgramCell;
use crate::exception::ExceptionObject;
use crate::header::{ObjectHeader, TEXT_FILE_TYPE_ID};
use crate::refcount::{typy_rt_decrement, typy_rt_increment};
use crate::string::{StrObject, code_point, length};

/// The files open now, most recent first.
static OPEN: ProgramCell<*mut TextFile> = ProgramCell::new(core::ptr::null_mut());

static DISK_FUNCTIONS: FileFunctions = FileFunctions { destroy, close_all };

/// How a file is opened, numbered as the compiler passes it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Access {
    /// `r`: an existing file, read.
    Read = 0,
    /// `w`: a file emptied or created, written.
    Write = 1,
    /// `a`: a file created if missing, written at its end.
    Append = 2,
    /// `x`: a new file, written; an existing one is an error.
    Create = 3,
}

impl Access {
    fn of(word: i64) -> Self {
        match crate::word::inline_value(word) {
            1 => Self::Write,
            2 => Self::Append,
            3 => Self::Create,
            _ => Self::Read,
        }
    }

    /// Returns the flags `open` takes for the access.
    const fn flags(self) -> i32 {
        use crate::libc::{O_APPEND, O_CLOEXEC, O_CREAT, O_EXCL, O_TRUNC, O_WRONLY};
        O_CLOEXEC
            | match self {
                Self::Read => 0,
                Self::Write => O_WRONLY | O_CREAT | O_TRUNC,
                Self::Append => O_WRONLY | O_CREAT | O_APPEND,
                Self::Create => O_WRONLY | O_CREAT | O_EXCL,
            }
    }
}

/// Returns the path as the bytes the C library takes, ended by a zero byte: UTF-8, with
/// U+DC80 to U+DCFF as the bytes they stand for, as CPython's `os.fsencode` gives them.
///
/// # Safety
///
/// `path` points to a live `str`.
unsafe fn path_bytes(
    path: *const StrObject,
) -> Result<crate::byte_buffer::ByteBuffer, *mut ExceptionObject> {
    let mut bytes = crate::byte_buffer::ByteBuffer::new();
    // SAFETY: the caller's guarantee.
    let count = unsafe { length(path) };
    for index in 0..count {
        // SAFETY: the index is below the length.
        let point = unsafe { code_point(path, index) };
        let error = match point {
            0 => Some(crate::exception::raise(
                crate::exception::ExceptionKind::ValueError,
                "embedded null byte",
            )),
            #[allow(
                clippy::cast_possible_truncation,
                reason = "U+DC80 to U+DCFF stand for the bytes 0x80 to 0xFF"
            )]
            0xdc80..=0xdcff => {
                bytes.extend(&[(point - 0xdc00) as u8]);
                None
            }
            0xd800..=0xdfff => {
                let end = (index..count)
                    // SAFETY: the index is below the length.
                    .find(|at| !(0xd800..=0xdfff).contains(&unsafe { code_point(path, *at) }))
                    .unwrap_or(count);
                // SAFETY: the caller's guarantee; surrogates run from `index` to `end`.
                Some(unsafe { crate::codec::utf8_encode_error(path, index, end) })
            }
            _ => {
                let mut encoded = [0_u8; 4];
                let character = char::from_u32(point).unwrap_or('\u{fffd}');
                bytes.extend(character.encode_utf8(&mut encoded).as_bytes());
                None
            }
        };
        if let Some(error) = error {
            bytes.discard();
            return Err(error);
        }
    }
    bytes.extend(&[0]);
    Ok(bytes)
}

/// Returns `open(path, mode)` for a text mode the checker has read: `access` is the number of
/// its `Access`, and `mode` the mode as written, which the file's `repr` shows.
///
/// # Safety
///
/// `path` and `mode` point to live strings, and `access` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_open(
    path: *const StrObject,
    mode: *const StrObject,
    access: i64,
) -> Raising<*mut TextFile> {
    let access = Access::of(access);
    // SAFETY: the caller's guarantee.
    let bytes = match unsafe { path_bytes(path) } {
        Ok(bytes) => bytes,
        Err(error) => return Raising::error(core::ptr::null_mut(), error),
    };
    let opened = crate::libc::open(bytes.bytes(), access.flags());
    bytes.discard();
    let descriptor = match opened {
        Ok(descriptor) => descriptor,
        Err(errno) => {
            // SAFETY: the caller passes a live path.
            let error = unsafe { super::os_error::os_error(errno, path) };
            return Raising::error(core::ptr::null_mut(), error);
        }
    };
    if access == Access::Read && crate::libc::is_directory(descriptor) {
        let _ = crate::libc::close(descriptor);
        // SAFETY: the caller passes a live path.
        let error = unsafe { super::os_error::os_error(crate::libc::EISDIR, path) };
        return Raising::error(core::ptr::null_mut(), error);
    }
    FUNCTIONS.with(|functions| *functions = Some(&DISK_FUNCTIONS));
    // SAFETY: the caller passes live strings, which the file keeps a reference to.
    unsafe {
        typy_rt_increment(path.cast_mut().cast());
        typy_rt_increment(mode.cast_mut().cast());
    }
    let (flags, reader, writer) = if access == Access::Read {
        (READABLE, boxed(Reader::new()), core::ptr::null_mut())
    } else {
        (WRITABLE, core::ptr::null_mut(), boxed(Writer::new()))
    };
    let next = OPEN.with(|open| *open);
    let file = boxed(TextFile {
        header: ObjectHeader::new(TEXT_FILE_TYPE_ID),
        descriptor,
        flags,
        name: "",
        path: path.cast_mut(),
        mode: mode.cast_mut(),
        reader,
        writer,
        previous: core::ptr::null_mut(),
        next,
    });
    if !next.is_null() {
        // SAFETY: the open files are live.
        unsafe { (*next).previous = file };
    }
    OPEN.with(|open| *open = file);
    Raising::ok(file)
}

/// Returns `value` in memory of its own, which `free` releases.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "malloc returns memory aligned for every type"
)]
fn boxed<T>(value: T) -> *mut T {
    let memory = crate::allocator::allocate(core::mem::size_of::<T>()).cast::<T>();
    // SAFETY: the allocation holds a `T`.
    unsafe { memory.write(value) };
    memory
}

/// Returns `ValueError: I/O operation on closed file.`
pub(crate) fn closed_error() -> *mut ExceptionObject {
    crate::exception::raise(
        crate::exception::ExceptionKind::ValueError,
        "I/O operation on closed file.",
    )
}

/// Returns `repr` of a file `open` made: `<_io.TextIOWrapper name='a.txt' mode='w'
/// encoding='utf-8'>`, with the `repr` of the path and the mode.
///
/// # Safety
///
/// The file's path and mode are live strings.
pub(crate) unsafe fn repr(file: &TextFile) -> *mut StrObject {
    // SAFETY: the caller's guarantee; each part is a live string released once.
    unsafe {
        let parts = [
            crate::string::from_str("<_io.TextIOWrapper name="),
            crate::string::typy_rt_str_repr(file.path),
            crate::string::from_str(" mode="),
            crate::string::typy_rt_str_repr(file.mode),
            crate::string::from_str(" encoding='utf-8'>"),
        ];
        let mut text = crate::string::from_str("");
        for part in parts {
            let joined = crate::string::typy_rt_str_concat(text, part);
            typy_rt_decrement(text.cast());
            typy_rt_decrement(part.cast());
            text = joined;
        }
        text
    }
}

/// Writes all of `bytes` to the descriptor.
fn sink(descriptor: i32) -> impl FnMut(&[u8]) -> Result<(), i32> {
    move |bytes| crate::libc::write_retrying(descriptor, bytes)
}

/// Adds `text` to what the file writes, or returns the error: closed, not writable, or one of
/// encoding or of writing out a full buffer.
///
/// # Safety
///
/// `text` points to a live string.
pub(crate) unsafe fn write(file: &mut TextFile, text: *const StrObject) -> *mut ExceptionObject {
    if file.flags & CLOSED != 0 {
        return closed_error();
    }
    if file.flags & WRITABLE == 0 {
        return crate::exception::raise_class(
            &crate::exception::classes::UNSUPPORTED_OPERATION,
            crate::string::from_str("not writable"),
        );
    }
    let mut out = sink(file.descriptor);
    // SAFETY: a writable file that is open has its writer; the caller's guarantee.
    match unsafe { (*file.writer).write(text, &mut out) } {
        Ok(()) => core::ptr::null_mut(),
        Err(error) => error,
    }
}

/// Writes out what the file holds; a closed file is an error.
pub(crate) fn flush(file: &mut TextFile) -> *mut ExceptionObject {
    if file.flags & CLOSED != 0 {
        return closed_error();
    }
    if file.writer.is_null() {
        return core::ptr::null_mut();
    }
    let mut out = sink(file.descriptor);
    // SAFETY: an open file's writer is live.
    match unsafe { (*file.writer).flush(&mut out) } {
        Ok(()) => core::ptr::null_mut(),
        Err(error) => error,
    }
}

/// Reads from the file: the rest when `limit` is `None` and not `line`, otherwise up to
/// `limit` characters, ending after a `\n` when `line`.
pub(crate) fn read(
    file: &mut TextFile,
    limit: Option<usize>,
    line: bool,
) -> Result<*mut StrObject, *mut ExceptionObject> {
    if file.flags & CLOSED != 0 {
        return Err(closed_error());
    }
    if file.flags & READABLE == 0 {
        return Err(super::not_readable());
    }
    let descriptor = file.descriptor;
    let mut source = |buffer: &mut [u8]| crate::libc::read_retrying(descriptor, buffer);
    // SAFETY: a readable file that is open has its reader.
    let reader = unsafe { &mut *file.reader };
    if limit.is_none() && !line {
        reader.read_rest(&mut source)
    } else {
        reader.read_text(limit, line, &mut source)
    }
}

/// Closes the file once: writes out its buffer, closes its descriptor and frees its buffers;
/// returns the first error, after closing the descriptor anyway, as CPython does.
pub(crate) fn close(file: &mut TextFile) -> *mut ExceptionObject {
    if file.flags & CLOSED != 0 {
        return core::ptr::null_mut();
    }
    let flushed = flush(file);
    let closed = crate::libc::close(file.descriptor);
    file.flags |= CLOSED;
    crate::allocator::free(file.reader.cast());
    crate::allocator::free(file.writer.cast());
    file.reader = core::ptr::null_mut();
    file.writer = core::ptr::null_mut();
    unlink(file);
    if !flushed.is_null() {
        return flushed;
    }
    match closed {
        Ok(()) => core::ptr::null_mut(),
        // SAFETY: an error of closing names no file.
        Err(errno) => unsafe { super::os_error::os_error(errno, core::ptr::null()) },
    }
}

/// Takes the file off the list of open files.
fn unlink(file: &mut TextFile) {
    let (previous, next) = (file.previous, file.next);
    // SAFETY: the files on the list are live.
    unsafe {
        if previous.is_null() {
            OPEN.with(|open| *open = next);
        } else {
            (*previous).next = next;
        }
        if !next.is_null() {
            (*next).previous = previous;
        }
    }
    file.previous = core::ptr::null_mut();
    file.next = core::ptr::null_mut();
}

/// Closes the file if it is open, dropping any error as CPython drops it when it frees a
/// file, then releases its path and mode and frees it.
///
/// # Safety
///
/// `file` points to a file `open` made that nothing refers to any more.
unsafe fn destroy(file: *mut TextFile) {
    // SAFETY: the caller's guarantee; the file owns its path and mode.
    unsafe {
        let error = close(&mut *file);
        typy_rt_decrement(error.cast());
        typy_rt_decrement((*file).path.cast());
        typy_rt_decrement((*file).mode.cast());
    }
    crate::allocator::free(file.cast());
}

/// Closes every file still open, dropping their errors, at the end of the program.
fn close_all() {
    loop {
        let file = OPEN.with(|open| *open);
        if file.is_null() {
            return;
        }
        // SAFETY: the files on the list are live; closing takes the file off it.
        unsafe {
            let error = close(&mut *file);
            typy_rt_decrement(error.cast());
        }
    }
}
