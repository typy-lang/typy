//! Standard output, buffered in 8 KiB, and standard error, unbuffered (detailed design section 14.2).

use crate::cell::ProgramCell;
use crate::libc::{STDERR, STDOUT, write_all};
use crate::string::{StrObject, write_utf8};

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

/// Writes a string to standard output as UTF-8.
///
/// # Safety
///
/// `text` points to a live `str` object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_write_stdout(text: *const StrObject) {
    // SAFETY: the caller passes a live string.
    unsafe { write_utf8(text, write_stdout_bytes) };
}

#[cfg(test)]
mod tests {
    use super::{CAPACITY, STDOUT_BUFFER, flush_stdout, typy_rt_write_stdout, write_stderr};
    use crate::string::from_str;

    #[test]
    fn output_is_buffered_until_flushed_and_large_writes_pass_through() {
        flush_stdout();
        // SAFETY: the strings are live.
        unsafe { typy_rt_write_stdout(from_str("")) };
        assert_eq!(STDOUT_BUFFER.with(|buffer| buffer.used), 0);
        let large = "x".repeat(CAPACITY + 10);
        // SAFETY: the string is live.
        unsafe { typy_rt_write_stdout(from_str(&large)) };
        write_stderr(b"");
        assert_eq!(STDOUT_BUFFER.with(|buffer| buffer.used), 0);
    }
}
