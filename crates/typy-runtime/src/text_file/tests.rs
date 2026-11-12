//! Tests of text files: the standard files, CPython's chunked reading and newline
//! translation, buffered writing, `OSError` for an error number, and files on disk.

use super::reader::{CHUNK, Reader};
use super::writer::{CAPACITY, Writer};
use super::{
    TEXT_FILE, TextFile, typy_rt_sys_stderr, typy_rt_sys_stdout, typy_rt_text_close,
    typy_rt_text_enter, typy_rt_text_flush, typy_rt_text_read, typy_rt_text_readline,
    typy_rt_text_readlines, typy_rt_text_write, typy_rt_text_writelines,
};
use crate::exception::ExceptionObject;
use crate::refcount::typy_rt_decrement;
use crate::string::{StrObject, code_point, from_str, length};

/// Returns the text of a `str` and releases it.
fn take(text: *mut StrObject) -> String {
    // SAFETY: the string is live and owned here.
    unsafe {
        let found = (0..length(text))
            .map(|index| char::from_u32(code_point(text, index)).unwrap_or('\u{fffd}'))
            .collect();
        typy_rt_decrement(text.cast());
        found
    }
}

/// Returns the class name and `str` of an exception, and releases it.
fn error_text(error: *mut ExceptionObject) -> (String, String) {
    assert!(!error.is_null());
    // SAFETY: the exception is live and owned here.
    unsafe {
        let class = (*error).class();
        let text = take(class.str.unwrap()(error.cast()).value);
        let name = format!("{}{}", class.module(), class.name());
        typy_rt_decrement(error.cast());
        (name, text)
    }
}

/// Returns the `repr` of an exception, and releases it.
fn error_repr(error: *mut ExceptionObject) -> String {
    // SAFETY: the exception is live and owned here.
    unsafe {
        let text = take((*error).class().repr.unwrap()(error.cast()).value);
        typy_rt_decrement(error.cast());
        text
    }
}

fn repr_of(file: *const TextFile) -> String {
    let repr = TEXT_FILE.repr.unwrap();
    // SAFETY: the file is a live text file.
    take(unsafe { repr(file.cast_mut().cast()).value })
}

#[test]
fn the_standard_files_show_as_cpython_shows_them() {
    assert_eq!(
        repr_of(typy_rt_sys_stdout()),
        "<_io.TextIOWrapper name='<stdout>' mode='w' encoding='utf-8'>"
    );
    assert_eq!(
        repr_of(typy_rt_sys_stderr()),
        "<_io.TextIOWrapper name='<stderr>' mode='w' encoding='utf-8'>"
    );
    assert_eq!(TEXT_FILE.name(), "TextIOWrapper");
}

#[test]
fn write_returns_the_number_of_code_points_for_each_standard_file() {
    let text = from_str("caf\u{e9}\n");
    let expected = crate::word::inline(5).unwrap();
    // SAFETY: the files and the text are live; the text is owned here.
    unsafe {
        for file in [
            typy_rt_sys_stdout(),
            typy_rt_sys_stderr(),
            core::ptr::null(),
        ] {
            let file = file.cast_mut();
            let written = typy_rt_text_write(file, text);
            assert!(written.exception.is_null());
            assert_eq!(written.value, expected);
            assert!(typy_rt_text_flush(file).is_null());
        }
        assert!(typy_rt_text_close(typy_rt_sys_stdout().cast_mut()).is_null());
        assert!(typy_rt_text_enter(typy_rt_sys_stderr().cast_mut()).is_null());
        typy_rt_decrement(text.cast());
    }
}

#[test]
fn reading_a_standard_file_is_not_supported() {
    let stdout = typy_rt_sys_stdout().cast_mut();
    let unsupported = (
        "ioUnsupportedOperation".to_owned(),
        "not readable".to_owned(),
    );
    // SAFETY: standard output is a live text file.
    unsafe {
        let size = crate::word::inline(-1).unwrap();
        assert_eq!(
            error_text(typy_rt_text_read(stdout, size).exception),
            unsupported
        );
        assert_eq!(
            error_text(typy_rt_text_readline(stdout, size).exception),
            unsupported
        );
        assert_eq!(
            error_text(typy_rt_text_readlines(stdout).exception),
            unsupported
        );
        assert_eq!(
            error_text(super::next_line(stdout).unwrap_err()),
            unsupported
        );
        let lines = crate::list::new_list(crate::list::ItemKind::Object, 0);
        assert!(typy_rt_text_writelines(stdout, lines).is_null());
        typy_rt_decrement(lines.cast());
    }
}

