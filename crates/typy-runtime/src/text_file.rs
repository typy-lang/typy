//! Text files, `_io.TextIOWrapper` (LIB-04, ADR-0019): standard output and standard error,
//! written as UTF-8, standard output through its buffer, and the files `open` makes, which
//! `disk` reads and writes.

mod binary;
mod disk;
mod errno_text;
mod newline;
mod os_error;
pub(crate) use os_error::typy_rt_os_error_str;
mod position;
mod reader;
mod state;
mod stdin;
pub(crate) use stdin::input_line;
#[cfg(test)]
mod binary_tests;
#[cfg(test)]
mod decoding_tests;
#[cfg(test)]
mod position_tests;
#[cfg(test)]
mod tests;
mod writer;

use crate::Raising;
use crate::cell::ProgramCell;
use crate::exception::ExceptionObject;
use crate::header::{IMMORTAL, ObjectHeader, TEXT_FILE_TYPE_ID};
use crate::list::ListObject;
use crate::message::MessageBuffer;
use crate::string::{StrObject, Surrogates, length, unescapable, write_utf8};
use crate::type_object::{OBJECT, TypeObject};

/// The file can be read.
const READABLE: u32 = 1;
/// The file can be written.
const WRITABLE: u32 = 2;
/// The file is closed.
const CLOSED: u32 = 4;
/// The file is standard input, output or error, a static object that is never closed.
const STANDARD: u32 = 8;
/// The file is binary: a `BufferedReader`, `BufferedWriter` or `BufferedRandom` (LIB-14).
const BINARY: u32 = 16;
/// The binary file is a `FileIO`, which has no buffers.
const RAW: u32 = 32;
/// The file was opened for a file descriptor, which is its `name`.
const DESCRIPTOR: u32 = 64;
/// The file was opened with `closefd=False`, so closing it leaves its descriptor open.
const KEEP_OPEN: u32 = 128;

/// A text file: its descriptor and state, its name and mode, and the buffers of a file
/// `open` made.
#[repr(C)]
#[derive(Debug)]
pub struct TextFile {
    header: ObjectHeader,
    descriptor: i32,
    flags: u32,
    /// The name `repr` shows for a standard file.
    name: &'static str,
    /// The path a file was opened with, which the file owns; null for a standard file.
    path: *mut StrObject,
    /// The mode as the program wrote it, which the file owns; null for a standard file.
    mode: *mut StrObject,
    /// The encoding as the program wrote it, which the file owns; null for `utf-8`.
    encoding: *mut StrObject,
    /// What a readable file has read and not given out; null otherwise and once closed.
    reader: *mut reader::Reader,
    /// What a writable file has not written out; null otherwise and once closed.
    writer: *mut writer::Writer,
    /// The files open before and after this one, which the program closes at its end.
    previous: *mut TextFile,
    next: *mut TextFile,
}

/// Returns a standard file of `descriptor`, with its flags.
const fn standard(descriptor: i32, name: &'static str, flags: u32) -> TextFile {
    TextFile {
        header: ObjectHeader {
            reference_count: IMMORTAL,
            type_id: TEXT_FILE_TYPE_ID,
        },
        descriptor,
        flags,
        name,
        path: core::ptr::null_mut(),
        mode: core::ptr::null_mut(),
        encoding: core::ptr::null_mut(),
        reader: core::ptr::null_mut(),
        writer: core::ptr::null_mut(),
        previous: core::ptr::null_mut(),
        next: core::ptr::null_mut(),
    }
}

/// `sys.stdout`.
static STDOUT: TextFile = standard(crate::libc::STDOUT, "<stdout>", WRITABLE | STANDARD);

/// `sys.stderr`.
static STDERR: TextFile = standard(crate::libc::STDERR, "<stderr>", WRITABLE | STANDARD);

// SAFETY: the runtime runs one thread (A-03), and a standard file is never written.
unsafe impl Sync for TextFile {}

/// The type `_io.TextIOWrapper`.
#[unsafe(export_name = "typy_rt_type_text_file")]
pub(crate) static TEXT_FILE: TypeObject = TypeObject {
    hash: Some(crate::instance::typy_rt_identity_hash),
    repr: Some(text_file_repr),
    module: "_io".as_ptr(),
    module_length: 3,
    ..TypeObject::new("TextIOWrapper", &raw const OBJECT)
};

/// What a program that opens files needs from the code every program has: freeing a file
/// whose last reference goes, and closing the files still open at the end. `open` installs
/// them, so a program that never opens a file does not link the code that reads and writes
/// one (NFR-01).
struct FileFunctions {
    destroy: unsafe fn(*mut TextFile),
    close_all: fn(),
}

