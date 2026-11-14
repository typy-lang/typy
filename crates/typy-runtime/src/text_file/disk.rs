//! The files `open` makes (ADR-0019): opening a path in a text mode, reading and writing
//! through the file's buffers, closing, and the list of open files the program closes at its
//! end.

use super::newline::Newline;
use super::reader::{Decoding, Reader};
use super::writer::{Encoding, Writer};
use super::{CLOSED, FUNCTIONS, FileFunctions, READABLE, TextFile, WRITABLE};
use crate::Raising;
use crate::cell::ProgramCell;
use crate::codec::{Codec, Handler};
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
    const fn of(number: i64) -> Self {
        match number {
            1 => Self::Write,
            2 => Self::Append,
            3 => Self::Create,
            _ => Self::Read,
        }
    }

    /// Returns the flags `open` takes for the access, reading and writing with `plus`.
    const fn flags(self, plus: bool) -> i32 {
        use crate::libc::{O_APPEND, O_CLOEXEC, O_CREAT, O_EXCL, O_RDWR, O_TRUNC, O_WRONLY};
        let direction = match (self, plus) {
            (_, true) => O_RDWR,
            (Self::Read, false) => 0,
            (_, false) => O_WRONLY,
        };
        O_CLOEXEC
            | direction
            | match self {
                Self::Read => 0,
                Self::Write => O_CREAT | O_TRUNC,
                Self::Append => O_CREAT | O_APPEND,
                Self::Create => O_CREAT | O_EXCL,
            }
    }
}