/// Reads `chunks` one at a time through `read`, with an error number after the last chunk
/// when `failing` is set.
fn source_of<'chunks>(
    chunks: &'chunks [&'chunks [u8]],
    failing: Option<i32>,
) -> impl FnMut(&mut [u8]) -> Result<usize, i32> + 'chunks {
    let mut next = chunks.iter();
    move |buffer| match next.next() {
        Some(chunk) => {
            assert!(chunk.len() <= buffer.len());
            buffer[..chunk.len()].copy_from_slice(chunk);
            Ok(chunk.len())
        }
        None => failing.map_or(Ok(0), Err),
    }
}

/// Returns each line `readline` gives until the end, or the error that stops it.
fn lines_of(chunks: &[&[u8]]) -> Result<Vec<String>, (String, String)> {
    let mut reader = Box::new(Reader::new());
    let mut source = source_of(chunks, None);
    let mut lines = Vec::new();
    loop {
        match reader.read_text(None, true, &mut source) {
            Ok(line) => {
                let line = take(line);
                if line.is_empty() {
                    return Ok(lines);
                }
                lines.push(line);
            }
            Err(error) => return Err(error_text(error)),
        }
    }
}

fn owned(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| (*line).to_owned()).collect()
}

#[test]
fn lines_end_after_each_newline_with_crlf_and_cr_read_as_newline() {
    let cases: [(&[&[u8]], &[&str]); 7] = [
        (&[], &[]),
        (&[b"a\r\nb\rc\nd"], &["a\n", "b\n", "c\n", "d"]),
        (&[b"a\r", b"\nb"], &["a\n", "b"]),
        (&[b"a\r", b"b"], &["a\n", "b"]),
        (&[b"a\r"], &["a\n"]),
        (&[b"\r\r\n\n"], &["\n", "\n", "\n"]),
        (
            &[b"caf\xc3", b"\xa9\n\xe2\x82", b"\xac"],
            &["caf\u{e9}\n", "\u{20ac}"],
        ),
    ];
    for (chunks, expected) in cases {
        assert_eq!(lines_of(chunks), Ok(owned(expected)), "{chunks:?}");
    }
}

#[test]
fn an_invalid_byte_stops_the_line_of_its_chunk_with_its_position_in_the_chunk() {
    let decode = |text: &str| Err(("UnicodeDecodeError".to_owned(), text.to_owned()));
    assert_eq!(
        lines_of(&[b"ok\n\xffz\n"]),
        decode("'utf-8' codec can't decode byte 0xff in position 3: invalid start byte")
    );
    assert_eq!(
        lines_of(&[b"ab\xc3", b"(\n"]),
        decode("'utf-8' codec can't decode byte 0xc3 in position 0: invalid continuation byte")
    );
    assert_eq!(
        lines_of(&[b"ab\xe2\x82"]),
        decode("'utf-8' codec can't decode bytes in position 0-1: unexpected end of data")
    );
    let mut reader = Box::new(Reader::new());
    let mut source = source_of(&[b"ok\n", b"\xff\n", b"after\n"], None);
    assert_eq!(
        take(reader.read_text(None, true, &mut source).unwrap()),
        "ok\n"
    );
    assert!(reader.read_text(None, true, &mut source).is_err());
    assert_eq!(
        take(reader.read_text(None, true, &mut source).unwrap()),
        "after\n"
    );
}

#[test]
fn a_limit_counts_characters_after_translation() {
    let mut reader = Box::new(Reader::new());
    let mut source = source_of(&[b"a\r\nbc\xc3\xa9d"], None);
    let mut read = |limit: Option<usize>, line: bool| {
        take(reader.read_text(limit, line, &mut source).unwrap())
    };
    assert_eq!(read(Some(0), false), "");
    assert_eq!(read(Some(2), false), "a\n");
    assert_eq!(read(Some(1), true), "b");
    assert_eq!(read(Some(9), false), "c\u{e9}d");
    assert_eq!(read(Some(9), false), "");
}