static FUNCTIONS: ProgramCell<Option<&'static FileFunctions>> = ProgramCell::new(None);

/// Returns the functions `open` installed, if it ran. The read is volatile: `open` stores
/// only one value, and an ordinary read lets the optimizer call that value's functions
/// directly, which would link them into every program.
fn functions() -> Option<&'static FileFunctions> {
    // SAFETY: the reference points to the cell's value, which is initialised.
    FUNCTIONS.with(|functions| unsafe { core::ptr::read_volatile(functions) })
}

/// Frees a file `open` made, whose last reference has gone.
///
/// # Safety
///
/// `object` points to a text file nothing refers to any more.
pub(crate) unsafe fn destroy(object: *mut ObjectHeader) {
    if let Some(functions) = functions() {
        // SAFETY: the caller's guarantee; only `open` makes text files that are counted.
        unsafe { (functions.destroy)(object.cast()) };
    }
}

/// Closes every file still open, at the end of the program, as CPython does.
pub(crate) fn close_all() {
    if let Some(functions) = functions() {
        (functions.close_all)();
    }
}

/// Returns `repr` of a text file, as CPython writes it:
/// `<_io.TextIOWrapper name='<stdout>' mode='w' encoding='utf-8'>`.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "a text file is a static or comes from malloc, aligned for its fields"
)]
unsafe extern "C" fn text_file_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    // SAFETY: the type of the object says it is a text file.
    let file = unsafe { &*object.cast::<TextFile>() };
    if file.flags & STANDARD != 0 {
        let mut text = MessageBuffer::new();
        text.push_str("<_io.TextIOWrapper name='");
        text.push_str(file.name);
        text.push_str(if file.flags & READABLE != 0 {
            "' mode='r' encoding='utf-8'>"
        } else {
            "' mode='w' encoding='utf-8'>"
        });
        return Raising::ok(text.text());
    }
    // SAFETY: a file `open` made owns its path and mode, both live strings.
    unsafe { Raising::ok(disk::repr(file)) }
}

/// Returns `sys.stdout`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_sys_stdout() -> *const TextFile {
    &raw const STDOUT
}

/// Returns `sys.stderr`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_sys_stderr() -> *const TextFile {
    &raw const STDERR
}

/// Returns the flags of `file` with `CLOSED` set for a standard file the program closed.
///
/// # Safety
///
/// `file` points to a live text file.
pub(crate) unsafe fn flags_of(file: *const TextFile) -> u32 {
    // SAFETY: the caller's guarantee.
    let (flags, descriptor) = unsafe { ((*file).flags, (*file).descriptor) };
    if flags & STANDARD != 0 && crate::io::standard_closed(descriptor) {
        return flags | CLOSED;
    }
    flags
}

/// Returns `ValueError: I/O operation on closed file.`
pub(crate) fn closed_error() -> *mut ExceptionObject {
    disk::closed_error()
}

/// Writes `text` to standard output for `print` in a program that may close it, or returns
/// the error once it is closed (LIB-14).
///
/// # Safety
///
/// `text` points to a live `str`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_write_stdout_checked(
    text: *const StrObject,
) -> *mut ExceptionObject {
    if crate::io::standard_closed(crate::libc::STDOUT) {
        return disk::closed_error();
    }
    // SAFETY: the caller's guarantee.
    unsafe { write_standard_output(text) }
}

/// Writes a string to standard output, or returns the `UnicodeEncodeError` of a surrogate
/// `surrogateescape` cannot write, before writing any of it, as CPython's encoder does.
///
/// # Safety
///
/// `text` points to a live string.
unsafe fn write_standard_output(text: *const StrObject) -> *mut ExceptionObject {
    // SAFETY: the caller's guarantee.
    unsafe {
        if let Some((start, end)) = unescapable(text) {
            return crate::codec::utf8_encode_error(text, start, end);
        }
        crate::io::typy_rt_write_stdout(text);
    }
    core::ptr::null_mut()
}

/// Returns whether `file` is null, which stands for standard output, or a standard file.
///
/// # Safety
///
/// `file` is null or points to a live text file.
unsafe fn is_standard(file: *const TextFile) -> bool {
    // SAFETY: the caller's guarantee.
    file.is_null() || unsafe { (*file).flags } & STANDARD != 0
}

/// Returns whether `file` is standard input.
///
/// # Safety
///
/// `file` is null or points to a live text file.
unsafe fn is_stdin(file: *const TextFile) -> bool {
    // SAFETY: the caller's guarantee.
    !file.is_null() && unsafe { (*file).flags } & (STANDARD | READABLE) == STANDARD | READABLE
}