/// Returns the access of a text mode the program gave as a value and whether it has `+`,
/// with CPython's `ValueError` for a mode it refuses: each character one of `rwxabt+` and
/// given once, not both `t` and `b`, and exactly one of `r`, `w`, `a` and `x`. A binary mode
/// raises `TypeError`, since such a mode gives a text file in TyPy (ADR-0031).
///
/// # Safety
///
/// `mode` points to a live `str`.
unsafe fn mode_value(mode: *const StrObject) -> Result<(Access, bool), *mut ExceptionObject> {
    use crate::exception::ExceptionKind::{TypeError, ValueError};
    // SAFETY: the caller's guarantee.
    let count = unsafe { length(mode) };
    // SAFETY: the caller's guarantee; every index is below the length.
    let point = |index: usize| unsafe { code_point(mode, index) };
    let has = |character: u8| (0..count).any(|index| point(index) == u32::from(character));
    let valid = (0..count).all(|index| {
        let each = point(index);
        b"rwxabt+".iter().any(|allowed| u32::from(*allowed) == each)
            && !(index + 1..count).any(|later| point(later) == each)
    });
    let refuse = |kind, text: &str, shown: bool| {
        let mut message = crate::message::MessageBuffer::new();
        message.push_str(text);
        if shown {
            // SAFETY: the caller's guarantee.
            unsafe { message.push_text(mode) };
            message.push_str("'");
        }
        message.raise(kind)
    };
    if !valid {
        return Err(refuse(ValueError, "invalid mode: '", true));
    }
    if has(b't') && has(b'b') {
        return Err(refuse(
            ValueError,
            "can't have text and binary mode at once",
            false,
        ));
    }
    let accesses = b"rwax".iter().filter(|access| has(**access)).count();
    if accesses == 0 {
        return Err(refuse(
            ValueError,
            "Must have exactly one of create/read/write/append mode and at most one plus",
            false,
        ));
    }
    if accesses > 1 {
        return Err(refuse(
            ValueError,
            "must have exactly one of create/read/write/append mode",
            false,
        ));
    }
    if has(b'b') {
        return Err(refuse(
            TypeError,
            "TyPy opens a text file for a mode that is not a literal, and this mode is \
             binary: '",
            true,
        ));
    }
    let access = if has(b'w') {
        Access::Write
    } else if has(b'a') {
        Access::Append
    } else if has(b'x') {
        Access::Create
    } else {
        Access::Read
    };
    Ok((access, has(b'+')))
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

/// How a file opened in a text mode decodes, encodes and buffers.
struct Options {
    decoding: Decoding,
    line_buffering: bool,
}

/// Returns the options of `open` that need the file open to be checked, as CPython checks
/// them after opening it: `buffering=0` and an illegal `newline`.
///
/// # Safety
///
/// `newline` is null or points to a live `str`.
unsafe fn options(
    buffering: i32,
    codec: i64,
    handler: i64,
    newline: *const StrObject,
) -> Result<Options, *mut ExceptionObject> {
    if buffering == 0 {
        return Err(crate::exception::raise(
            crate::exception::ExceptionKind::ValueError,
            "can't have unbuffered text I/O",
        ));
    }
    // SAFETY: the caller's guarantee.
    let newline = unsafe { Newline::of(newline)? };
    Ok(Options {
        decoding: Decoding {
            codec: Codec::from_number(codec),
            handler: Handler::from_number(handler),
            newline,
        },
        line_buffering: buffering == 1,
    })
}

/// Returns `open(path, mode, buffering, encoding, errors, newline)` for a text mode the
/// checker has read. `how` holds the number of its `Access` in its two low bits, whether the
/// mode has `+` in the next, the codec `encoding` names in the two after, the error handler
/// `errors` names in the three after, then whether the file is binary and whether it is a
/// `FileIO`, so the call keeps to six arguments; `mode` and `encoding` are as written, which the file's `repr`
/// shows, `encoding` null for `None`; `newline` is null for `None`.
///
/// # Safety
///
/// `path` and `mode` point to live strings, `encoding` and `newline` are null or point to
/// live strings, and `how` and `buffering` are `int` words whose large integers are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_open(
    path: *const StrObject,
    mode: *const StrObject,
    how: i64,
    buffering: i64,
    encoding: *const StrObject,
    newline: *const StrObject,
) -> Raising<*mut TextFile> {
    let how = crate::word::inline_value(how);
    // A mode that is not a literal is read here, as CPython reads it (ADR-0031).
    let (access, plus) = if how & (1 << 10) == 0 {
        (Access::of(how & 3), how & 4 != 0)
    } else {
        // SAFETY: the caller passes a live mode.
        match unsafe { mode_value(mode) } {
            Ok(found) => found,
            Err(error) => return Raising::error(core::ptr::null_mut(), error),
        }
    };
    let (codec, handler) = ((how >> 3) & 3, (how >> 5) & 7);
    let (binary, raw) = (how & (1 << 8) != 0, how & (1 << 9) != 0);
    // SAFETY: the caller's guarantee.
    let buffering = unsafe { super::position::value_of(buffering) };
    let Some(buffering) = buffering.and_then(|value| i32::try_from(value).ok()) else {
        let error = crate::exception::raise(
            crate::exception::ExceptionKind::OverflowError,
            "Python int too large to convert to C int",
        );
        return Raising::error(core::ptr::null_mut(), error);
    };
    // SAFETY: the caller's guarantee.
    let bytes = match unsafe { path_bytes(path) } {
        Ok(bytes) => bytes,
        Err(error) => return Raising::error(core::ptr::null_mut(), error),
    };
    let opened = crate::libc::open(bytes.bytes(), access.flags(plus));
    bytes.discard();
    let descriptor = match opened {
        Ok(descriptor) => descriptor,
        Err(errno) => {
            // SAFETY: the caller passes a live path.
            let error = unsafe { super::os_error::os_error(errno, path) };
            return Raising::error(core::ptr::null_mut(), error);
        }
    };
    if access == Access::Read && !plus && crate::libc::is_directory(descriptor) {
        let _ = crate::libc::close(descriptor);
        // SAFETY: the caller passes a live path.
        let error = unsafe { super::os_error::os_error(crate::libc::EISDIR, path) };
        return Raising::error(core::ptr::null_mut(), error);
    }
    // SAFETY: the caller's guarantee.
    let options = match unsafe { options(buffering, codec, handler, newline) } {
        Ok(options) => options,
        Err(error) => {
            let _ = crate::libc::close(descriptor);
            return Raising::error(core::ptr::null_mut(), error);
        }
    };
    if access == Access::Append {
        // CPython's `FileIO` moves to the end at once, so `tell` gives the size.
        let _ = crate::libc::seek(descriptor, 0, crate::libc::SEEK_END);
    }
    FUNCTIONS.with(|functions| *functions = Some(&DISK_FUNCTIONS));
    // SAFETY: the caller passes live strings, which the file keeps a reference to.
    unsafe {
        typy_rt_increment(path.cast_mut().cast());
        typy_rt_increment(mode.cast_mut().cast());
        typy_rt_increment(encoding.cast_mut().cast());
    }
    let (flags, reader, writer) = buffers(access, plus, (binary, raw), &options);
    let next = OPEN.with(|open| *open);
    let file = boxed(TextFile {
        header: ObjectHeader::new(TEXT_FILE_TYPE_ID),
        descriptor,
        flags,
        name: "",
        path: path.cast_mut(),
        mode: mode.cast_mut(),
        encoding: encoding.cast_mut(),
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

/// Returns the flags of a new file and the buffers it reads and writes through: a mode with
/// `+` both reads and writes, as CPython's `BufferedRandom` does; a binary file reads its
/// bytes through a reader of `latin-1`, which checks none; a `FileIO` has no buffers.
fn buffers(
    access: Access,
    plus: bool,
    (binary, raw): (bool, bool),
    options: &Options,
) -> (u32, *mut Reader, *mut Writer) {
    let readable = access == Access::Read || plus;
    let writable = access != Access::Read || plus;
    let decoding = if binary {
        Decoding {
            codec: Codec::Latin1,
            handler: Handler::Strict,
            newline: Newline::Lf,
        }
    } else {
        options.decoding
    };
    let reader = if readable && !raw {
        boxed(Reader::with(decoding))
    } else {
        core::ptr::null_mut()
    };
    let writer = if writable && !raw {
        boxed(Writer::with(Encoding {
            codec: options.decoding.codec,
            handler: options.decoding.handler,
            newline: options.decoding.newline,
            line_buffering: options.line_buffering,
        }))
    } else {
        core::ptr::null_mut()
    };
    let flags = if readable { READABLE } else { 0 }
        | if writable { WRITABLE } else { 0 }
        | if binary { super::BINARY } else { 0 }
        | if raw { super::RAW } else { 0 };
    (flags, reader, writer)
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
    use super::{BINARY, RAW};
    let flags = file.flags;
    if flags & RAW != 0 && flags & CLOSED != 0 {
        return crate::string::from_str("<_io.FileIO [closed]>");
    }
    let class = match (
        flags & BINARY != 0,
        flags & RAW != 0,
        flags & (READABLE | WRITABLE),
    ) {
        (false, _, _) => "<_io.TextIOWrapper name=",
        (true, true, _) => "<_io.FileIO name=",
        (true, false, READABLE) => "<_io.BufferedReader name=",
        (true, false, WRITABLE) => "<_io.BufferedWriter name=",
        (true, false, _) => "<_io.BufferedRandom name=",
    };
    // SAFETY: the caller's guarantee; each part is a live string released once.
    unsafe {
        let tail: [*mut StrObject; 4] = match (flags & BINARY != 0, flags & RAW != 0) {
            (false, _) => [
                crate::string::from_str(" mode="),
                crate::string::typy_rt_str_repr(file.mode),
                crate::string::from_str(" encoding="),
                if file.encoding.is_null() {
                    crate::string::from_str("'utf-8'>")
                } else {
                    let shown = crate::string::typy_rt_str_repr(file.encoding);
                    let closing = crate::string::from_str(">");
                    let joined = crate::string::typy_rt_str_concat(shown, closing);
                    typy_rt_decrement(shown.cast());
                    typy_rt_decrement(closing.cast());
                    joined
                },
            ],
            (true, true) => [
                crate::string::from_str(" mode="),
                crate::string::typy_rt_str_repr(file.mode),
                crate::string::from_str(" closefd=True>"),
                crate::string::from_str(""),
            ],
            (true, false) => [
                crate::string::from_str(">"),
                crate::string::from_str(""),
                crate::string::from_str(""),
                crate::string::from_str(""),
            ],
        };
        let parts = [
            crate::string::from_str(class),
            crate::string::typy_rt_str_repr(file.path),
            tail[0],
            tail[1],
            tail[2],
            tail[3],
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
    // A write after a read starts where the reader stopped reading the descriptor and drops
    // what it holds, as CPython's text layer does over `BufferedRandom`.
    if !file.reader.is_null() {
        // SAFETY: an open readable file's reader is live.
        unsafe { (*file.reader).reset() };
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
    // A read after a write writes out what the file holds first.
    let flushed = flush(file);
    if !flushed.is_null() {
        return Err(flushed);
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
        typy_rt_decrement((*file).encoding.cast());
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
