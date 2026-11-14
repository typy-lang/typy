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
    READER.with(|reader| line_from(reader, &mut standard_input))
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
mod tests {
    use super::{Reader, line_from};

    /// Returns the code points of each line `input()` gives from `chunks`, read one chunk at
    /// a time.
    fn lines_of(chunks: &[&[u8]]) -> Vec<Vec<u32>> {
        let mut reader = Reader::escaping();
        let mut next = chunks.iter();
        let mut source = |buffer: &mut [u8]| {
            let Some(chunk) = next.next() else {
                return Ok(0);
            };
            buffer[..chunk.len()].copy_from_slice(chunk);
            Ok(chunk.len())
        };
        let mut lines = Vec::new();
        while let Some(line) = line_from(&mut reader, &mut source).unwrap() {
            // SAFETY: the line is live, and released once it is copied.
            unsafe {
                lines.push(
                    (0..super::length(line))
                        .map(|index| super::code_point(line, index))
                        .collect(),
                );
                crate::refcount::typy_rt_decrement(line.cast());
            }
        }
        lines
    }

    #[test]
    fn input_gives_each_line_without_its_newline_until_the_end() {
        let cases: [(&[&[u8]], &[&str]); 7] = [
            (&[], &[]),
            (&[b"one\n"], &["one"]),
            (&[b"one\n\ntwo\r\n"], &["one", "", "two"]),
            (&[b"a\rb\r", b"\nc"], &["a", "b", "c"]),
            (&[b"last"], &["last"]),
            (
                &[b"sp", b"lit\nacross", b" chunks\n"],
                &["split", "across chunks"],
            ),
            (&[b"caf\xc3", b"\xa9\n"], &["caf\u{e9}"]),
        ];
        for (chunks, expected) in cases {
            let expected: Vec<Vec<u32>> = expected
                .iter()
                .map(|line| line.chars().map(u32::from).collect())
                .collect();
            assert_eq!(lines_of(chunks), expected, "{chunks:?}");
        }
    }

    #[test]
    fn a_byte_outside_utf8_is_escaped_as_cpython_escapes_it() {
        let lines = lines_of(&[b"a\xffb\xe9", b"\n\xe2\x82"]);
        assert_eq!(
            lines,
            [vec![0x61, 0xdcff, 0x62, 0xdce9], vec![0xdce2, 0xdc82]]
        );
    }

    #[test]
    fn input_reads_a_line_longer_than_a_chunk() {
        let long = [b'x'; super::super::reader::CHUNK];
        let lines = lines_of(&[&long, &long[..100], b"\n"]);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), super::super::reader::CHUNK + 100);
    }
}
