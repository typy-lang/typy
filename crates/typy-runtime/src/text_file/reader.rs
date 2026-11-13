//! Reading a text file as CPython's `TextIOWrapper` reads it (ADR-0019): 8192 bytes at a time,
//! each chunk checked as UTF-8 when it is read, with the bytes of a character cut off at its
//! end carried into the next, and `\r\n` and `\r` read as `\n`.

use crate::byte_buffer::ByteBuffer;
use crate::codec::{utf8_decode_error, utf8_sequence};
use crate::exception::ExceptionObject;
use crate::string::{StrBuilder, StrObject};

/// The bytes CPython's text layer decodes at a time, `TextIOWrapper._CHUNK_SIZE`.
pub(crate) const CHUNK: usize = 8192;

/// Reads `buffer.len()` bytes at most, and returns how many, 0 at the end, or the error number.
pub(crate) type Source<'source> = &'source mut dyn FnMut(&mut [u8]) -> Result<usize, i32>;

/// What a readable file has read and not given out yet.
pub(crate) struct Reader {
    /// Room for a chunk, the bytes of a character cut off at the end of the last one, and a
    /// `\r` that waits for the next byte.
    bytes: [u8; CHUNK + 4],
    /// The next byte to give out.
    start: usize,
    /// The end of the bytes checked as UTF-8.
    valid: usize,
    /// The end of the bytes read.
    end: usize,
    /// Whether the source has given its last byte.
    at_end: bool,
}

/// Returns the length of the valid UTF-8 at the start of `data`, leaving out a character cut
/// off at its end unless `last`, or the start, end and reason of the first error, as CPython's
/// incremental decoder finds them.
fn check(data: &[u8], last: bool) -> Result<usize, (usize, usize, &'static str)> {
    let byte = |index: usize| data.get(index).map(|byte| u32::from(*byte));
    let mut at = 0;
    while at < data.len() {
        match utf8_sequence(byte, at) {
            Ok((count, _)) => at += count,
            Err((usize::MAX, _)) if !last => return Ok(at),
            Err((usize::MAX, reason)) => return Err((at, data.len(), reason)),
            Err((end, reason)) => return Err((at, end, reason)),
        }
    }
    Ok(at)
}

/// Returns the code point at `at` of bytes checked as UTF-8, and its length.
fn decode_at(bytes: &[u8], at: usize) -> (usize, u32) {
    let byte = |index: usize| bytes.get(index).map(|byte| u32::from(*byte));
    utf8_sequence(byte, at).unwrap_or((1, 0xfffd))
}

impl Reader {
    /// Returns a reader that has read nothing.
    pub(crate) const fn new() -> Self {
        Self {
            bytes: [0; CHUNK + 4],
            start: 0,
            valid: 0,
            end: 0,
            at_end: false,
        }
    }

    /// Reads the next chunk after the bytes not given out yet and checks it; at the end of the
    /// source, a character cut off is an error. A chunk with an error is dropped.
    fn refill(&mut self, source: Source<'_>) -> Result<(), *mut ExceptionObject> {
        self.bytes.copy_within(self.start..self.end, 0);
        self.valid -= self.start;
        self.end -= self.start;
        self.start = 0;
        let tail = self.valid;
        let count = source(&mut self.bytes[self.end..self.end + CHUNK])
            // SAFETY: an error of reading names no file.
            .map_err(|errno| unsafe { super::os_error::os_error(errno, core::ptr::null()) })?;
        self.end += count;
        self.at_end = count == 0;
        match check(&self.bytes[tail..self.end], self.at_end) {
            Ok(length) => {
                self.valid = tail + length;
                Ok(())
            }
            Err((start, end, reason)) => {
                let error = utf8_decode_error(&self.bytes[tail..self.end], start, end, reason);
                self.end = tail;
                Err(error)
            }
        }
    }

    /// Returns up to `limit` characters, all of them when `limit` is `None`, ending after the
    /// first `\n` when `line`; an empty string at the end of the source.
    pub(crate) fn read_text(
        &mut self,
        limit: Option<usize>,
        line: bool,
        source: Source<'_>,
    ) -> Result<*mut StrObject, *mut ExceptionObject> {
        let mut out = StrBuilder::with_capacity(80);
        let mut count = 0;
        loop {
            while self.start < self.valid {
                if limit == Some(count) {
                    return Ok(out.finish());
                }
                let point = if self.bytes[self.start] == b'\r' {
                    // A `\r` at the end of what was read waits for the next byte.
                    if self.start + 1 == self.end && !self.at_end {
                        break;
                    }
                    self.start += 1;
                    if self.start < self.valid && self.bytes[self.start] == b'\n' {
                        self.start += 1;
                    }
                    u32::from(b'\n')
                } else {
                    let (length, point) = decode_at(&self.bytes[..self.valid], self.start);
                    self.start += length;
                    point
                };
                out.push(point);
                count += 1;
                if line && point == u32::from(b'\n') {
                    return Ok(out.finish());
                }
            }
            if self.at_end {
                return Ok(out.finish());
            }
            if let Err(error) = self.refill(source) {
                out.discard();
                return Err(error);
            }
        }
    }

    /// Returns the rest of the source, decoded at once as CPython's `read()` decodes it, so an
    /// error's position counts from the first byte not yet decoded.
    pub(crate) fn read_rest(
        &mut self,
        source: Source<'_>,
    ) -> Result<*mut StrObject, *mut ExceptionObject> {
        let mut all = ByteBuffer::new();
        all.extend(&self.bytes[self.start..self.end]);
        let decoded = self.valid - self.start;
        let mut chunk = [0_u8; CHUNK];
        while !self.at_end {
            match source(&mut chunk) {
                Ok(0) => self.at_end = true,
                Ok(count) => all.extend(&chunk[..count]),
                Err(errno) => {
                    all.discard();
                    // SAFETY: an error of reading names no file.
                    return Err(unsafe { super::os_error::os_error(errno, core::ptr::null()) });
                }
            }
        }
        self.start = 0;
        self.valid = 0;
        self.end = 0;
        let bytes = all.bytes();
        if let Err((start, end, reason)) = check(&bytes[decoded..], true) {
            let error = utf8_decode_error(&bytes[decoded..], start, end, reason);
            all.discard();
            return Err(error);
        }
        let mut out = StrBuilder::with_capacity(bytes.len());
        let mut at = 0;
        while at < bytes.len() {
            if bytes[at] == b'\r' {
                at += 1;
                if bytes.get(at) == Some(&b'\n') {
                    at += 1;
                }
                out.push(u32::from(b'\n'));
                continue;
            }
            let (length, point) = decode_at(bytes, at);
            at += length;
            out.push(point);
        }
        all.discard();
        Ok(out.finish())
    }
}
