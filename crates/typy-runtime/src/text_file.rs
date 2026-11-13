//! Text files, `_io.TextIOWrapper` (LIB-04, ADR-0019): standard output and standard error,
//! written as UTF-8, standard output through its buffer, and the files `open` makes, which
//! `disk` reads and writes.

mod disk;
mod errno_text;
mod os_error;
mod reader;
#[cfg(test)]
mod tests;
mod writer;

use crate::Raising;
use crate::cell::ProgramCell;
use crate::exception::ExceptionObject;
use crate::header::{IMMORTAL, ObjectHeader, TEXT_FILE_TYPE_ID};
use crate::list::ListObject;
use crate::message::MessageBuffer;
use crate::string::{StrObject, length, write_utf8};
use crate::type_object::{OBJECT, TypeObject};

/// The file can be read.
const READABLE: u32 = 1;
/// The file can be written.
const WRITABLE: u32 = 2;
/// The file is closed.
const CLOSED: u32 = 4;
/// The file is standard output or standard error, a static object that is never closed.
const STANDARD: u32 = 8;

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
    /// What a readable file has read and not given out; null otherwise and once closed.
    reader: *mut reader::Reader,
    /// What a writable file has not written out; null otherwise and once closed.
    writer: *mut writer::Writer,
    /// The files open before and after this one, which the program closes at its end.
    previous: *mut TextFile,
    next: *mut TextFile,
}

/// Returns a standard file writing to `descriptor`.
const fn standard(descriptor: i32, name: &'static str) -> TextFile {
    TextFile {
        header: ObjectHeader {
            reference_count: IMMORTAL,
            type_id: TEXT_FILE_TYPE_ID,
        },
        descriptor,
        flags: WRITABLE | STANDARD,
        name,
        path: core::ptr::null_mut(),
        mode: core::ptr::null_mut(),
        reader: core::ptr::null_mut(),
        writer: core::ptr::null_mut(),
        previous: core::ptr::null_mut(),
        next: core::ptr::null_mut(),
    }
}

/// `sys.stdout`.
static STDOUT: TextFile = standard(crate::libc::STDOUT, "<stdout>");

/// `sys.stderr`.
static STDERR: TextFile = standard(crate::libc::STDERR, "<stderr>");

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
    if file.path.is_null() {
        let mut text = MessageBuffer::new();
        text.push_str("<_io.TextIOWrapper name='");
        text.push_str(file.name);
        text.push_str("' mode='w' encoding='utf-8'>");
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

/// Returns whether `file` is null, which stands for standard output, or a standard file.
///
/// # Safety
///
/// `file` is null or points to a live text file.
unsafe fn is_standard(file: *const TextFile) -> bool {
    // SAFETY: the caller's guarantee.
    file.is_null() || unsafe { (*file).flags } & STANDARD != 0
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
        if !is_standard(file) {
            let error = disk::write(&mut *file, text);
            if !error.is_null() {
                return Raising::error(0, error);
            }
        } else if file.is_null() || (*file).descriptor == crate::libc::STDOUT {
            crate::io::typy_rt_write_stdout(text);
        } else {
            write_utf8(text, crate::io::write_stderr);
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
        if !is_standard(file) && (*file).flags & CLOSED != 0 {
            return disk::closed_error();
        }
        // The length is read at each step, as CPython's iterator reads it.
        let mut index = 0;
        while index < (*lines).length {
            let line = (*lines).item(index) as *const StrObject;
            let written = typy_rt_text_write(file, line);
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
            return disk::flush(&mut *file);
        }
        if file.is_null() || (*file).descriptor == crate::libc::STDOUT {
            crate::io::flush_stdout();
        }
    }
    core::ptr::null_mut()
}

/// Closes `file`: a file `open` made writes out its buffer and closes its descriptor, once; a
/// standard file is written out and stays open.
///
/// # Safety
///
/// `file` points to a live text file.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_text_close(file: *mut TextFile) -> *mut ExceptionObject {
    // SAFETY: the caller's guarantee.
    unsafe {
        if is_standard(file) {
            return typy_rt_text_flush(file);
        }
        disk::close(&mut *file)
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
    if unsafe { (*file).flags } & CLOSED != 0 {
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
            return Raising::error(core::ptr::null_mut(), not_readable());
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
            return Raising::error(core::ptr::null_mut(), not_readable());
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
        if is_standard(file) {
            return Err(not_readable());
        }
        let line = disk::read(&mut *file, None, true)?;
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
