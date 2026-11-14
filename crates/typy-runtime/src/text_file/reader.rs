//! Reading a text file as CPython's `TextIOWrapper` reads it (ADR-0019, LIB-14): 8192 bytes at
//! a time, each chunk checked by the file's codec when it is read, with the bytes of a
//! character cut off at its end carried into the next. The error handler decodes what the
//! codec cannot, and the newline mode decides where a line ends and whether `\r\n` and `\r`
//! read as `\n`. Standard input decodes UTF-8 with `surrogateescape`, as CPython's UTF-8 mode
//! does, the default from 3.15.

use super::newline::Newline;
use crate::byte_buffer::ByteBuffer;
use crate::codec::{Codec, Handler, chunk_decode_error, utf8_sequence};
use crate::exception::ExceptionObject;
use crate::string::{StrBuilder, StrObject};

/// The bytes CPython's text layer decodes at a time, `TextIOWrapper._CHUNK_SIZE`.
pub(crate) const CHUNK: usize = 8192;

/// Reads `buffer.len()` bytes at most, and returns how many, 0 at the end, or the error number.
pub(crate) type Source<'source> = &'source mut dyn FnMut(&mut [u8]) -> Result<usize, i32>;

/// How a file decodes its bytes and ends its lines.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Decoding {
    pub(crate) codec: Codec,
    pub(crate) handler: Handler,
    pub(crate) newline: Newline,
}

impl Decoding {
    /// UTF-8, strict, with universal newlines: what `open` gives by default.
    pub(crate) const DEFAULT: Self = Self {
        codec: Codec::Utf8,
        handler: Handler::Strict,
        newline: Newline::Universal,
    };

    /// Whether the handler decodes every byte, so checking a chunk finds no error.
    fn lenient(self) -> bool {
        !matches!(self.handler, Handler::Strict | Handler::XmlCharRefReplace)
    }
}

/// What a readable file has read and not given out yet.
pub(crate) struct Reader {
    /// Room for a chunk, the bytes of a character cut off at the end of the last one, and a
    /// `\r` that waits for the next byte.
    bytes: [u8; CHUNK + 4],
    /// The next byte to give out.
    start: usize,
    /// The end of the bytes checked by the codec.
    valid: usize,
    /// The end of the bytes read.
    end: usize,
    /// Whether the source has given its last byte.
    at_end: bool,
    /// How the bytes are decoded and lines end.
    decoding: Decoding,
    /// Characters decoded and not given out yet: the rest of a `backslashreplace` escape
    /// that a size cut, from `pending_start` to `pending_end`.
    pending: [u32; 4],
    pending_start: u8,
    pending_end: u8,
    /// Whether a `for` loop has taken a line since the last `seek` and not reached the end,
    /// which turns `tell` off, as in CPython.
    iterating: bool,
}

/// Returns the length of the bytes at the start of `data` that decode, leaving out a
/// character cut off at its end unless `last`, or the start, end and reason of the first
/// error, as CPython's incremental decoder finds them. A lenient handler decodes every byte.
fn check(
    data: &[u8],
    last: bool,
    decoding: Decoding,
) -> Result<usize, (usize, usize, &'static str)> {
    let lenient = decoding.lenient();
    match decoding.codec {
        Codec::Latin1 => Ok(data.len()),
        Codec::Ascii => match data.iter().position(|byte| *byte >= 0x80) {
            Some(at) if !lenient => Err((at, at + 1, "ordinal not in range(128)")),
            _ => Ok(data.len()),
        },
        Codec::Utf8 => {
            let byte = |index: usize| data.get(index).map(|byte| u32::from(*byte));
            let mut at = 0;
            while at < data.len() {
                match utf8_sequence(byte, at) {
                    Ok((count, _)) => at += count,
                    Err((usize::MAX, _)) if !last => return Ok(at),
                    Err(_) if lenient => at += 1,
                    Err((usize::MAX, reason)) => return Err((at, data.len(), reason)),
                    Err((end, reason)) => return Err((at, end, reason)),
                }
            }
            Ok(at)
        }
    }
}

/// What decoding at one position gives: the bytes it takes and up to four characters.
struct Decoded {
    length: usize,
    points: [u32; 4],
    count: u8,
}

impl Decoded {
    const fn one(length: usize, point: u32) -> Self {
        Self {
            length,
            points: [point, 0, 0, 0],
            count: 1,
        }
    }
}

