//! Tests of what a text file says about itself and of `seek`, `tell` and `truncate`, against
//! the results and messages of CPython 3.15.

use super::position::{typy_rt_text_seek, typy_rt_text_tell, typy_rt_text_truncate};
use super::state::{typy_rt_text_fileno, typy_rt_text_mode, typy_rt_text_name, typy_rt_text_state};
use super::stdin::STDIN;
use super::tests::{Scratch, error_text, size, take};
use super::{
    TextFile, typy_rt_sys_stderr, typy_rt_sys_stdout, typy_rt_text_close, typy_rt_text_read,
    typy_rt_text_readline, typy_rt_text_write,
};
use crate::refcount::typy_rt_decrement;
use crate::string::from_str;

const TERMINAL: i64 = 0;
const READABLE: i64 = 1;
const WRITABLE: i64 = 2;
const SEEKABLE: i64 = 3;
const CLOSED: i64 = 4;

/// Returns the answer to `question` about `file`, or the text of its error.
fn state(file: *const TextFile, question: i64) -> Result<bool, String> {
    // SAFETY: the file is live.
    let answer = unsafe { typy_rt_text_state(file, size(question)) };
    if answer.exception.is_null() {
        Ok(answer.value)
    } else {
        Err(error_text(answer.exception).1)
    }
}

/// Returns the value of an `int` word the runtime returned, or the class and text of its
/// error.
fn number(result: crate::Raising<i64>) -> Result<i64, (String, String)> {
    if !result.exception.is_null() {
        return Err(error_text(result.exception));
    }
    if crate::word::is_inline(result.value) {
        return Ok(crate::word::inline_value(result.value));
    }
    // SAFETY: the word is a large integer owned here.
    let value = unsafe { crate::int::value::Integer::from_word(result.value) }
        .to_i128()
        .and_then(|value| i64::try_from(value).ok())
        .unwrap();
    // SAFETY: as above.
    unsafe { typy_rt_decrement(crate::int::object_of(result.value).cast()) };
    Ok(value)
}

/// Returns the `int` word of `value`, a large integer when it is not inline.
fn word(value: i128) -> i64 {
    crate::int::value::Integer::from_i128(value).into_word()
}

/// Returns `file.seek(cookie, whence)`.
fn seek(file: *mut TextFile, cookie: i128, whence: i128) -> Result<i64, (String, String)> {
    let (cookie, whence) = (word(cookie), word(whence));
    // SAFETY: the file is live and the words are owned here.
    unsafe {
        let moved = number(typy_rt_text_seek(file, cookie, whence));
        for value in [cookie, whence] {
            if !crate::word::is_inline(value) {
                typy_rt_decrement(crate::int::object_of(value).cast());
            }
        }
        moved
    }
}

/// Returns `file.tell()`.
fn tell(file: *mut TextFile) -> Result<i64, (String, String)> {
    // SAFETY: the file is live.
    number(unsafe { typy_rt_text_tell(file) })
}

/// Returns `file.truncate(size)`, or `file.truncate()` for `None`.
fn truncate(file: *mut TextFile, cut: Option<i128>) -> Result<i64, (String, String)> {
    let cut_word = cut.map_or(size(0), word);
    let given = size(i64::from(cut.is_some()));
    // SAFETY: the file is live and the word is owned here.
    unsafe {
        let result = number(typy_rt_text_truncate(file, cut_word, given));
        if !crate::word::is_inline(cut_word) {
            typy_rt_decrement(crate::int::object_of(cut_word).cast());
        }
        result
    }
}

/// Writes `text` to `file`.
fn write(file: *mut TextFile, text: &str) {
    let text = from_str(text);
    // SAFETY: the file and the text are live; the text is owned here.
    unsafe {
        assert!(typy_rt_text_write(file, text).exception.is_null());
        typy_rt_decrement(text.cast());
    }
}

