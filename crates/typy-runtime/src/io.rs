//! Standard output, buffered in 8 KiB, and standard error, unbuffered (detailed design section 14.2).

use crate::cell::ProgramCell;
use crate::libc::{STDERR, STDOUT, write_all};
use crate::string::{StrObject, Surrogates, write_utf8};

const CAPACITY: usize = 8192;

struct Buffer {
    bytes: [u8; CAPACITY],
    used: usize,
}

static STDOUT_BUFFER: ProgramCell<Buffer> = ProgramCell::new(Buffer {
    bytes: [0; CAPACITY],
    used: 0,
});

fn write_stdout_bytes(bytes: &[u8]) {
    STDOUT_BUFFER.with(|buffer| {
        if buffer.used + bytes.len() > CAPACITY {
            write_all(STDOUT, &buffer.bytes[..buffer.used]);
            buffer.used = 0;
        }
        if bytes.len() > CAPACITY {
            write_all(STDOUT, bytes);
        } else {
            buffer.bytes[buffer.used..buffer.used + bytes.len()].copy_from_slice(bytes);
            buffer.used += bytes.len();
        }
    });
}

/// Which of standard input, output and error the program closed, one bit per descriptor, as
/// `sys.stdin.close()` and its companions set it (LIB-14). CPython opens the three with
/// `closefd=False`, so the descriptors stay open.
static STANDARD_CLOSED: ProgramCell<u8> = ProgramCell::new(0);

/// Returns whether the program closed the standard file of `descriptor`, 0 to 2.
pub(crate) fn standard_closed(descriptor: i32) -> bool {
    STANDARD_CLOSED.with(|closed| *closed & (1 << (descriptor & 3)) != 0)
}

/// Records that the program closed the standard file of `descriptor`, 0 to 2.
pub(crate) fn close_standard(descriptor: i32) {
    STANDARD_CLOSED.with(|closed| *closed |= 1 << (descriptor & 3));
}

/// Writes everything buffered for standard output.
pub(crate) fn flush_stdout() {
    STDOUT_BUFFER.with(|buffer| {
        write_all(STDOUT, &buffer.bytes[..buffer.used]);
        buffer.used = 0;
    });
}

/// Writes to standard error, after flushing standard output so the two keep their order on a terminal.
pub(crate) fn write_stderr(bytes: &[u8]) {
    flush_stdout();
    write_all(STDERR, bytes);
}

/// Writes a string to standard output as UTF-8, a surrogate as `surrogateescape` writes it.
///
/// # Safety
///
/// `text` points to a live `str` object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_write_stdout(text: *const StrObject) {
    // SAFETY: the caller passes a live string.
    unsafe { write_utf8(text, Surrogates::Escape, write_stdout_bytes) };
}

#[cfg(test)]
mod tests;