/// Decodes at `at` of bytes the codec has checked: one character, or what the handler gives
/// for the bytes it cannot decode there. `ignore` and `replace` take the whole error, as
/// CPython does; `backslashreplace` and `surrogateescape` take one byte at a time, which gives
/// the same characters.
fn decode_at(bytes: &[u8], at: usize, decoding: Decoding) -> Decoded {
    let error_end = match decoding.codec {
        Codec::Latin1 => return Decoded::one(1, u32::from(bytes[at])),
        Codec::Ascii if bytes[at] < 0x80 => return Decoded::one(1, u32::from(bytes[at])),
        Codec::Ascii => at + 1,
        Codec::Utf8 => {
            let byte = |index: usize| bytes.get(index).map(|byte| u32::from(*byte));
            match utf8_sequence(byte, at) {
                Ok((length, point)) => return Decoded::one(length, point),
                Err((end, _)) => end.min(bytes.len()),
            }
        }
    };
    let bad = u32::from(bytes[at]);
    match decoding.handler {
        Handler::Ignore => Decoded {
            length: error_end - at,
            points: [0; 4],
            count: 0,
        },
        Handler::Replace => Decoded::one(error_end - at, 0xfffd),
        Handler::BackslashReplace => {
            let mut points = [0; 4];
            let mut used = 0;
            crate::codec::escape(bad, &mut |point| {
                points[used] = point;
                used += 1;
            });
            Decoded {
                length: 1,
                points,
                count: 4,
            }
        }
        _ => Decoded::one(1, 0xdc00 + bad),
    }
}

impl Reader {
    /// Returns a reader that has read nothing and decodes as `decoding` says.
    pub(crate) const fn with(decoding: Decoding) -> Self {
        Self {
            bytes: [0; CHUNK + 4],
            start: 0,
            valid: 0,
            end: 0,
            at_end: false,
            decoding,
            pending: [0; 4],
            pending_start: 0,
            pending_end: 0,
            iterating: false,
        }
    }

    /// Returns a reader that has read nothing and escapes a byte that is not part of a
    /// character, as standard input does.
    pub(crate) const fn escaping() -> Self {
        Self::with(Decoding {
            handler: Handler::SurrogateEscape,
            ..Decoding::DEFAULT
        })
    }

    /// Returns the number of bytes read from the source and not given out yet.
    pub(crate) const fn unread(&self) -> usize {
        self.end - self.start
    }

    /// Drops what was read and not given out, as a `seek` does.
    pub(crate) const fn reset(&mut self) {
        self.start = 0;
        self.valid = 0;
        self.end = 0;
        self.at_end = false;
        self.pending_start = 0;
        self.pending_end = 0;
        self.iterating = false;
    }

    /// Returns whether a `for` loop has turned `tell` off.
    pub(crate) const fn iterating(&self) -> bool {
        self.iterating
    }

    /// Records that a `for` loop took a line, or reached the end when `line` is false.
    pub(crate) const fn set_iterating(&mut self, line: bool) {
        self.iterating = line;
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
        match check(&self.bytes[tail..self.end], self.at_end, self.decoding) {
            Ok(length) => {
                self.valid = tail + length;
                Ok(())
            }
            Err((start, end, reason)) => {
                let error = chunk_decode_error(
                    self.decoding.codec,
                    self.decoding.handler,
                    &self.bytes[tail..self.end],
                    start,
                    end,
                    reason,
                );
                self.end = tail;
                Err(error)
            }
        }
    }

    /// Returns whether the byte after `self.start` is still to be read, so a `\r` there waits.
    const fn waits(&self) -> bool {
        self.start + 1 == self.end && !self.at_end
    }

