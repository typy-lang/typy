//! Standard input, `sys.stdin` (LIB-13): a standard text file read as CPython's
//! `TextIOWrapper` reads it, with `\r\n` and `\r` read as `\n`. `input()` reads its lines
//! from the same buffer, so the two can be mixed as in CPython.

use super::reader::{Reader, Source};
use super::{READABLE, STANDARD, TextFile};
use crate::cell::ProgramCell;
use crate::exception::ExceptionObject;
use crate::string::{StrObject, code_point, length, new_string, set_code_point};

/// `sys.stdin`.
pub(super) static STDIN: TextFile =
    super::standard(crate::libc::STDIN, "<stdin>", READABLE | STANDARD);

/// What standard input has read and not given out yet.
static READER: ProgramCell<Reader> = ProgramCell::new(Reader::escaping());

/// Reads from standard input: the rest when `limit` is `None` and not `line`, otherwise up to
/// `limit` characters, ending after a `\n` when `line`.
pub(super) fn read(
    limit: Option<usize>,
    line: bool,
) -> Result<*mut StrObject, *mut ExceptionObject> {
    READER.with(|reader| read_from(reader, limit, line, &mut standard_input))
}

/// Returns the next line of standard input without its `\n`, or `None` at its end, as
/// `input()` reads it.
pub(crate) fn input_line() -> Result<Option<*mut StrObject>, *mut ExceptionObject> {
    if crate::io::standard_closed(crate::libc::STDIN) {
        return Err(super::disk::closed_error());
    }
    READER.with(|reader| line_from(reader, &mut standard_input))
}

/// Runs `action` with what standard input has read and not given out.
pub(super) fn with_reader<R>(action: impl FnOnce(&mut Reader) -> R) -> R {
    READER.with(action)
}

/// Reads from file descriptor 0.
fn standard_input(buffer: &mut [u8]) -> Result<usize, i32> {
    crate::libc::read_retrying(crate::libc::STDIN, buffer)
}

/// Reads from `source` through `reader`, as `read` does.
fn read_from(
    reader: &mut Reader,
    limit: Option<usize>,
    line: bool,
    source: Source<'_>,
) -> Result<*mut StrObject, *mut ExceptionObject> {
    if limit.is_none() && !line {
        reader.read_rest(source)
    } else {
        reader.read_text(limit, line, source)
    }
}

/// Returns the next line of `source` without its `\n`, or `None` at its end.
fn line_from(
    reader: &mut Reader,
    source: Source<'_>,
) -> Result<Option<*mut StrObject>, *mut ExceptionObject> {
    let line = read_from(reader, None, true, source)?;
    // SAFETY: the reader returned a live string, released here when a copy replaces it.
    unsafe {
        let count = length(line);
        if count == 0 {
            crate::refcount::typy_rt_decrement(line.cast());
            return Ok(None);
        }
        if code_point(line, count - 1) != u32::from(b'\n') {
            return Ok(Some(line));
        }
        let kept = new_string(count - 1, (*line).kind);
        for index in 0..count - 1 {
            set_code_point(kept, index, code_point(line, index));
        }
        crate::refcount::typy_rt_decrement(line.cast());
        Ok(Some(kept))
    }
}

/// Returns `sys.stdin`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_sys_stdin() -> *const TextFile {
    &raw const STDIN
}

#[cfg(test)]
mod tests;