#[test]
fn read_decodes_the_rest_at_once_with_positions_from_the_first_byte_not_decoded() {
    let mut reader = Box::new(Reader::new());
    let mut source = source_of(&[b"x\r", b"\ny\r", b"\nz"], None);
    assert_eq!(
        take(reader.read_text(Some(1), false, &mut source).unwrap()),
        "x"
    );
    assert_eq!(take(reader.read_rest(&mut source).unwrap()), "\ny\nz");
    assert_eq!(take(reader.read_rest(&mut source).unwrap()), "");

    let mut reader = Box::new(Reader::new());
    let long = [b'x'; CHUNK];
    let chunks: [&[u8]; 3] = [&long, &long, b"\xff"];
    let mut source = source_of(&chunks, None);
    let error = reader.read_rest(&mut source).unwrap_err();
    assert_eq!(
        error_text(error).1,
        "'utf-8' codec can't decode byte 0xff in position 16384: invalid start byte"
    );
}

#[test]
fn an_error_of_reading_is_an_os_error() {
    let mut reader = Box::new(Reader::new());
    let mut source = source_of(&[], Some(5));
    let error = reader.read_text(None, true, &mut source).unwrap_err();
    assert_eq!(
        error_text(error),
        (
            "OSError".to_owned(),
            "[Errno 5] Input/output error".to_owned()
        )
    );
    let mut source = source_of(&[], Some(5));
    assert!(reader.read_rest(&mut source).is_err());
}

#[test]
fn a_write_waits_in_the_buffer_until_it_fills_or_is_flushed() {
    let mut writer = Box::new(Writer::new());
    let mut written: Vec<Vec<u8>> = Vec::new();
    let mut sink = |bytes: &[u8]| {
        written.push(bytes.to_vec());
        Ok(())
    };
    let text = from_str("caf\u{e9}\n");
    let long = from_str(&"y".repeat(CAPACITY));
    // SAFETY: the strings are live and owned here.
    unsafe {
        writer.write(text, &mut sink).unwrap();
        writer.write(long, &mut sink).unwrap();
        writer.flush(&mut sink).unwrap();
        typy_rt_decrement(text.cast());
        typy_rt_decrement(long.cast());
    }
    let all: Vec<u8> = written.concat();
    assert_eq!(&all[..6], "caf\u{e9}\n".as_bytes());
    assert_eq!(all.len(), 6 + CAPACITY);
    assert_eq!(written.len(), 2);
}

#[test]
fn a_surrogate_is_refused_before_anything_is_written() {
    let mut writer = Box::new(Writer::new());
    let mut sink = |_: &[u8]| Ok(());
    let text = crate::string::new_string(4, crate::string::StrKind::Ucs2);
    for (index, point) in [0x61, 0xd800, 0xdc00, 0x62].into_iter().enumerate() {
        // SAFETY: the string is new with four code points.
        unsafe { crate::string::set_code_point(text, index, point) };
    }
    // SAFETY: the string is live and owned here.
    let error = unsafe { writer.write(text, &mut sink).unwrap_err() };
    assert_eq!(
        error_text(error).1,
        "'utf-8' codec can't encode characters in position 1-2: surrogates not allowed"
    );
    // SAFETY: the string is owned here.
    unsafe { typy_rt_decrement(text.cast()) };
    let mut failing = |_: &[u8]| Err(28);
    assert_eq!(
        error_text(writer.flush(&mut failing).unwrap_err()).1,
        "[Errno 28] No space left on device"
    );
}

