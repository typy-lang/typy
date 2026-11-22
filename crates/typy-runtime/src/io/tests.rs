//! Tests of the standard output and standard error buffers.

use super::{
    CAPACITY, STDOUT_BUFFER, flush_stdout, typy_rt_write_stdout, write_stderr, write_stdout_bytes,
};
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

#[test]
fn bytes_larger_than_the_buffer_are_written_at_once() {
    flush_stdout();
    write_stdout_bytes(&[b'\n'; CAPACITY + 1]);
    assert_eq!(STDOUT_BUFFER.with(|buffer| buffer.used), 0);
    write_stdout_bytes(b"\n");
    assert_eq!(STDOUT_BUFFER.with(|buffer| buffer.used), 1);
    flush_stdout();
}
