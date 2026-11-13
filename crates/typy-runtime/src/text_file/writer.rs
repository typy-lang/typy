//! Writing a text file as CPython's `TextIOWrapper` writes it (ADR-0019): text is encoded as
//! UTF-8 whole, so a surrogate raises before anything is written, and the bytes wait in a
//! buffer of 8192 until it fills, the file is flushed or closed, or the program ends.

use crate::exception::ExceptionObject;
use crate::string::{StrObject, code_point, length};

/// The bytes a file holds before it writes them out.
pub(crate) const CAPACITY: usize = 8192;

/// Writes all of the bytes it is given, or returns the error number.
pub(crate) type Sink<'sink> = &'sink mut dyn FnMut(&[u8]) -> Result<(), i32>;

/// What a writable file has been given and not written out yet.
pub(crate) struct Writer {
    bytes: [u8; CAPACITY],
    used: usize,
}

/// Returns the first run of surrogates in `text`, which UTF-8 cannot encode.
///
/// # Safety
///
/// `text` points to a live `str`.
unsafe fn surrogates(text: *const StrObject) -> Option<(usize, usize)> {
    let is_surrogate = |index: usize| {
        // SAFETY: the caller's guarantee; the index is below the length.
        (0xd800..=0xdfff).contains(&unsafe { code_point(text, index) })
    };
    // SAFETY: the caller's guarantee.
    let count = unsafe { length(text) };
    let start = (0..count).find(|index| is_surrogate(*index))?;
    let end = (start..count)
        .find(|index| !is_surrogate(*index))
        .unwrap_or(count);
    Some((start, end))
}

impl Writer {
    /// Returns an empty writer.
    pub(crate) const fn new() -> Self {
        Self {
            bytes: [0; CAPACITY],
            used: 0,
        }
    }

    /// Adds `text` in UTF-8, writing out the buffer through `sink` whenever it fills.
    ///
    /// # Safety
    ///
    /// `text` points to a live `str`.
    pub(crate) unsafe fn write(
        &mut self,
        text: *const StrObject,
        sink: Sink<'_>,
    ) -> Result<(), *mut ExceptionObject> {
        // SAFETY: the caller's guarantee.
        if let Some((start, end)) = unsafe { surrogates(text) } {
            // SAFETY: the caller's guarantee; the surrogates run from `start` to `end`.
            return Err(unsafe { crate::codec::utf8_encode_error(text, start, end) });
        }
        // SAFETY: the caller's guarantee.
        for index in 0..unsafe { length(text) } {
            if self.used + 4 > CAPACITY {
                self.flush(sink)?;
            }
            // SAFETY: the index is below the length; the text has no surrogates.
            let point = unsafe { code_point(text, index) };
            let character = char::from_u32(point).unwrap_or('\u{fffd}');
            self.used += character.encode_utf8(&mut self.bytes[self.used..]).len();
        }
        Ok(())
    }

    /// Writes out what the buffer holds.
    pub(crate) fn flush(&mut self, sink: Sink<'_>) -> Result<(), *mut ExceptionObject> {
        let used = core::mem::replace(&mut self.used, 0);
        // SAFETY: an error of writing names no file.
        sink(&self.bytes[..used])
            .map_err(|errno| unsafe { super::os_error::os_error(errno, core::ptr::null()) })
    }
}