fn error(class: &str, text: &str) -> (String, String) {
    (class.to_owned(), text.to_owned())
}

#[test]
fn the_standard_files_give_their_names_modes_and_descriptors() {
    let files = [
        (typy_rt_sys_stdout(), "<stdout>", "w", 1, false),
        (typy_rt_sys_stderr(), "<stderr>", "w", 2, false),
        (&raw const STDIN, "<stdin>", "r", 0, true),
    ];
    for (file, name, mode, descriptor, readable) in files {
        // SAFETY: the standard files are live.
        unsafe {
            assert_eq!(take(typy_rt_text_name(file)), name);
            assert_eq!(take(typy_rt_text_mode(file)), mode);
            assert_eq!(number(typy_rt_text_fileno(file)), Ok(descriptor), "{name}");
        }
        assert_eq!(state(file, READABLE), Ok(readable), "{name}");
        assert_eq!(state(file, WRITABLE), Ok(!readable), "{name}");
        assert_eq!(state(file, CLOSED), Ok(false), "{name}");
        // Miri cannot ask a standard file whether it is a terminal or can seek.
        #[cfg(not(miri))]
        {
            assert!(state(file, TERMINAL).is_ok());
            assert!(state(file, SEEKABLE).is_ok());
        }
    }
}

#[test]
fn standard_input_cannot_be_truncated() {
    let stdin = (&raw const STDIN).cast_mut();
    assert_eq!(
        truncate(stdin, None),
        Err(error("ioUnsupportedOperation", "truncate"))
    );
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn a_file_on_disk_answers_and_then_raises_once_closed() {
    let scratch = Scratch::new("state");
    let written = scratch.open("w.txt", "w", 1).unwrap();
    let read = scratch.open("w.txt", "r", 0).unwrap();
    // SAFETY: the files are live; each is released once.
    unsafe {
        assert!(take(typy_rt_text_name(written)).ends_with("w.txt"));
        assert_eq!(take(typy_rt_text_mode(written)), "w");
        assert!(number(typy_rt_text_fileno(written)).unwrap() > 2);
        for (file, readable) in [(written, false), (read, true)] {
            assert_eq!(state(file, TERMINAL), Ok(false));
            assert_eq!(state(file, READABLE), Ok(readable));
            assert_eq!(state(file, WRITABLE), Ok(!readable));
            assert_eq!(state(file, SEEKABLE), Ok(true));
            assert!(typy_rt_text_close(file).is_null());
            assert_eq!(state(file, CLOSED), Ok(true));
            let closed = Err("I/O operation on closed file".to_owned());
            for question in [TERMINAL, SEEKABLE] {
                assert_eq!(state(file, question), closed);
            }
            // The question about the direction the file lacks still answers false.
            let (own, other) = if readable {
                (READABLE, WRITABLE)
            } else {
                (WRITABLE, READABLE)
            };
            assert_eq!(state(file, own), closed);
            assert_eq!(state(file, other), Ok(false));
            assert_eq!(
                number(typy_rt_text_fileno(file)),
                Err(error("ValueError", "I/O operation on closed file"))
            );
            let closed = Err(error("ValueError", "I/O operation on closed file."));
            assert_eq!(tell(file), closed);
            assert_eq!(seek(file, 0, 0), closed);
            assert_eq!(truncate(file, None), closed);
            assert_eq!(
                take(typy_rt_text_mode(file)),
                if readable { "r" } else { "w" }
            );
        }
        typy_rt_decrement(written.cast());
        typy_rt_decrement(read.cast());
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn seek_and_tell_move_through_a_written_file_by_bytes() {
    let scratch = Scratch::new("seek");
    let file = scratch.open("s.txt", "w", 1).unwrap();
    write(file, "h\u{e9}llo\nworld\n");
    assert_eq!(tell(file), Ok(13));
    assert_eq!(seek(file, 0, 0), Ok(0));
    write(file, "J");
    assert_eq!(seek(file, 0, 2), Ok(13));
    assert_eq!(seek(file, 0, 1), Ok(13));
    let refused = [
        (
            1,
            1,
            "ioUnsupportedOperation",
            "can't do nonzero cur-relative seeks",
        ),
        (
            -1,
            2,
            "ioUnsupportedOperation",
            "can't do nonzero end-relative seeks",
        ),
        (
            -1,
            3,
            "ValueError",
            "invalid whence (3, should be 0, 1 or 2)",
        ),
        (-1, 0, "ValueError", "negative seek position -1"),
        (
            -(1 << 70),
            0,
            "ValueError",
            "negative seek position -1180591620717411303424",
        ),
        (1 << 63, 0, "OSError", "[Errno 22] Invalid argument"),
        (1 << 70, 0, "OSError", "[Errno 22] Invalid argument"),
        (
            0,
            1 << 40,
            "OverflowError",
            "Python int too large to convert to C int",
        ),
        (
            0,
            1 << 70,
            "OverflowError",
            "Python int too large to convert to C int",
        ),
    ];
    for (cookie, whence, class, text) in refused {
        assert_eq!(
            seek(file, cookie, whence),
            Err(error(class, text)),
            "{cookie} {whence}"
        );
    }
    assert_eq!(truncate(file, Some(3)), Ok(3));
    assert_eq!(tell(file), Ok(13));
    assert_eq!(truncate(file, None), Ok(13));
    assert_eq!(
        truncate(file, Some(-1)),
        Err(error("OSError", "[Errno 22] Invalid argument"))
    );
    let too_large = error("OverflowError", "Python int too large to convert to C long");
    assert_eq!(truncate(file, Some(1 << 70)), Err(too_large.clone()));
    assert_eq!(truncate(file, Some(-(1 << 70))), Err(too_large));
    // SAFETY: the file is live and released once.
    unsafe { typy_rt_decrement(file.cast()) };
    let bytes = std::fs::read(scratch.0.join("s.txt")).unwrap();
    assert_eq!(bytes, b"J\xc3\xa9\0\0\0\0\0\0\0\0\0\0");
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn tell_counts_what_the_reader_holds_and_seek_drops_it() {
    let scratch = Scratch::new("tell");
    std::fs::write(scratch.0.join("r.txt"), "ab\r\ncd\nef\n").unwrap();
    let file = scratch.open("r.txt", "r", 0).unwrap();
    // SAFETY: the file is live and released once.
    unsafe {
        assert_eq!(take(typy_rt_text_read(file, size(3)).value), "ab\n");
        assert_eq!(tell(file), Ok(4));
        assert_eq!(take(typy_rt_text_readline(file, size(-1)).value), "cd\n");
        assert_eq!(tell(file), Ok(7));
        assert_eq!(seek(file, 1, 0), Ok(1));
        assert_eq!(take(typy_rt_text_read(file, size(-1)).value), "b\ncd\nef\n");
        assert_eq!(tell(file), Ok(10));
        assert_eq!(
            truncate(file, Some(1)),
            Err(error("ioUnsupportedOperation", "truncate"))
        );
        typy_rt_decrement(file.cast());
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn a_for_loop_turns_tell_off_until_its_end_or_a_seek() {
    let scratch = Scratch::new("loop");
    std::fs::write(scratch.0.join("l.txt"), "ab\ncd\n").unwrap();
    let file = scratch.open("l.txt", "r", 0).unwrap();
    let disabled = Err(error("OSError", "telling position disabled by next() call"));
    // SAFETY: the file is live and released once; each line is released once.
    unsafe {
        let line = super::loop_line(file).unwrap().unwrap();
        assert_eq!(take(line), "ab\n");
        assert_eq!(tell(file), disabled);
        assert_eq!(seek(file, 0, 1), disabled);
        assert_eq!(seek(file, 0, 2), Ok(6));
        assert_eq!(tell(file), Ok(6));
        assert_eq!(seek(file, 0, 0), Ok(0));
        while let Some(line) = super::loop_line(file).unwrap() {
            typy_rt_decrement(line.cast());
        }
        assert_eq!(tell(file), Ok(6));
        typy_rt_decrement(file.cast());
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn an_appended_file_starts_at_its_end() {
    let scratch = Scratch::new("append");
    std::fs::write(scratch.0.join("a.txt"), "abc").unwrap();
    let file = scratch.open("a.txt", "a", 2).unwrap();
    assert_eq!(tell(file), Ok(3));
    assert_eq!(seek(file, 0, 0), Ok(0));
    write(file, "Q");
    assert_eq!(tell(file), Ok(4));
    // SAFETY: the file is live and released once.
    unsafe { typy_rt_decrement(file.cast()) };
}

#[test]
#[cfg_attr(
    miri,
    ignore = "closing a standard file closes it for every test, and Miri runs them in one process"
)]
fn closing_standard_output_keeps_its_descriptor_and_refuses_what_follows() {
    let stdout = typy_rt_sys_stdout().cast_mut();
    let text = from_str("x");
    let closed = (
        "ValueError".to_owned(),
        "I/O operation on closed file.".to_owned(),
    );
    // SAFETY: standard output and the text are live; the text is released once.
    unsafe {
        assert!(typy_rt_text_close(stdout).is_null());
        assert!(typy_rt_text_close(stdout).is_null());
        assert_eq!(state(stdout, CLOSED), Ok(true));
        assert_eq!(state(stdout, READABLE), Ok(false));
        assert_eq!(
            state(stdout, WRITABLE),
            Err("I/O operation on closed file".to_owned())
        );
        assert_eq!(
            error_text(typy_rt_text_write(stdout, text).exception),
            closed
        );
        assert_eq!(error_text(super::typy_rt_text_flush(stdout)), closed);
        assert_eq!(
            error_text(super::typy_rt_write_stdout_checked(text)),
            closed
        );
        assert_eq!(
            error_text(crate::sys::typy_rt_input(core::ptr::null()).exception),
            closed
        );
        typy_rt_decrement(text.cast());
    }
    // The descriptor stays open, as CPython opens standard output with `closefd=False`;
    // Miri does not seek on standard output.
    #[cfg(not(miri))]
    assert_ne!(crate::libc::seek(1, 0, crate::libc::SEEK_CUR), Err(9));
}

#[test]
#[cfg_attr(
    miri,
    ignore = "closing a standard file closes it for every test, and Miri runs them in one process"
)]
fn closing_standard_input_refuses_reading_it() {
    let stdin = (&raw const STDIN).cast_mut();
    let closed = (
        "ValueError".to_owned(),
        "I/O operation on closed file.".to_owned(),
    );
    // SAFETY: standard input is live.
    unsafe {
        assert!(typy_rt_text_close(stdin).is_null());
        assert_eq!(
            error_text(typy_rt_text_readline(stdin, size(-1)).exception),
            closed
        );
        assert_eq!(
            error_text(typy_rt_text_read(stdin, size(-1)).exception),
            closed
        );
        assert_eq!(error_text(super::stdin::input_line().unwrap_err()), closed);
        assert_eq!(error_text(super::typy_rt_text_enter(stdin)), closed);
    }
}

#[test]
fn printing_to_standard_output_while_it_is_open_writes() {
    let text = from_str("");
    // SAFETY: the text is live and released once.
    unsafe {
        assert!(super::typy_rt_write_stdout_checked(text).is_null());
        typy_rt_decrement(text.cast());
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn a_plus_file_writes_where_its_reader_stopped_and_reads_after_writing_out() {
    let scratch = Scratch::new("plus");
    std::fs::write(scratch.0.join("p.txt"), "abcdef\n").unwrap();
    // `r+` is the access `r` with the bit of `+`.
    let file = scratch.open("p.txt", "r+", 4).unwrap();
    assert_eq!(state(file, READABLE), Ok(true));
    assert_eq!(state(file, WRITABLE), Ok(true));
    // SAFETY: the file is live and released once.
    unsafe {
        assert_eq!(take(typy_rt_text_read(file, size(2)).value), "ab");
        assert_eq!(tell(file), Ok(2));
        write(file, "X");
        assert_eq!(tell(file), Ok(8));
        assert_eq!(seek(file, 0, 0), Ok(0));
        write(file, "YY");
        assert_eq!(take(typy_rt_text_read(file, size(-1)).value), "cdef\nX");
        typy_rt_decrement(file.cast());
    }
    let written = std::fs::read(scratch.0.join("p.txt")).unwrap();
    assert_eq!(written, b"YYcdef\nX");
}

/// Returns a file over the write end of a new pipe, opened with `closefd=False`, and the pipe.
fn pipe_file() -> (*mut TextFile, std::io::PipeReader, std::io::PipeWriter) {
    let (reading, writing) = std::io::pipe().unwrap();
    let descriptor = std::os::fd::AsRawFd::as_raw_fd(&writing);
    let mode = from_str("w");
    // SAFETY: the mode is live and released once; the file keeps a reference of its own.
    unsafe {
        let opened = super::disk::typy_rt_open_descriptor(
            size(i64::from(descriptor)),
            mode,
            size(1 | 1 << 11),
            size(-1),
            core::ptr::null(),
            core::ptr::null(),
        );
        typy_rt_decrement(mode.cast());
        assert!(opened.exception.is_null());
        (opened.value, reading, writing)
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn a_pipe_cannot_tell_or_seek() {
    let (file, _reading, _writing) = pipe_file();
    let refused = Err(error(
        "ioUnsupportedOperation",
        "underlying stream is not seekable",
    ));
    assert_eq!(tell(file), refused);
    assert_eq!(seek(file, 0, 0), refused);
    // SAFETY: the file is live and released once.
    unsafe { typy_rt_decrement(file.cast()) };
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn a_negative_position_too_long_to_show_raises_the_digit_limit() {
    let scratch = Scratch::new("seek-digits");
    let file = scratch.open("d.txt", "w", 1).unwrap();
    // 300 limbs hold more than 4300 decimal digits.
    let cookie = crate::int::value::Integer {
        negative: true,
        magnitude: crate::int::limbs::Limbs::from_slice(&[1; 300]),
    }
    .into_word();
    // SAFETY: the file and the word are live; each is released once.
    unsafe {
        let moved = typy_rt_text_seek(file, cookie, size(0));
        assert_eq!(
            error_text(moved.exception),
            error(
                "ValueError",
                "Exceeds the limit (4300 digits) for integer string conversion; use \
                 sys.set_int_max_str_digits() to increase the limit"
            )
        );
        typy_rt_decrement(crate::int::object_of(cookie).cast());
        typy_rt_decrement(file.cast());
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not open files without disabling isolation")]
fn a_write_the_device_refuses_fails_the_next_operation_that_writes_it_out() {
    let scratch = Scratch::new("full-text");
    let full = error("OSError", "[Errno 28] No space left on device");
    // Joining an absolute path gives that path.
    let file = scratch.open("/dev/full", "w+", 1 | 4).unwrap();
    write(file, "x");
    assert_eq!(tell(file), Err(full.clone()));
    write(file, "x");
    assert_eq!(seek(file, 0, 0), Err(full.clone()));
    write(file, "x");
    assert_eq!(truncate(file, None), Err(full.clone()));
    write(file, "x");
    // SAFETY: the file is live and released once.
    unsafe {
        assert_eq!(
            error_text(typy_rt_text_read(file, size(-1)).exception),
            full
        );
        write(file, "x");
        assert_eq!(error_text(typy_rt_text_close(file)), full);
        typy_rt_decrement(file.cast());
    }
}
