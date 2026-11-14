//! Tests of binary files (LIB-14): `BufferedReader`, `BufferedWriter`, `BufferedRandom` and
//! `FileIO`, against the results and messages of CPython 3.15.

use super::binary::{
    typy_rt_binary_read, typy_rt_binary_readline, typy_rt_binary_readlines, typy_rt_binary_write,
};
use super::position::{typy_rt_text_seek, typy_rt_text_tell, typy_rt_text_truncate};
use super::tests::{Scratch, error_text, repr_of, size};
use super::{TextFile, typy_rt_text_close, typy_rt_text_flush, typy_rt_text_writelines};
use crate::refcount::typy_rt_decrement;
use crate::string::{StrObject, code_point, length};

/// The access numbers `typy_rt_open` takes, with the bits of `+`, binary and `FileIO`.
const PLUS: i64 = 4;
const BINARY: i64 = 1 << 8;
const RAW: i64 = 1 << 9;

/// Returns the bytes of a `bytes` the runtime returned, and releases it.
fn bytes_of(result: crate::Raising<*mut StrObject>) -> Result<Vec<u8>, (String, String)> {
    if !result.exception.is_null() {
        return Err(error_text(result.exception));
    }
    // SAFETY: the bytes are live and owned here.
    unsafe {
        let found = (0..length(result.value))
            .map(|index| u8::try_from(code_point(result.value, index)).unwrap())
            .collect();
        typy_rt_decrement(result.value.cast());
        Ok(found)
    }
}

/// Returns the value of an `int` word the runtime returned, or the class and text of its
/// error.
fn number(result: crate::Raising<i64>) -> Result<i64, (String, String)> {
    if result.exception.is_null() {
        Ok(crate::word::inline_value(result.value))
    } else {
        Err(error_text(result.exception))
    }
}

fn read(file: *mut TextFile, count: i64) -> Result<Vec<u8>, (String, String)> {
    // SAFETY: the file is live.
    bytes_of(unsafe { typy_rt_binary_read(file, size(count)) })
}

fn readline(file: *mut TextFile, count: i64) -> Result<Vec<u8>, (String, String)> {
    // SAFETY: the file is live.
    bytes_of(unsafe { typy_rt_binary_readline(file, size(count)) })
}

fn write(file: *mut TextFile, data: &[u8]) -> Result<i64, (String, String)> {
    let value = crate::bytes::from_slice(data);
    // SAFETY: the file and the bytes are live; the bytes are released once.
    unsafe {
        let written = number(typy_rt_binary_write(file, value));
        typy_rt_decrement(value.cast());
        written
    }
}

fn seek(file: *mut TextFile, offset: i64, whence: i64) -> Result<i64, (String, String)> {
    // SAFETY: the file is live.
    number(unsafe { typy_rt_text_seek(file, size(offset), size(whence)) })
}

fn tell(file: *mut TextFile) -> Result<i64, (String, String)> {
    // SAFETY: the file is live.
    number(unsafe { typy_rt_text_tell(file) })
}

fn truncate(file: *mut TextFile, at: Option<i64>) -> Result<i64, (String, String)> {
    let (word, given) = at.map_or((size(0), size(0)), |at| (size(at), size(1)));
    // SAFETY: the file is live.
    number(unsafe { typy_rt_text_truncate(file, word, given) })
}

fn error(class: &str, text: &str) -> Result<i64, (String, String)> {
    Err((class.to_owned(), text.to_owned()))
}