/// Reads from a standard file: standard input as `stdin::read` does, and the others not.
///
/// # Safety
///
/// `file` points to a live standard text file.
unsafe fn read_standard(
    file: *const TextFile,
    limit: Option<usize>,
    line: bool,
) -> Result<*mut StrObject, *mut ExceptionObject> {
    // SAFETY: the caller's guarantee.
    if unsafe { flags_of(file) } & CLOSED != 0 {
        return Err(disk::closed_error());
    }
    // SAFETY: the caller's guarantee.
    if unsafe { is_stdin(file) } {
        return stdin::read(limit, line);
    }
    Err(not_readable())
}

/// Returns `io.UnsupportedOperation: not writable`, what writing to standard input raises.
fn not_writable() -> *mut ExceptionObject {
    crate::exception::raise_class(
        &crate::exception::classes::UNSUPPORTED_OPERATION,
        crate::string::from_str("not writable"),
    )
}

/// Returns `io.UnsupportedOperation: not readable`, what reading a standard file raises.
fn not_readable() -> *mut ExceptionObject {
    crate::exception::raise_class(
        &crate::exception::classes::UNSUPPORTED_OPERATION,
        crate::string::from_str("not readable"),
    )
}

/// Writes `text` to `file`, or to standard output when `file` is null, as `print(file=None)`
/// does, and returns the number of code points written, an `int` word.
///
/// # Safety
///
/// `file` is null or points to a live text file, and `text` to a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_text_write(
    file: *mut TextFile,
    text: *const StrObject,
) -> Raising<i64> {
    // SAFETY: the caller's guarantee.
    unsafe {
        if !file.is_null() && flags_of(file) & CLOSED != 0 {
            return Raising::error(0, disk::closed_error());
        }
        if is_stdin(file) {
            return Raising::error(0, not_writable());
        }
        if !is_standard(file) {
            let error = disk::write(&mut *file, text);
            if !error.is_null() {
                return Raising::error(0, error);
            }
        } else if file.is_null() || (*file).descriptor == crate::libc::STDOUT {
            let error = write_standard_output(text);
            if !error.is_null() {
                return Raising::error(0, error);
            }
        } else {
            write_utf8(text, Surrogates::Backslash, crate::io::write_stderr);
        }
    }
    // SAFETY: the caller passes a live string.
    let count = i64::try_from(unsafe { length(text) }).unwrap_or(i64::MAX);
    Raising::ok(crate::word::inline(count).unwrap_or(1))
}

/// Writes each `str` of `lines` to `file`, as `writelines` does.
///
/// # Safety
///
/// `file` points to a live text file, and `lines` to a live list of `str`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_text_writelines(
    file: *mut TextFile,
    lines: *const ListObject,
) -> *mut ExceptionObject {
    // SAFETY: the caller's guarantee.
    unsafe {
        if flags_of(file) & CLOSED != 0 {
            return disk::closed_error();
        }
        // The length is read at each step, as CPython's iterator reads it.
        let mut index = 0;
        let binary = binary::is_binary(file);
        while index < (*lines).length {
            let line = (*lines).item(index) as *const StrObject;
            let written = if binary {
                binary::typy_rt_binary_write(file, line)
            } else {
                typy_rt_text_write(file, line)
            };
            if !written.exception.is_null() {
                return written.exception;
            }
            index += 1;
        }
    }
    core::ptr::null_mut()
}

/// Writes out what `file` holds; standard error holds nothing.
///
/// # Safety
///
/// `file` is null or points to a live text file.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_text_flush(file: *mut TextFile) -> *mut ExceptionObject {
    // SAFETY: the caller's guarantee.
    unsafe {
        if !is_standard(file) {
            // A closed `BufferedWriter` or `BufferedRandom` names the operation.
            let flags = (*file).flags;
            if flags & (BINARY | RAW | WRITABLE | CLOSED) == BINARY | WRITABLE | CLOSED {
                return crate::exception::raise(
                    crate::exception::ExceptionKind::ValueError,
                    "flush of closed file",
                );
            }
            return disk::flush(&mut *file);
        }
        if !file.is_null() && flags_of(file) & CLOSED != 0 {
            return disk::closed_error();
        }
        if file.is_null() || (*file).descriptor == crate::libc::STDOUT {
            crate::io::flush_stdout();
        }
    }
    core::ptr::null_mut()
}