#[test]
fn an_error_number_gives_cpythons_class_text_and_arguments() {
    let path = from_str("missing.txt");
    let cases = [
        (
            2,
            "FileNotFoundError",
            "[Errno 2] No such file or directory: 'missing.txt'",
        ),
        (
            13,
            "PermissionError",
            "[Errno 13] Permission denied: 'missing.txt'",
        ),
        (
            1,
            "PermissionError",
            "[Errno 1] Operation not permitted: 'missing.txt'",
        ),
        (
            17,
            "FileExistsError",
            "[Errno 17] File exists: 'missing.txt'",
        ),
        (
            20,
            "NotADirectoryError",
            "[Errno 20] Not a directory: 'missing.txt'",
        ),
        (
            21,
            "IsADirectoryError",
            "[Errno 21] Is a directory: 'missing.txt'",
        ),
        (
            32,
            "BrokenPipeError",
            "[Errno 32] Broken pipe: 'missing.txt'",
        ),
        (
            36,
            "OSError",
            "[Errno 36] File name too long: 'missing.txt'",
        ),
        (41, "OSError", "[Errno 41] Unknown error 41: 'missing.txt'"),
        (
            500,
            "OSError",
            "[Errno 500] Unknown error 500: 'missing.txt'",
        ),
    ];
    for (errno, class, text) in cases {
        // SAFETY: the path is live.
        let error = unsafe { super::os_error::os_error(errno, path) };
        assert_eq!(error_text(error), (class.to_owned(), text.to_owned()));
    }
    for errno in [3, 4, 10, 11, 103, 104, 110, 111, 114, 115, 108] {
        let class = super::os_error::class_of(errno);
        assert!(
            crate::exception::classes::derives_from(class, &crate::exception::classes::OS_ERROR),
            "{errno}"
        );
    }
    // SAFETY: the path is live.
    let error = unsafe { super::os_error::os_error(2, path) };
    assert_eq!(
        error_repr(error),
        "FileNotFoundError(2, 'No such file or directory')"
    );
    // SAFETY: the path is owned here.
    unsafe { typy_rt_decrement(path.cast()) };
}