fn release(file: *mut TextFile) {
    // SAFETY: the file is live and released once.
    unsafe { typy_rt_decrement(file.cast()) };
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn a_buffered_writer_writes_bytes_and_seeks_from_anywhere() {
    let scratch = Scratch::new("binary-writer");
    let file = scratch.open("w.bin", "wb", 1 | BINARY).unwrap();
    assert_eq!(
        repr_of(file),
        "<_io.BufferedWriter name='".to_owned()
            + &scratch.0.join("w.bin").display().to_string()
            + "'>"
    );
    assert_eq!(write(file, b"ab\r\ncd\ref\n\xff"), Ok(11));
    assert_eq!(tell(file), Ok(11));
    assert_eq!(seek(file, -2, 1), Ok(9));
    assert_eq!(seek(file, -1, 2), Ok(10));
    assert_eq!(seek(file, 0, 3), Ok(0));
    assert_eq!(
        seek(file, -100, 1),
        error("OSError", "[Errno 22] Invalid argument")
    );
    assert_eq!(
        seek(file, 0, 5),
        error("ValueError", "whence value 5 unsupported")
    );
    assert_eq!(
        read(file, -1).map(|_| 0),
        error("ioUnsupportedOperation", "read").map(|_| 0)
    );
    assert_eq!(truncate(file, Some(5)), Ok(5));
    // SAFETY: the file is live.
    unsafe { assert!(typy_rt_text_close(file).is_null()) };
    assert_eq!(
        write(file, b"x"),
        error("ValueError", "write to closed file")
    );
    // SAFETY: the file is live.
    let flushed = unsafe { typy_rt_text_flush(file) };
    assert_eq!(error_text(flushed).1, "flush of closed file");
    assert_eq!(seek(file, 0, 0), error("ValueError", "seek of closed file"));
    assert_eq!(
        seek(file, 0, 9),
        error("ValueError", "whence value 9 unsupported")
    );
    assert_eq!(
        tell(file),
        error("ValueError", "I/O operation on closed file")
    );
    assert_eq!(
        truncate(file, None),
        error("ValueError", "truncate of closed file")
    );
    release(file);
    assert_eq!(std::fs::read(scratch.0.join("w.bin")).unwrap(), b"ab\r\nc");
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn a_buffered_reader_reads_bytes_and_lines() {
    let scratch = Scratch::new("binary-reader");
    std::fs::write(scratch.0.join("r.bin"), b"12\r\nc").unwrap();
    let file = scratch.open("r.bin", "rb", BINARY).unwrap();
    assert!(repr_of(file).starts_with("<_io.BufferedReader name='"));
    assert_eq!(read(file, 2), Ok(b"12".to_vec()));
    assert_eq!(readline(file, -1), Ok(b"\r\n".to_vec()));
    assert_eq!(readline(file, 0), Ok(Vec::new()));
    assert_eq!(tell(file), Ok(4));
    assert_eq!(seek(file, -3, 2), Ok(2));
    // SAFETY: the file is live; the list is released once.
    unsafe {
        let lines = typy_rt_binary_readlines(file).value;
        assert_eq!((*lines).length, 2);
        typy_rt_decrement(lines.cast());
    }
    assert_eq!(read(file, -1), Ok(Vec::new()));
    assert_eq!(write(file, b"x"), error("ioUnsupportedOperation", "write"));
    assert_eq!(
        truncate(file, None),
        error("ioUnsupportedOperation", "truncate")
    );
    // SAFETY: the file is live.
    unsafe { assert!(typy_rt_text_close(file).is_null()) };
    let closed = |result: Result<Vec<u8>, (String, String)>| result.unwrap_err().1;
    assert_eq!(closed(read(file, -1)), "read of closed file");
    assert_eq!(closed(readline(file, -1)), "readline of closed file");
    // SAFETY: the file is live.
    let lines = unsafe { typy_rt_binary_readlines(file) };
    assert_eq!(
        error_text(lines.exception).1,
        "I/O operation on closed file."
    );
    // SAFETY: the file is live.
    let flushed = unsafe { typy_rt_text_flush(file) };
    assert_eq!(error_text(flushed).1, "I/O operation on closed file.");
    release(file);
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn a_buffered_random_file_writes_where_tell_says() {
    let scratch = Scratch::new("binary-random");
    std::fs::write(scratch.0.join("p.bin"), b"abcdef\nghi\n").unwrap();
    let file = scratch.open("p.bin", "rb+", PLUS | BINARY).unwrap();
    assert!(repr_of(file).starts_with("<_io.BufferedRandom name='"));
    assert_eq!(read(file, 2), Ok(b"ab".to_vec()));
    assert_eq!(write(file, b"X"), Ok(1));
    assert_eq!(tell(file), Ok(3));
    assert_eq!(read(file, -1), Ok(b"def\nghi\n".to_vec()));
    release(file);
    assert_eq!(
        std::fs::read(scratch.0.join("p.bin")).unwrap(),
        b"abXdef\nghi\n"
    );
    let lines = crate::list::new_list(crate::list::ItemKind::Object, 0);
    let item = crate::bytes::from_slice(b"Z");
    let file = scratch.open("p.bin", "ab+", 2 | PLUS | BINARY).unwrap();
    // SAFETY: the list, the bytes and the file are live; each is released once.
    unsafe {
        crate::list::typy_rt_list_append(lines, item as u64);
        assert!(typy_rt_text_writelines(file, lines).is_null());
        typy_rt_decrement(lines.cast());
        typy_rt_decrement(item.cast());
    }
    assert_eq!(tell(file), Ok(12));
    release(file);
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn a_file_io_reads_and_writes_the_descriptor_at_once() {
    let scratch = Scratch::new("binary-raw");
    std::fs::write(scratch.0.join("f.bin"), b"ab\ncd\n").unwrap();
    let file = scratch.open("f.bin", "rb", BINARY | RAW).unwrap();
    assert!(repr_of(file).ends_with("' mode='rb' closefd=True>"));
    assert_eq!(read(file, 1), Ok(b"a".to_vec()));
    assert_eq!(readline(file, -1), Ok(b"b\n".to_vec()));
    assert_eq!(tell(file), Ok(3));
    assert_eq!(read(file, -1), Ok(b"cd\n".to_vec()));
    assert_eq!(
        write(file, b"x"),
        error("ioUnsupportedOperation", "File not open for writing")
    );
    assert_eq!(
        truncate(file, None),
        error("ioUnsupportedOperation", "File not open for writing")
    );
    assert_eq!(
        seek(file, 0, 5),
        error("OSError", "[Errno 22] Invalid argument")
    );
    // SAFETY: the file is live.
    unsafe { assert!(typy_rt_text_close(file).is_null()) };
    assert_eq!(repr_of(file), "<_io.FileIO [closed]>");
    let closed = error("ValueError", "I/O operation on closed file");
    assert_eq!(read(file, -1).map(|_| 0), closed.clone());
    assert_eq!(write(file, b"x"), closed.clone());
    assert_eq!(seek(file, 0, 0), closed.clone());
    assert_eq!(truncate(file, None), closed);
    release(file);
    let file = scratch.open("f.bin", "wb", 1 | BINARY | RAW).unwrap();
    assert_eq!(write(file, b"xyz"), Ok(3));
    assert_eq!(std::fs::read(scratch.0.join("f.bin")).unwrap(), b"xyz");
    assert_eq!(
        read(file, -1).map(|_| 0),
        error("ioUnsupportedOperation", "File not open for reading")
    );
    assert_eq!(truncate(file, Some(1)), Ok(1));
    release(file);
}
