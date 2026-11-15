//! Writing a text file as CPython's `TextIOWrapper` writes it (ADR-0019, LIB-14): text has
//! each `\n` translated as the newline mode says and is encoded whole, so an error raises
//! before anything is written; the bytes wait in a buffer of 8192 until it fills, the file is
//! flushed or closed, or the program ends. A line-buffered file is flushed after each write of
//! a `\n` or a `\r`.

use super::newline::Newline;
use crate::codec::{Codec, Handler};
use crate::exception::ExceptionObject;
use crate::string::{StrObject, code_point, length};

/// The bytes a file holds before it writes them out.
pub(crate) const CAPACITY: usize = 8192;

/// Writes all of the bytes it is given, or returns the error number.
pub(crate) type Sink<'sink> = &'sink mut dyn FnMut(&[u8]) -> Result<(), i32>;

/// How a file encodes what it is given.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Encoding {
    pub(crate) codec: Codec,
    pub(crate) handler: Handler,
    pub(crate) newline: Newline,
    /// `buffering=1`: each write of a `\n` or a `\r` is written out.
    pub(crate) line_buffering: bool,
}

#[cfg(test)]
impl Encoding {
    /// UTF-8, strict, `\n` as it is, and a full buffer: what `open` gives by default.
    pub(crate) const DEFAULT: Self = Self {
        codec: Codec::Utf8,
        handler: Handler::Strict,
        newline: Newline::Universal,
        line_buffering: false,
    };
}

/// What a writable file has been given and not written out yet.
pub(crate) struct Writer {
    bytes: [u8; CAPACITY],
    used: usize,
    encoding: Encoding,
}

impl Writer {
    /// Returns an empty writer that encodes as `encoding` says.
    pub(crate) const fn with(encoding: Encoding) -> Self {
        Self {
            bytes: [0; CAPACITY],
            used: 0,
            encoding,
        }
    }

    /// Adds `text`, translated and encoded, writing out the buffer through `sink` whenever it
    /// fills, and at the end when the file is line-buffered and `text` has a line ending.
    ///
    /// # Safety
    ///
    /// `text` points to a live `str`.
    pub(crate) unsafe fn write(
        &mut self,
        text: *const StrObject,
        sink: Sink<'_>,
    ) -> Result<(), *mut ExceptionObject> {
        let encoding = self.encoding;
        // SAFETY: the caller's guarantee; the translated text and the bytes are released once.
        unsafe {
            let translated = encoding.newline.translated(text);
            let encoded = crate::codec::encode(translated, encoding.codec, encoding.handler);
            crate::refcount::typy_rt_decrement(translated.cast());
            let bytes = encoded?;
            let added = self.add(bytes, sink);
            crate::refcount::typy_rt_decrement(bytes.cast());
            added?;
            let count = length(text);
            let ends_line = (0..count).any(|index| matches!(code_point(text, index), 0x0a | 0x0d));
            if encoding.line_buffering && ends_line {
                self.flush(sink)?;
            }
        }
        Ok(())
    }

    /// Adds the bytes of a `bytes`, writing out the buffer whenever it fills.
    ///
    /// # Safety
    ///
    /// `bytes` points to a live `bytes`.
    pub(crate) unsafe fn add(
        &mut self,
        bytes: *const StrObject,
        sink: Sink<'_>,
    ) -> Result<(), *mut ExceptionObject> {
        // SAFETY: the caller's guarantee; each index is below the length.
        unsafe {
            for index in 0..length(bytes) {
                if self.used == CAPACITY {
                    self.flush(sink)?;
                }
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "an item of a bytes is below 256"
                )]
                let byte = code_point(bytes, index) as u8;
                self.bytes[self.used] = byte;
                self.used += 1;
            }
        }
        Ok(())
    }

    /// Returns the number of bytes given and not written out yet.
    pub(crate) const fn pending(&self) -> usize {
        self.used
    }

    /// Writes out what the buffer holds.
    pub(crate) fn flush(&mut self, sink: Sink<'_>) -> Result<(), *mut ExceptionObject> {
        let used = core::mem::replace(&mut self.used, 0);
        // SAFETY: an error of writing names no file.
        sink(&self.bytes[..used])
            .map_err(|errno| unsafe { super::os_error::os_error(errno, core::ptr::null()) })
    }
}