    /// Returns up to `limit` characters, all of them when `limit` is `None`, ending after the
    /// first line ending when `line`; an empty string at the end of the source.
    pub(crate) fn read_text(
        &mut self,
        limit: Option<usize>,
        line: bool,
        source: Source<'_>,
    ) -> Result<*mut StrObject, *mut ExceptionObject> {
        let mut out = StrBuilder::with_capacity(80);
        let mut count = 0;
        while self.pending_start < self.pending_end {
            if limit == Some(count) {
                return Ok(out.finish());
            }
            out.push(self.pending[usize::from(self.pending_start)]);
            self.pending_start += 1;
            count += 1;
        }
        loop {
            while self.start < self.valid {
                if limit == Some(count) {
                    return Ok(out.finish());
                }
                match self.line_byte(line) {
                    Some(Ending::Wait) => break,
                    Some(Ending::Pair) => {
                        out.push(u32::from(b'\r'));
                        self.start += 1;
                        if limit == Some(count + 1) {
                            return Ok(out.finish());
                        }
                        out.push(u32::from(b'\n'));
                        self.start += 1;
                        return Ok(out.finish());
                    }
                    Some(Ending::Point(point, ends)) => {
                        out.push(point);
                        count += 1;
                        if ends {
                            return Ok(out.finish());
                        }
                        continue;
                    }
                    None => {}
                }
                let decoded = decode_at(&self.bytes[..self.valid], self.start, self.decoding);
                self.start += decoded.length;
                for (index, point) in decoded.points[..usize::from(decoded.count)]
                    .iter()
                    .enumerate()
                {
                    if limit == Some(count) {
                        // A size cut an escape: the rest waits for the next read.
                        let rest = &decoded.points[index..usize::from(decoded.count)];
                        self.pending[..rest.len()].copy_from_slice(rest);
                        self.pending_start = 0;
                        self.pending_end = u8::try_from(rest.len()).unwrap_or(0);
                        return Ok(out.finish());
                    }
                    out.push(*point);
                    count += 1;
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

    /// Reads a `\r` or `\n` at `self.start` as the newline mode says: the character it gives
    /// and whether it ends the line when reading a `line`, `Pair` for a `\r\n` that ends it,
    /// `Wait` when a `\r` needs the next byte, or `None` for any other byte.
    fn line_byte(&mut self, line: bool) -> Option<Ending> {
        let byte = self.bytes[self.start];
        let newline = self.decoding.newline;
        if byte == b'\n' {
            self.start += 1;
            let ends = line && !matches!(newline, Newline::Cr | Newline::CrLf);
            return Some(Ending::Point(u32::from(b'\n'), ends));
        }
        if byte != b'\r' {
            return None;
        }
        let needs_next = match newline {
            Newline::Universal => true,
            Newline::Untranslated | Newline::CrLf => line,
            Newline::Lf | Newline::Cr => false,
        };
        if needs_next && self.waits() {
            return Some(Ending::Wait);
        }
        let followed = self.start + 1 < self.valid && self.bytes[self.start + 1] == b'\n';
        if line && followed && matches!(newline, Newline::Untranslated | Newline::CrLf) {
            return Some(Ending::Pair);
        }
        self.start += 1;
        Some(match newline {
            Newline::Universal => {
                if followed {
                    self.start += 1;
                }
                Ending::Point(u32::from(b'\n'), line)
            }
            Newline::Untranslated | Newline::Cr => Ending::Point(u32::from(b'\r'), line),
            Newline::CrLf | Newline::Lf => Ending::Point(u32::from(b'\r'), false),
        })
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
        if let Err((start, end, reason)) = check(&bytes[decoded..], true, self.decoding) {
            let error = chunk_decode_error(
                self.decoding.codec,
                self.decoding.handler,
                &bytes[decoded..],
                start,
                end,
                reason,
            );
            all.discard();
            return Err(error);
        }
        let mut out = StrBuilder::with_capacity(bytes.len());
        while self.pending_start < self.pending_end {
            out.push(self.pending[usize::from(self.pending_start)]);
            self.pending_start += 1;
        }
        let translate = self.decoding.newline == Newline::Universal;
        let mut at = 0;
        while at < bytes.len() {
            if translate && bytes[at] == b'\r' {
                at += 1;
                if bytes.get(at) == Some(&b'\n') {
                    at += 1;
                }
                out.push(u32::from(b'\n'));
                continue;
            }
            let found = decode_at(bytes, at, self.decoding);
            at += found.length;
            found.points[..usize::from(found.count)]
                .iter()
                .for_each(|point| out.push(*point));
        }
        all.discard();
        Ok(out.finish())
    }
}

/// What a `\r` or `\n` gives a read.
enum Ending {
    /// The character, and whether it ends the line.
    Point(u32, bool),
    /// A `\r` whose next byte is not read yet.
    Wait,
    /// A `\r\n` that ends the line, which a size may cut after its `\r`.
    Pair,
}
