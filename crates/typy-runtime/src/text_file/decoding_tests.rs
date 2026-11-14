//! Tests of the arguments of `open` after its mode (LIB-14): codecs, error handlers, newline
//! modes and line buffering, against the results and messages of CPython 3.15.

use super::newline::Newline;
use super::reader::{Decoding, Reader};
use super::tests::{OpenOptions, Scratch, error_text, size, source_of, take};
use super::writer::{Encoding, Writer};
use super::{typy_rt_text_close, typy_rt_text_read, typy_rt_text_write};
use crate::codec::{Codec, Handler};
use crate::refcount::typy_rt_decrement;
use crate::string::from_str;

/// Returns what a reader of `decoding` gives for `calls`, each a size and whether it reads a
/// line, from `chunks`, or the class and text of the error that stops it.
fn reads(
    decoding: Decoding,
    chunks: &[&[u8]],
    calls: &[(Option<usize>, bool)],
) -> Result<Vec<String>, (String, String)> {
    let mut reader = Box::new(Reader::with(decoding));
    let mut source = source_of(chunks, None);
    calls
        .iter()
        .map(|(limit, line)| {
            reader
                .read_text(*limit, *line, &mut source)
                .map(take)
                .map_err(error_text)
        })
        .collect()
}

/// Returns what `read()` of a reader of `decoding` gives for `chunks`.
fn read_all(decoding: Decoding, chunks: &[&[u8]]) -> Result<String, (String, String)> {
    let mut reader = Box::new(Reader::with(decoding));
    let mut source = source_of(chunks, None);
    reader.read_rest(&mut source).map(take).map_err(error_text)
}

fn decoding(codec: Codec, handler: Handler, newline: Newline) -> Decoding {
    Decoding {
        codec,
        handler,
        newline,
    }
}

fn texts(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| (*line).to_owned()).collect()
}