/// Closes `file` once: a file `open` made writes out its buffer and closes its descriptor; a
/// standard file is written out and marked closed, and its descriptor stays open, as
/// CPython opens it with `closefd=False` (LIB-14).
///
/// # Safety
///
/// `file` points to a live text file.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_text_close(file: *mut TextFile) -> *mut ExceptionObject {
    // SAFETY: the caller's guarantee.
    unsafe {
        if !is_standard(file) {
            return disk::close(&mut *file);
        }
        if flags_of(file) & CLOSED != 0 {
            return core::ptr::null_mut();
        }
        let flushed = typy_rt_text_flush(file);
        crate::io::close_standard((*file).descriptor);
        flushed
    }
}

/// Checks that `file` can enter a `with` statement, which a closed file cannot, as
/// `__enter__` checks.
///
/// # Safety
///
/// `file` points to a live text file.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_text_enter(file: *mut TextFile) -> *mut ExceptionObject {
    // SAFETY: the caller's guarantee.
    if unsafe { flags_of(file) } & CLOSED != 0 {
        return disk::closed_error();
    }
    core::ptr::null_mut()
}

/// Returns `file.read(size)`: `size` characters at most, or the rest of the file when `size`
/// is negative.
///
/// # Safety
///
/// `file` points to a live text file, and `size` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_text_read(
    file: *mut TextFile,
    size: i64,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller's guarantee.
    unsafe {
        if is_standard(file) {
            return read_standard(file, limit(size), false).into();
        }
        disk::read(&mut *file, limit(size), false).into()
    }
}

/// Returns `file.readline(size)`: the next line with its `\n`, `size` characters at most when
/// `size` is not negative, and an empty string at the end.
///
/// # Safety
///
/// `file` points to a live text file, and `size` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_text_readline(
    file: *mut TextFile,
    size: i64,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller's guarantee.
    unsafe {
        if is_standard(file) {
            return read_standard(file, limit(size), true).into();
        }
        disk::read(&mut *file, limit(size), true).into()
    }
}

/// Returns `file.readlines()`: the list of the lines left.
///
/// # Safety
///
/// `file` points to a live text file.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_text_readlines(file: *mut TextFile) -> Raising<*mut ListObject> {
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

/// Runs `action` with the reader of `file`: standard input's, or the one of a readable file
/// `open` made that is still open, and `None` for the others.
///
/// # Safety
///
/// `file` points to a live text file.
unsafe fn with_reader<R>(
    file: *mut TextFile,
    action: impl FnOnce(Option<&mut reader::Reader>) -> R,
) -> R {
    // SAFETY: the caller's guarantee; an open readable file's reader is live.
    unsafe {
        if is_stdin(file) {
            return stdin::with_reader(|reader| action(Some(reader)));
        }
        if is_standard(file) || (*file).reader.is_null() {
            return action(None);
        }
        action(Some(&mut *(*file).reader))
    }
}

/// Returns the next line of `file` for a `for` loop, or `None` at its end; a line turns
/// `tell` off until the end or a `seek`, as CPython's `next` does.
///
/// # Safety
///
/// `file` points to a live text file.
pub(crate) unsafe fn loop_line(
    file: *mut TextFile,
) -> Result<Option<*mut StrObject>, *mut ExceptionObject> {
    // SAFETY: the caller's guarantee.
    unsafe {
        let line = next_line(file)?;
        with_reader(file, |reader| {
            if let Some(reader) = reader {
                reader.set_iterating(line.is_some());
            }
        });
        Ok(line)
    }
}

/// Returns the next line of `file`, or `None` at its end, for iterating over the file.
///
/// # Safety
///
/// `file` points to a live text file.
pub(crate) unsafe fn next_line(
    file: *mut TextFile,
) -> Result<Option<*mut StrObject>, *mut ExceptionObject> {
    // SAFETY: the caller's guarantee.
    unsafe {
        if binary::is_binary(file) {
            return binary::next_line(file);
        }
        let line = if is_standard(file) {
            read_standard(file, None, true)?
        } else {
            disk::read(&mut *file, None, true)?
        };
        if length(line) == 0 {
            crate::refcount::typy_rt_decrement(line.cast());
            return Ok(None);
        }
        Ok(Some(line))
    }
}

/// Returns the limit a `size` argument gives: none for a negative one.
fn limit(size: i64) -> Option<usize> {
    usize::try_from(crate::word::inline_value(size)).ok()
}

impl From<Result<*mut StrObject, *mut ExceptionObject>> for Raising<*mut StrObject> {
    fn from(result: Result<*mut StrObject, *mut ExceptionObject>) -> Self {
        match result {
            Ok(value) => Self::ok(value),
            Err(error) => Self::error(core::ptr::null_mut(), error),
        }
    }
}