/// A directory of its own for a test of files on disk, removed at the end.
struct Scratch(std::path::PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let directory =
            std::env::temp_dir().join(format!("typy-text-file-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        Self(directory)
    }

    fn path(&self, name: &str) -> *mut StrObject {
        from_str(self.0.join(name).to_str().unwrap())
    }

    fn open(&self, name: &str, mode: &str, access: i64) -> Result<*mut TextFile, (String, String)> {
        let path = self.path(name);
        let mode = from_str(mode);
        // SAFETY: the strings are live; the file keeps references of its own.
        let opened =
            unsafe { super::disk::typy_rt_open(path, mode, crate::word::inline(access).unwrap()) };
        // SAFETY: the strings are owned here.
        unsafe {
            typy_rt_decrement(path.cast());
            typy_rt_decrement(mode.cast());
        }
        if opened.exception.is_null() {
            Ok(opened.value)
        } else {
            Err(error_text(opened.exception))
        }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn size(value: i64) -> i64 {
    crate::word::inline(value).unwrap()
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn a_file_written_then_read_gives_its_text_back() {
    let scratch = Scratch::new("round");
    let text = from_str("one\r\ntwo\n");
    // SAFETY: the files and strings are live; each is released once.
    unsafe {
        let file = scratch.open("t.txt", "w", 1).unwrap();
        assert!(repr_of(file).ends_with("t.txt' mode='w' encoding='utf-8'>"));
        assert_eq!(typy_rt_text_write(file, text).value, size(9));
        let lines = crate::list::new_list(crate::list::ItemKind::Object, 2);
        crate::list::typy_rt_list_append(lines, text as u64);
        assert!(typy_rt_text_writelines(file, lines).is_null());
        typy_rt_decrement(lines.cast());
        let unsupported = error_text(typy_rt_text_read(file, size(-1)).exception);
        assert_eq!(unsupported.1, "not readable");
        assert!(typy_rt_text_close(file).is_null());
        assert!(typy_rt_text_close(file).is_null());
        let closed = (
            "ValueError".to_owned(),
            "I/O operation on closed file.".to_owned(),
        );
        assert_eq!(error_text(typy_rt_text_write(file, text).exception), closed);
        assert_eq!(error_text(typy_rt_text_flush(file)), closed);
        assert_eq!(error_text(typy_rt_text_enter(file)), closed);
        assert_eq!(error_text(typy_rt_text_writelines(file, lines)), closed);
        typy_rt_decrement(file.cast());

        let file = scratch.open("t.txt", "r", 0).unwrap();
        assert!(typy_rt_text_enter(file).is_null());
        assert_eq!(take(typy_rt_text_readline(file, size(-1)).value), "one\n");
        assert_eq!(take(typy_rt_text_read(file, size(2)).value), "tw");
        let rest = typy_rt_text_readlines(file).value;
        assert_eq!((*rest).length, 3);
        typy_rt_decrement(rest.cast());
        assert_eq!(take(typy_rt_text_read(file, size(-1)).value), "");
        assert!(typy_rt_text_flush(file).is_null());
        assert_eq!(
            error_text(typy_rt_text_write(file, text).exception).1,
            "not writable"
        );
        typy_rt_decrement(file.cast());
        typy_rt_decrement(text.cast());
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn opening_gives_cpythons_errors() {
    let scratch = Scratch::new("errors");
    std::fs::create_dir(scratch.0.join("d")).unwrap();
    std::fs::write(scratch.0.join("there"), "x").unwrap();
    let missing = scratch.open("missing", "r", 0).unwrap_err();
    assert_eq!(missing.0, "FileNotFoundError");
    assert!(
        missing
            .1
            .starts_with("[Errno 2] No such file or directory: '")
    );
    assert_eq!(
        scratch.open("d", "r", 0).unwrap_err().0,
        "IsADirectoryError"
    );
    assert_eq!(
        scratch.open("d", "w", 1).unwrap_err().0,
        "IsADirectoryError"
    );
    assert_eq!(
        scratch.open("there", "x", 3).unwrap_err().0,
        "FileExistsError"
    );
    assert_eq!(
        scratch.open("there/x", "r", 0).unwrap_err().0,
        "NotADirectoryError"
    );
    let null = scratch.open("a\0b", "r", 0).unwrap_err();
    assert_eq!(
        null,
        ("ValueError".to_owned(), "embedded null byte".to_owned())
    );
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn appending_and_closing_at_the_end_keep_what_was_written() {
    let scratch = Scratch::new("append");
    std::fs::write(scratch.0.join("a.txt"), "start\n").unwrap();
    let text = from_str("more");
    // SAFETY: the file and the text are live; each is released once.
    unsafe {
        let file = scratch.open("a.txt", "a", 2).unwrap();
        typy_rt_text_write(file, text);
        assert_eq!(
            std::fs::read_to_string(scratch.0.join("a.txt")).unwrap(),
            "start\n"
        );
        super::close_all();
        assert_eq!(
            std::fs::read_to_string(scratch.0.join("a.txt")).unwrap(),
            "start\nmore"
        );
        typy_rt_decrement(file.cast());
        let file = scratch.open("b.txt", "w", 1).unwrap();
        typy_rt_text_write(file, text);
        typy_rt_decrement(file.cast());
        assert_eq!(
            std::fs::read_to_string(scratch.0.join("b.txt")).unwrap(),
            "more"
        );
        typy_rt_decrement(text.cast());
    }
}

#[test]
fn the_end_of_a_program_that_opened_no_file_closes_nothing() {
    // Each test runs in a process of its own, so no file has installed the hooks here.
    super::close_all();
    let mut header = crate::header::ObjectHeader::new(crate::header::TEXT_FILE_TYPE_ID);
    // SAFETY: without the hooks, nothing reads the object.
    unsafe { super::destroy(&raw mut header) };
}

/// Returns the `repr` of an iterator.
///
/// # Safety
///
/// `iterator` points to a live iterator.
unsafe fn repr_of_iterator(iterator: *mut crate::iterator::IteratorObject) -> *mut StrObject {
    // SAFETY: the caller's guarantee.
    unsafe {
        let ty = crate::iterator::type_of(iterator);
        ty.repr.unwrap()(iterator.cast()).value
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn iterating_over_a_file_gives_its_lines_and_shows_the_file() {
    let scratch = Scratch::new("iterate");
    std::fs::write(scratch.0.join("lines.txt"), "one\ntwo\n").unwrap();
    let file = scratch.open("lines.txt", "r", 0).unwrap();
    // SAFETY: the file and the iterator are live; each is released once.
    unsafe {
        let iterator = crate::iterator::typy_rt_iterable_iterator(file.cast(), core::ptr::null());
        let shown = take(repr_of_iterator(iterator));
        assert!(shown.starts_with("<_io.TextIOWrapper name='"), "{shown}");
        let mut lines = Vec::new();
        while crate::iterator::typy_rt_iterator_advance(iterator, crate::word::inline(0).unwrap())
            .value
        {
            let line = crate::iterator::typy_rt_iterator_item(iterator) as *mut StrObject;
            crate::refcount::typy_rt_increment(line.cast());
            lines.push(take(line));
        }
        assert_eq!(lines, ["one\n", "two\n"]);
        assert_eq!(take(repr_of_iterator(iterator)), repr_of(file));
        assert_eq!(
            crate::iterator::type_of(iterator).module(),
            crate::text_file::TEXT_FILE.module()
        );
        typy_rt_decrement(iterator.cast());
        typy_rt_decrement(file.cast());
    }
}

#[test]
fn iterating_over_standard_output_is_not_supported() {
    // SAFETY: standard output is a live text file; the iterator is released once.
    unsafe {
        let stdout = typy_rt_sys_stdout().cast_mut();
        let iterator = crate::iterator::typy_rt_iterable_iterator(stdout.cast(), core::ptr::null());
        let stepped =
            crate::iterator::typy_rt_iterator_advance(iterator, crate::word::inline(0).unwrap());
        assert_eq!(error_text(stepped.exception).1, "not readable");
        typy_rt_decrement(iterator.cast());
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn a_path_is_encoded_as_cpython_encodes_it() {
    let scratch = Scratch::new("paths");
    let mut escaped = String::from("caf\u{e9}-");
    escaped.push(char::from_u32(0x2603).unwrap());
    // SAFETY: the files are live; each is released once.
    unsafe {
        let file = scratch.open(&escaped, "w", 1).unwrap();
        typy_rt_decrement(file.cast());
    }
    assert!(scratch.0.join(&escaped).exists());
    let path = crate::string::new_string(3, crate::string::StrKind::Ucs2);
    for (index, point) in [0x61, 0xdcff, 0xd800].into_iter().enumerate() {
        // SAFETY: the string is new with three code points.
        unsafe { crate::string::set_code_point(path, index, point) };
    }
    let mode = from_str("w");
    // SAFETY: the strings are live; each is released once.
    unsafe {
        let opened = super::disk::typy_rt_open(path, mode, crate::word::inline(1).unwrap());
        assert_eq!(
            error_text(opened.exception).1,
            "'utf-8' codec can't encode character '\\ud800' in position 2: surrogates not allowed"
        );
        typy_rt_decrement(path.cast());
        typy_rt_decrement(mode.cast());
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn writelines_stops_at_the_first_line_that_fails() {
    let scratch = Scratch::new("writelines");
    let file = scratch.open("w.txt", "w", 1).unwrap();
    let lines = crate::list::new_list(crate::list::ItemKind::Object, 2);
    let surrogate = crate::string::new_string(1, crate::string::StrKind::Ucs2);
    let good = from_str("kept\n");
    // SAFETY: the objects are live; each is released once.
    unsafe {
        crate::string::set_code_point(surrogate, 0, 0xdc80);
        crate::list::typy_rt_list_append(lines, good as u64);
        crate::list::typy_rt_list_append(lines, surrogate as u64);
        let error = typy_rt_text_writelines(file, lines);
        assert_eq!(error_text(error).0, "UnicodeEncodeError");
        assert!(typy_rt_text_close(file).is_null());
        for object in [lines.cast(), surrogate.cast(), good.cast(), file.cast()] {
            typy_rt_decrement(object);
        }
    }
    assert_eq!(
        std::fs::read_to_string(scratch.0.join("w.txt")).unwrap(),
        "kept\n"
    );
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn files_close_in_any_order_and_the_rest_close_at_the_end() {
    let scratch = Scratch::new("order");
    let text = from_str("x");
    // SAFETY: the files and the text are live; each is released once.
    unsafe {
        let first = scratch.open("first.txt", "w", 1).unwrap();
        let middle = scratch.open("middle.txt", "w", 1).unwrap();
        let last = scratch.open("last.txt", "w", 1).unwrap();
        for file in [first, middle, last] {
            typy_rt_text_write(file, text);
        }
        assert!(typy_rt_text_close(middle).is_null());
        let closed = (
            "ValueError".to_owned(),
            "I/O operation on closed file.".to_owned(),
        );
        assert_eq!(
            error_text(typy_rt_text_read(middle, size(-1)).exception),
            closed
        );
        super::close_all();
        for name in ["first.txt", "middle.txt", "last.txt"] {
            assert_eq!(
                std::fs::read_to_string(scratch.0.join(name)).unwrap(),
                "x",
                "{name}"
            );
        }
        for object in [first.cast(), middle.cast(), last.cast(), text.cast()] {
            typy_rt_decrement(object);
        }
    }
}