/// A newline mode, and the four reads of each of `readline()`, `readline(3)` and `read(2)`.
type NewlineCase = (
    Newline,
    [&'static str; 4],
    [&'static str; 4],
    [&'static str; 4],
);

const LINES: [(Option<usize>, bool); 4] = [(None, true); 4];
const LINES_OF_THREE: [(Option<usize>, bool); 4] = [(Some(3), true); 4];
const PAIRS: [(Option<usize>, bool); 4] = [(Some(2), false); 4];

#[test]
fn each_newline_mode_ends_and_translates_lines_as_cpython_does() {
    let data: &[&[u8]] = &[b"ab\r\r\ncd\r"];
    let cases: [NewlineCase; 5] = [
        (
            Newline::Universal,
            ["ab\n", "\n", "cd\n", ""],
            ["ab\n", "\n", "cd\n", ""],
            ["ab", "\n\n", "cd", "\n"],
        ),
        (
            Newline::Untranslated,
            ["ab\r", "\r\n", "cd\r", ""],
            ["ab\r", "\r\n", "cd\r", ""],
            ["ab", "\r\r", "\nc", "d\r"],
        ),
        (
            Newline::CrLf,
            ["ab\r\r\n", "cd\r", "", ""],
            ["ab\r", "\r\n", "cd\r", ""],
            ["ab", "\r\r", "\nc", "d\r"],
        ),
        (
            Newline::Cr,
            ["ab\r", "\r", "\ncd\r", ""],
            ["ab\r", "\r", "\ncd", "\r"],
            ["ab", "\r\r", "\nc", "d\r"],
        ),
        (
            Newline::Lf,
            ["ab\r\r\n", "cd\r", "", ""],
            ["ab\r", "\r\n", "cd\r", ""],
            ["ab", "\r\r", "\nc", "d\r"],
        ),
    ];
    for (newline, lines, short_lines, pairs) in cases {
        let decoding = decoding(Codec::Utf8, Handler::Strict, newline);
        assert_eq!(
            reads(decoding, data, &LINES),
            Ok(texts(&lines)),
            "{newline:?}"
        );
        assert_eq!(
            reads(decoding, data, &LINES_OF_THREE),
            Ok(texts(&short_lines)),
            "{newline:?}"
        );
        assert_eq!(
            reads(decoding, data, &PAIRS),
            Ok(texts(&pairs)),
            "{newline:?}"
        );
    }
}

#[test]
fn a_carriage_return_at_the_end_of_a_chunk_waits_for_the_next() {
    let split: &[&[u8]] = &[b"ab\r", b"\ncd"];
    for (newline, first) in [
        (Newline::Universal, "ab\n"),
        (Newline::Untranslated, "ab\r\n"),
        (Newline::CrLf, "ab\r\n"),
    ] {
        let decoding = decoding(Codec::Utf8, Handler::Strict, newline);
        let found = reads(decoding, split, &[(None, true), (None, true)]);
        assert_eq!(found, Ok(texts(&[first, "cd"])), "{newline:?}");
    }
    let decoding = decoding(Codec::Utf8, Handler::Strict, Newline::CrLf);
    let cut = reads(decoding, &[b"\r\nx"], &[(Some(1), true), (None, true)]);
    assert_eq!(cut, Ok(texts(&["\r", "\nx"])));
}

#[test]
fn read_translates_only_universal_newlines() {
    let data: &[&[u8]] = &[b"a\r\nb\rc"];
    let universal = decoding(Codec::Utf8, Handler::Strict, Newline::Universal);
    assert_eq!(read_all(universal, data), Ok("a\nb\nc".to_owned()));
    for newline in [
        Newline::Untranslated,
        Newline::Lf,
        Newline::Cr,
        Newline::CrLf,
    ] {
        let decoding = decoding(Codec::Utf8, Handler::Strict, newline);
        assert_eq!(read_all(decoding, data), Ok("a\r\nb\rc".to_owned()));
    }
}

#[test]
fn each_handler_decodes_what_the_codec_cannot() {
    let data: &[&[u8]] = &[b"a\xff\xfeb\xe2\x82"];
    let cases = [
        (Handler::Replace, "a\u{fffd}\u{fffd}b\u{fffd}"),
        (Handler::Ignore, "ab"),
        (Handler::BackslashReplace, "a\\xff\\xfeb\\xe2\\x82"),
        // `take` shows each of U+DCFF, U+DCFE, U+DCE2 and U+DC82 as U+FFFD.
        (
            Handler::SurrogateEscape,
            "a\u{fffd}\u{fffd}b\u{fffd}\u{fffd}",
        ),
    ];
    for (handler, expected) in cases {
        let decoding = decoding(Codec::Utf8, handler, Newline::Universal);
        let found = reads(decoding, data, &[(None, false)]).map(|texts| texts[0].clone());
        assert_eq!(found, Ok(expected.to_owned()), "{handler:?}");
        assert_eq!(
            read_all(decoding, data),
            Ok(expected.to_owned()),
            "{handler:?}"
        );
    }
}

#[test]
fn ascii_and_latin_1_decode_one_byte_at_a_time() {
    let data: &[&[u8]] = &[b"a\xe9b"];
    let latin = decoding(Codec::Latin1, Handler::Strict, Newline::Universal);
    assert_eq!(read_all(latin, data), Ok("a\u{e9}b".to_owned()));
    let replaced = decoding(Codec::Ascii, Handler::Replace, Newline::Universal);
    assert_eq!(read_all(replaced, data), Ok("a\u{fffd}b".to_owned()));
    let strict = decoding(Codec::Ascii, Handler::Strict, Newline::Universal);
    let error = (
        "UnicodeDecodeError".to_owned(),
        "'ascii' codec can't decode byte 0xe9 in position 1: ordinal not in range(128)".to_owned(),
    );
    assert_eq!(read_all(strict, data), Err(error.clone()));
    assert_eq!(reads(strict, data, &[(Some(1), false)]), Err(error));
}

#[test]
fn xmlcharrefreplace_cannot_decode() {
    let xml = decoding(Codec::Utf8, Handler::XmlCharRefReplace, Newline::Universal);
    let error = (
        "TypeError".to_owned(),
        "don't know how to handle UnicodeDecodeError in error callback".to_owned(),
    );
    assert_eq!(read_all(xml, &[b"a\xff"]), Err(error.clone()));
    assert_eq!(reads(xml, &[b"a\xff"], &[(None, true)]), Err(error));
    assert_eq!(read_all(xml, &[b"ok"]), Ok("ok".to_owned()));
}

#[test]
fn a_size_that_cuts_an_escape_leaves_the_rest_for_the_next_read() {
    let escaping = decoding(Codec::Ascii, Handler::BackslashReplace, Newline::Universal);
    let calls = [(Some(3), false), (Some(2), false), (None, false)];
    let found = reads(escaping, &[b"a\xffb"], &calls);
    assert_eq!(found, Ok(texts(&["a\\x", "ff", "b"])));
    let calls = [
        (Some(2), false),
        (Some(1), false),
        (Some(1), false),
        (None, false),
    ];
    let found = reads(escaping, &[b"a\xffb"], &calls);
    assert_eq!(found, Ok(texts(&["a\\", "x", "f", "fb"])));
    let mut reader = Box::new(Reader::with(escaping));
    let mut source = source_of(&[b"\xff"], None);
    assert_eq!(
        take(reader.read_text(Some(1), false, &mut source).unwrap()),
        "\\"
    );
    assert_eq!(take(reader.read_rest(&mut source).unwrap()), "xff");
}

/// Returns the bytes a writer of `encoding` gives for `writes`, and whether each write left
/// the buffer empty, or the error of the first write that fails.
fn written(options: Encoding, writes: &[&str]) -> Result<(Vec<u8>, Vec<bool>), (String, String)> {
    let mut file = Box::new(Writer::with(options));
    let mut out = Vec::new();
    let mut emptied = Vec::new();
    for text in writes {
        let value = from_str(text);
        let before = out.len();
        let mut sink = |bytes: &[u8]| {
            out.extend_from_slice(bytes);
            Ok(())
        };
        // SAFETY: the text is live and released once.
        let result = unsafe { file.write(value, &mut sink) };
        // SAFETY: as above.
        unsafe { typy_rt_decrement(value.cast()) };
        result.map_err(error_text)?;
        emptied.push(out.len() > before);
    }
    let mut sink = |bytes: &[u8]| {
        out.extend_from_slice(bytes);
        Ok(())
    };
    file.flush(&mut sink).map_err(error_text)?;
    Ok((out, emptied))
}

fn encoding(codec: Codec, handler: Handler, newline: Newline, line_buffering: bool) -> Encoding {
    Encoding {
        codec,
        handler,
        newline,
        line_buffering,
    }
}

#[test]
fn writing_translates_then_encodes_with_the_handler() {
    let crlf = encoding(Codec::Ascii, Handler::Replace, Newline::CrLf, false);
    assert_eq!(
        written(crlf, &["a\n\u{e9}"]).map(|found| found.0),
        Ok(b"a\r\n?".to_vec())
    );
    let strict = encoding(Codec::Ascii, Handler::Strict, Newline::CrLf, false);
    assert_eq!(
        written(strict, &["a\n\u{e9}"]),
        Err((
            "UnicodeEncodeError".to_owned(),
            "'ascii' codec can't encode character '\\xe9' in position 3: ordinal not in \
             range(128)"
                .to_owned()
        ))
    );
    let xml = encoding(
        Codec::Latin1,
        Handler::XmlCharRefReplace,
        Newline::Universal,
        false,
    );
    assert_eq!(
        written(xml, &["\u{e9}\u{20ac}"]).map(|found| found.0),
        Ok(b"\xe9&#8364;".to_vec())
    );
}

#[test]
fn a_line_buffered_file_writes_out_each_line_ending() {
    let lines = encoding(Codec::Utf8, Handler::Strict, Newline::Universal, true);
    let found = written(lines, &["ab", "c\nd", "e\r", "f"]);
    assert_eq!(
        found,
        Ok((b"abc\nde\rf".to_vec(), vec![false, true, true, false]))
    );
    let full = encoding(Codec::Utf8, Handler::Strict, Newline::Universal, false);
    let found = written(full, &["ab", "c\nd"]);
    assert_eq!(found, Ok((b"abc\nd".to_vec(), vec![false, false])));
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn open_checks_buffering_and_newline_as_cpython_does() {
    let scratch = Scratch::new("options");
    let unbuffered = OpenOptions {
        buffering: 0,
        ..OpenOptions::DEFAULT
    };
    assert_eq!(
        scratch.open_with("zero.txt", "w", 1, &unbuffered),
        Err((
            "ValueError".to_owned(),
            "can't have unbuffered text I/O".to_owned()
        ))
    );
    // CPython opens the file before it checks, so the file is made.
    assert!(scratch.0.join("zero.txt").exists());
    let illegal = OpenOptions {
        newline: Some("x"),
        ..OpenOptions::DEFAULT
    };
    assert_eq!(
        scratch.open_with("newline.txt", "w", 1, &illegal),
        Err((
            "ValueError".to_owned(),
            "illegal newline value: x".to_owned()
        ))
    );
    assert!(scratch.0.join("newline.txt").exists());
    let too_large = Err((
        "OverflowError".to_owned(),
        "Python int too large to convert to C int".to_owned(),
    ));
    for buffering in [1 << 40, 1 << 70] {
        let options = OpenOptions {
            buffering,
            ..OpenOptions::DEFAULT
        };
        assert_eq!(scratch.open_with("big.txt", "w", 1, &options), too_large);
    }
    assert!(!scratch.0.join("big.txt").exists());
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn a_file_with_options_shows_its_encoding_and_reads_back_what_it_wrote() {
    let scratch = Scratch::new("encoded");
    let options = OpenOptions {
        buffering: 1,
        encoding: Some("Latin-1"),
        codec: 2,
        handler: 0,
        newline: Some("\r\n"),
    };
    let file = scratch.open_with("e.txt", "w", 1, &options).unwrap();
    let text = from_str("\u{e9}\n");
    // SAFETY: the file and the text are live; each is released once.
    unsafe {
        assert!(super::tests::repr_of(file).ends_with("mode='w' encoding='Latin-1'>"));
        assert_eq!(typy_rt_text_write(file, text).value, size(2));
        assert_eq!(std::fs::read(scratch.0.join("e.txt")).unwrap(), b"\xe9\r\n");
        assert!(typy_rt_text_close(file).is_null());
        typy_rt_decrement(file.cast());
        typy_rt_decrement(text.cast());
    }
    let reading = OpenOptions {
        encoding: Some("latin-1"),
        codec: 2,
        newline: Some(""),
        ..OpenOptions::DEFAULT
    };
    let file = scratch.open_with("e.txt", "r", 0, &reading).unwrap();
    // SAFETY: the file is live and released once.
    unsafe {
        assert_eq!(take(typy_rt_text_read(file, size(-1)).value), "\u{e9}\r\n");
        typy_rt_decrement(file.cast());
    }
}

/// Returns the class and text of the error of opening `name` with a mode value, or the mode
/// the file shows.
fn opened_with_mode(scratch: &Scratch, mode: &str) -> Result<String, (String, String)> {
    let path = scratch.path("m.txt");
    let value = from_str(mode);
    let word = |value: i64| crate::word::inline(value).unwrap();
    // SAFETY: the strings are live; the file keeps references of its own; each is released
    // once.
    unsafe {
        let opened = super::disk::typy_rt_open(
            path,
            value,
            word(1 << 10),
            word(-1),
            core::ptr::null(),
            core::ptr::null(),
        );
        typy_rt_decrement(path.cast());
        typy_rt_decrement(value.cast());
        if !opened.exception.is_null() {
            return Err(error_text(opened.exception));
        }
        let shown = super::tests::repr_of(opened.value);
        typy_rt_decrement(opened.value.cast());
        Ok(shown)
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn a_mode_value_is_read_as_cpython_reads_a_mode() {
    let scratch = Scratch::new("mode-value");
    for mode in ["w", "a+", "r", "rt", "+r"] {
        let shown = opened_with_mode(&scratch, mode).unwrap();
        assert!(
            shown.ends_with(&format!("mode='{mode}' encoding='utf-8'>")),
            "{shown}"
        );
    }
    let refused = [
        ("z", "ValueError", "invalid mode: 'z'"),
        ("rr", "ValueError", "invalid mode: 'rr'"),
        (
            "bt",
            "ValueError",
            "can't have text and binary mode at once",
        ),
        (
            "rw",
            "ValueError",
            "must have exactly one of create/read/write/append mode",
        ),
        (
            "+",
            "ValueError",
            "Must have exactly one of create/read/write/append mode and at most one plus",
        ),
        (
            "rb",
            "TypeError",
            "TyPy opens a text file for a mode that is not a literal, and this mode is binary: \
             'rb'",
        ),
    ];
    for (mode, class, text) in refused {
        assert_eq!(
            opened_with_mode(&scratch, mode),
            Err((class.to_owned(), text.to_owned())),
            "{mode}"
        );
    }
}

/// Returns the `repr` of the file `open` gives for the `int` word `fd` with `how`, or the
/// class and text of its error; the file is closed and released.
fn opened_for_descriptor(fd: i64, how: i64) -> Result<String, (String, String)> {
    let mode = from_str("r");
    let word = |value: i64| crate::word::inline(value).unwrap();
    // SAFETY: the mode is live and released once; the file is released once.
    unsafe {
        let opened = super::disk::typy_rt_open_descriptor(
            fd,
            mode,
            word(how),
            word(-1),
            core::ptr::null(),
            core::ptr::null(),
        );
        typy_rt_decrement(mode.cast());
        if !opened.exception.is_null() {
            return Err(error_text(opened.exception));
        }
        let shown = super::tests::repr_of(opened.value);
        assert_eq!(
            super::state::typy_rt_text_descriptor_name(opened.value),
            word(i64::from((*opened.value).descriptor))
        );
        assert!(typy_rt_text_close(opened.value).is_null());
        typy_rt_decrement(opened.value.cast());
        Ok(shown)
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn a_descriptor_opens_a_file_named_by_its_number() {
    let scratch = Scratch::new("descriptor");
    std::fs::write(scratch.0.join("d.txt"), "x").unwrap();
    let base = scratch.open("d.txt", "r", 0).unwrap();
    // SAFETY: the file is live.
    let number = unsafe { (*base).descriptor };
    let word = crate::word::inline(i64::from(number)).unwrap();
    let keep_open = 1 << 11;
    let shown = opened_for_descriptor(word, keep_open).unwrap();
    assert_eq!(
        shown,
        format!("<_io.TextIOWrapper name={number} mode='r' encoding='utf-8'>")
    );
    // `closefd=False` left the descriptor open; closing this one closes it.
    assert!(crate::libc::directory_status(number).is_ok());
    opened_for_descriptor(word, 0).unwrap();
    assert_eq!(crate::libc::directory_status(number), Err(9));
    // SAFETY: the file is live; closing its closed descriptor fails, as in CPython.
    unsafe {
        let error = typy_rt_text_close(base);
        assert_eq!(error_text(error).1, "[Errno 9] Bad file descriptor");
        typy_rt_decrement(base.cast());
    }
    let refused = [
        (-1, "ValueError", "negative file descriptor"),
        (999, "OSError", "[Errno 9] Bad file descriptor"),
    ];
    for (fd, class, text) in refused {
        let word = crate::word::inline(fd).unwrap();
        assert_eq!(
            opened_for_descriptor(word, 0),
            Err((class.to_owned(), text.to_owned()))
        );
    }
    let large = crate::int::value::Integer::from_i128(1 << 40).into_word();
    assert_eq!(
        opened_for_descriptor(large, 0),
        Err((
            "TypeError".to_owned(),
            "expected str, bytes or os.PathLike object, not int".to_owned()
        ))
    );
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn a_directory_descriptor_and_closefd_with_a_path_are_refused() {
    let scratch = Scratch::new("descriptor-directory");
    let directory = std::fs::File::open(&scratch.0).unwrap();
    let number = std::os::fd::AsRawFd::as_raw_fd(&directory);
    let word = crate::word::inline(i64::from(number)).unwrap();
    assert_eq!(
        opened_for_descriptor(word, 1 << 11),
        Err((
            "IsADirectoryError".to_owned(),
            format!("[Errno 21] Is a directory: {number}")
        ))
    );
    let path = scratch.path("p.txt");
    let mode = from_str("w");
    let small = |value: i64| crate::word::inline(value).unwrap();
    // SAFETY: the strings are live and released once.
    unsafe {
        let opened = super::disk::typy_rt_open(
            path,
            mode,
            small(1 | 1 << 11),
            small(-1),
            core::ptr::null(),
            core::ptr::null(),
        );
        assert_eq!(
            error_text(opened.exception).1,
            "Cannot use closefd=False with file name"
        );
        typy_rt_decrement(path.cast());
        typy_rt_decrement(mode.cast());
    }
    assert!(!scratch.0.join("p.txt").exists());
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn a_mode_value_of_x_creates_a_file_that_is_new() {
    let scratch = Scratch::new("mode-value-x");
    let shown = opened_with_mode(&scratch, "x").unwrap();
    assert!(shown.ends_with("mode='x' encoding='utf-8'>"), "{shown}");
    let path = scratch.0.join("m.txt").display().to_string();
    assert_eq!(
        opened_with_mode(&scratch, "x"),
        Err((
            "FileExistsError".to_owned(),
            format!("[Errno 17] File exists: '{path}'")
        ))
    );
}

/// Returns `open(fd, mode, buffering, closefd=False)` with the access and form bits of `how`,
/// or the class and text of its error.
fn opened_keeping(
    descriptor: i32,
    mode: &str,
    how: i64,
    buffering: i64,
) -> Result<*mut super::TextFile, (String, String)> {
    let mode = from_str(mode);
    // SAFETY: the mode is live and released once; the file keeps a reference of its own.
    unsafe {
        let opened = super::disk::typy_rt_open_descriptor(
            size(i64::from(descriptor)),
            mode,
            size(how | 1 << 11),
            size(buffering),
            core::ptr::null(),
            core::ptr::null(),
        );
        typy_rt_decrement(mode.cast());
        if opened.exception.is_null() {
            Ok(opened.value)
        } else {
            Err(error_text(opened.exception))
        }
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn closefd_false_keeps_the_descriptor_when_unbuffered_text_is_refused() {
    let (mut reading, writing) = std::io::pipe().unwrap();
    let descriptor = std::os::fd::AsRawFd::as_raw_fd(&writing);
    assert_eq!(
        opened_keeping(descriptor, "w", 1, 0).map(|_| ()),
        Err((
            "ValueError".to_owned(),
            "can't have unbuffered text I/O".to_owned()
        ))
    );
    assert_eq!(crate::libc::write_retrying(descriptor, b"still"), Ok(()));
    let mut found = [0; 5];
    std::io::Read::read_exact(&mut reading, &mut found).unwrap();
    assert_eq!(&found, b"still");
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn a_file_io_with_closefd_false_shows_it() {
    let (reading, _writing) = std::io::pipe().unwrap();
    let descriptor = std::os::fd::AsRawFd::as_raw_fd(&reading);
    let file = opened_keeping(descriptor, "rb", 1 << 8 | 1 << 9, -1).unwrap();
    assert_eq!(
        super::tests::repr_of(file),
        format!("<_io.FileIO name={descriptor} mode='rb' closefd=False>")
    );
    // SAFETY: the file is live and released once.
    unsafe {
        assert!(typy_rt_text_close(file).is_null());
        typy_rt_decrement(file.cast());
    }
    assert!(crate::libc::directory_status(descriptor).is_ok());
}

#[test]
fn a_binary_line_stops_at_an_error_of_reading() {
    let latin = decoding(Codec::Latin1, Handler::Strict, Newline::Lf);
    let mut reader = Box::new(Reader::with(latin));
    let mut source = source_of(&[b"ab"], Some(5));
    let error = reader.read_bytes(None, true, &mut source).unwrap_err();
    assert_eq!(
        error_text(error),
        (
            "OSError".to_owned(),
            "[Errno 5] Input/output error".to_owned()
        )
    );
}
