//! `str.encode` and `bytes.decode` with the codecs `utf-8`, `ascii` and `latin-1`, and the
//! error handlers `strict`, `ignore`, `replace`, `backslashreplace`, `xmlcharrefreplace` and
//! `surrogateescape`, with CPython's results and messages (LIB-03). The checker resolves the
//! names, so the runtime receives their numbers.

use crate::Raising;
use crate::exception::{ExceptionKind, ExceptionObject};
use crate::list::ItemKind;
use crate::message::MessageBuffer;
use crate::string::{StrBuilder, StrObject, code_point, length};
use crate::tuple::{Field, TupleType};
use crate::word::inline_value;

/// A codec, numbered as the compiler passes it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Codec {
    Utf8 = 0,
    Ascii = 1,
    Latin1 = 2,
}

impl Codec {
    fn of(word: i64) -> Self {
        match inline_value(word) {
            1 => Self::Ascii,
            2 => Self::Latin1,
            _ => Self::Utf8,
        }
    }

    /// The name CPython's messages give the codec.
    const fn name(self) -> &'static str {
        match self {
            Self::Utf8 => "utf-8",
            Self::Ascii => "ascii",
            Self::Latin1 => "latin-1",
        }
    }

    /// The first code point the codec cannot encode as one byte, for `ascii` and `latin-1`.
    const fn limit(self) -> u32 {
        match self {
            Self::Ascii => 0x80,
            _ => 0x100,
        }
    }
}

/// An error handler, numbered as the compiler passes it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Handler {
    Strict = 0,
    Ignore = 1,
    Replace = 2,
    BackslashReplace = 3,
    XmlCharRefReplace = 4,
    SurrogateEscape = 5,
}

impl Handler {
    fn of(word: i64) -> Self {
        match inline_value(word) {
            1 => Self::Ignore,
            2 => Self::Replace,
            3 => Self::BackslashReplace,
            4 => Self::XmlCharRefReplace,
            5 => Self::SurrogateEscape,
            _ => Self::Strict,
        }
    }
}

/// A growing buffer of bytes that becomes a `bytes`.
struct ByteBuilder {
    inner: StrBuilder,
}

impl ByteBuilder {
    fn new(capacity: usize) -> Self {
        Self {
            inner: StrBuilder::with_capacity(capacity),
        }
    }

    fn push(&mut self, byte: u32) {
        self.inner.push(byte);
    }

    fn push_ascii(&mut self, text: &[u8]) {
        for byte in text {
            self.push(u32::from(*byte));
        }
    }

    fn discard(self) {
        self.inner.discard();
    }

    fn finish(self) -> *mut StrObject {
        let text = self.inner.finish();
        // SAFETY: the string is new, of one byte per code point, and nothing else refers to it.
        unsafe { (*text).header.type_id = crate::header::BYTES_TYPE_ID };
        text
    }
}

/// Writes `\x`, `\u` or `\U` and the hexadecimal digits of a code point, as CPython escapes one
/// in a message and in `backslashreplace`.
pub(crate) fn escape(point: u32, push: &mut impl FnMut(u32)) {
    let (letter, digits) = if point <= 0xff {
        (b'x', 2)
    } else if point <= 0xffff {
        (b'u', 4)
    } else {
        (b'U', 8)
    };
    push(u32::from(b'\\'));
    push(u32::from(letter));
    for shift in (0..digits).rev() {
        let digit = (point >> (4 * shift)) & 0xf;
        push(u32::from(
            b"0123456789abcdef"[usize::try_from(digit).unwrap_or(0)],
        ));
    }
}

fn encode_error(
    codec: Codec,
    text: *const StrObject,
    start: usize,
    end: usize,
    reason: &str,
) -> *mut ExceptionObject {
    let mut message = MessageBuffer::new();
    message.push_str("'");
    message.push_str(codec.name());
    message.push_str("' codec can't encode ");
    if end == start + 1 {
        message.push_str("character '");
        // SAFETY: the caller passes a live string and a position inside it.
        escape(unsafe { code_point(text, start) }, &mut |point| {
            message.push(point);
        });
        message.push_str("' in position ");
        message.push_number(i64::try_from(start).unwrap_or(i64::MAX));
    } else {
        message.push_str("characters in position ");
        message.push_number(i64::try_from(start).unwrap_or(i64::MAX));
        message.push_str("-");
        message.push_number(i64::try_from(end - 1).unwrap_or(i64::MAX));
    }
    message.push_str(": ");
    message.push_str(reason);
    let exception = message.raise(ExceptionKind::UnicodeEncodeError);
    // SAFETY: the exception is new, and the caller passes a live string.
    unsafe { unicode_arguments(exception, codec, text, start, end, reason) };
    exception
}

fn decode_error(
    codec: Codec,
    data: *const StrObject,
    start: usize,
    end: usize,
    reason: &str,
) -> *mut ExceptionObject {
    let mut message = MessageBuffer::new();
    message.push_str("'");
    message.push_str(codec.name());
    message.push_str("' codec can't decode ");
    if end == start + 1 {
        message.push_str("byte 0x");
        // SAFETY: the caller passes live bytes and a position inside them.
        let byte = unsafe { code_point(data, start) };
        for shift in [4, 0] {
            let digit = (byte >> shift) & 0xf;
            message.push(u32::from(
                b"0123456789abcdef"[usize::try_from(digit).unwrap_or(0)],
            ));
        }
        message.push_str(" in position ");
        message.push_number(i64::try_from(start).unwrap_or(i64::MAX));
    } else {
        message.push_str("bytes in position ");
        message.push_number(i64::try_from(start).unwrap_or(i64::MAX));
        message.push_str("-");
        message.push_number(i64::try_from(end - 1).unwrap_or(i64::MAX));
    }
    message.push_str(": ");
    message.push_str(reason);
    let exception = message.raise(ExceptionKind::UnicodeDecodeError);
    // SAFETY: the exception is new, and the caller passes live bytes.
    unsafe { unicode_arguments(exception, codec, data, start, end, reason) };
    exception
}

/// Returns the `UnicodeDecodeError` of the UTF-8 codec for `data[start..end]`, with a new
/// `bytes` of `data` as its object, as a text file's decoder raises it (ADR-0019).
pub(crate) fn utf8_decode_error(
    data: &[u8],
    start: usize,
    end: usize,
    reason: &str,
) -> *mut ExceptionObject {
    let object = crate::bytes::new_bytes(data.len());
    for (index, byte) in data.iter().enumerate() {
        // SAFETY: the bytes are new and hold `data.len()` bytes.
        unsafe { crate::string::set_code_point(object, index, u32::from(*byte)) };
    }
    let exception = decode_error(Codec::Utf8, object, start, end, reason);
    // SAFETY: the exception holds a reference of its own to the bytes.
    unsafe { crate::refcount::typy_rt_decrement(object.cast()) };
    exception
}

/// Returns the `UnicodeEncodeError` of the UTF-8 codec for the surrogates of `text` from
/// `start` to `end`, as a text file's encoder raises it (ADR-0019).
///
/// # Safety
///
/// `text` points to a live `str` with surrogates from `start` to `end`.
pub(crate) unsafe fn utf8_encode_error(
    text: *const StrObject,
    start: usize,
    end: usize,
) -> *mut ExceptionObject {
    encode_error(Codec::Utf8, text, start, end, "surrogates not allowed")
}

/// The layout of the arguments of a Unicode error: the codec's name, the text or the bytes,
/// the start, the end and the reason.
static UNICODE_ARGUMENTS: TupleType = TupleType {
    size: 40,
    length: 5,
    fields: UNICODE_FIELDS.as_ptr(),
};

static UNICODE_FIELDS: [Field; 5] = [
    Field::plain(ItemKind::Object, 0),
    Field::plain(ItemKind::Object, 8),
    Field::plain(ItemKind::Int, 16),
    Field::plain(ItemKind::Int, 24),
    Field::plain(ItemKind::Object, 32),
];

/// Gives a Unicode error CPython's five arguments, which its `repr` shows:
/// `UnicodeDecodeError('utf-8', b'\xff', 0, 1, 'invalid start byte')`.
///
/// # Safety
///
/// `exception` is new and has no arguments, and `object` points to a live `str` or `bytes`.
unsafe fn unicode_arguments(
    exception: *mut ExceptionObject,
    codec: Codec,
    object: *const StrObject,
    start: usize,
    end: usize,
    reason: &str,
) {
    let position = |value: usize| {
        crate::word::inline(i64::try_from(value).unwrap_or(i64::MAX))
            .unwrap_or(0)
            .cast_unsigned()
    };
    // SAFETY: the caller's guarantee; the exception takes the new references.
    unsafe {
        crate::refcount::typy_rt_increment(object.cast_mut().cast());
        crate::exception::set_arguments(
            exception,
            &UNICODE_ARGUMENTS,
            &[
                crate::string::from_str(codec.name()).expose_provenance() as u64,
                object.expose_provenance() as u64,
                position(start),
                position(end),
                crate::string::from_str(reason).expose_provenance() as u64,
            ],
        );
    }
}

/// Returns the bytes of `point` in UTF-8, which the caller has checked is not a surrogate.
fn utf8(point: u32, out: &mut ByteBuilder) {
    match point {
        0..=0x7f => out.push(point),
        0x80..=0x7ff => {
            out.push(0xc0 | (point >> 6));
            out.push(0x80 | (point & 0x3f));
        }
        0x800..=0xffff => {
            out.push(0xe0 | (point >> 12));
            out.push(0x80 | ((point >> 6) & 0x3f));
            out.push(0x80 | (point & 0x3f));
        }
        _ => {
            out.push(0xf0 | (point >> 18));
            out.push(0x80 | ((point >> 12) & 0x3f));
            out.push(0x80 | ((point >> 6) & 0x3f));
            out.push(0x80 | (point & 0x3f));
        }
    }
}

/// Returns `text.encode(codec, errors)` as a new `bytes`. The code points a codec cannot
/// encode, surrogates for `utf-8` and those past its range for `ascii` and `latin-1`, are
/// handled a run at a time, as CPython does; `strict` raises `UnicodeEncodeError` for the run.
///
/// # Safety
///
/// `text` points to a live string, and `codec` and `handler` are `int` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_encode(
    text: *const StrObject,
    codec: i64,
    handler: i64,
) -> Raising<*mut StrObject> {
    let (codec, handler) = (Codec::of(codec), Handler::of(handler));
    // SAFETY: the caller passes a live string; every index stays inside it.
    unsafe {
        let whole = length(text);
        let encodable = |point: u32| match codec {
            Codec::Utf8 => !(0xd800..=0xdfff).contains(&point),
            _ => point < codec.limit(),
        };
        let reason = match codec {
            Codec::Utf8 => "surrogates not allowed",
            Codec::Ascii => "ordinal not in range(128)",
            Codec::Latin1 => "ordinal not in range(256)",
        };
        let mut out = ByteBuilder::new(whole);
        let mut at = 0;
        while at < whole {
            let point = code_point(text, at);
            if encodable(point) {
                if codec == Codec::Utf8 {
                    utf8(point, &mut out);
                } else {
                    out.push(point);
                }
                at += 1;
                continue;
            }
            let start = at;
            while at < whole && !encodable(code_point(text, at)) {
                at += 1;
            }
            for index in start..at {
                let bad = code_point(text, index);
                match handler {
                    Handler::Ignore => {}
                    Handler::Replace => out.push(u32::from(b'?')),
                    Handler::BackslashReplace => escape(bad, &mut |each| out.push(each)),
                    Handler::XmlCharRefReplace => {
                        out.push_ascii(b"&#");
                        let mut digits = [0_u8; 10];
                        let (mut value, mut used) = (bad, 0);
                        loop {
                            digits[used] = b'0' + u8::try_from(value % 10).unwrap_or(0);
                            used += 1;
                            value /= 10;
                            if value == 0 {
                                break;
                            }
                        }
                        digits[..used].reverse();
                        out.push_ascii(&digits[..used]);
                        out.push(u32::from(b';'));
                    }
                    Handler::SurrogateEscape if (0xdc80..=0xdcff).contains(&bad) => {
                        out.push(bad - 0xdc00);
                    }
                    Handler::SurrogateEscape | Handler::Strict => {
                        out.discard();
                        return Raising::error(
                            core::ptr::null_mut(),
                            encode_error(codec, text, start, at, reason),
                        );
                    }
                }
            }
        }
        Raising::ok(out.finish())
    }
}

/// Returns the length of the valid UTF-8 sequence at `at`, or the end of the error there and
/// its reason, as CPython's decoder finds them: a lead byte outside `C2` to `F4` is an invalid
/// start byte; a byte after it outside the range the lead allows ends an invalid continuation
/// before that byte; the data running out first is an unexpected end of data, to the end.
pub(crate) fn utf8_sequence(
    byte: impl Fn(usize) -> Option<u32>,
    at: usize,
) -> Result<(usize, u32), (usize, &'static str)> {
    let lead = byte(at).unwrap_or(0);
    let (count, second, first_bits) = match lead {
        0x00..=0x7f => return Ok((1, lead)),
        0xc2..=0xdf => (2, 0x80..=0xbf, lead & 0x1f),
        0xe0 => (3, 0xa0..=0xbf, lead & 0x0f),
        0xed => (3, 0x80..=0x9f, lead & 0x0f),
        0xe1..=0xef => (3, 0x80..=0xbf, lead & 0x0f),
        0xf0 => (4, 0x90..=0xbf, lead & 0x07),
        0xf4 => (4, 0x80..=0x8f, lead & 0x07),
        0xf1..=0xf3 => (4, 0x80..=0xbf, lead & 0x07),
        _ => return Err((at + 1, "invalid start byte")),
    };
    let mut point = first_bits;
    for offset in 1..count {
        let Some(next) = byte(at + offset) else {
            return Err((usize::MAX, "unexpected end of data"));
        };
        let allowed = if offset == 1 {
            second.contains(&next)
        } else {
            (0x80..=0xbf).contains(&next)
        };
        if !allowed {
            return Err((at + offset, "invalid continuation byte"));
        }
        point = (point << 6) | (next & 0x3f);
    }
    Ok((count, point))
}

/// Returns `data.decode(codec, errors)` as a new string. An error range is handled as a
/// whole: `replace` gives one U+FFFD for it, `backslashreplace` `\xhh` for each byte,
/// `surrogateescape` U+DC80 to U+DCFF for each byte, and `strict` raises
/// `UnicodeDecodeError`.
///
/// # Safety
///
/// `data` points to a live `bytes`, and `codec` and `handler` are `int` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytes_decode(
    data: *const StrObject,
    codec: i64,
    handler: i64,
) -> Raising<*mut StrObject> {
    let (codec, handler) = (Codec::of(codec), Handler::of(handler));
    // SAFETY: the caller passes live bytes; every index stays inside them.
    unsafe {
        let whole = length(data);
        let byte = |index: usize| (index < whole).then(|| code_point(data, index));
        let mut out = StrBuilder::with_capacity(whole);
        let mut at = 0;
        while at < whole {
            let decoded = match codec {
                Codec::Latin1 => Ok((1, code_point(data, at))),
                Codec::Ascii if code_point(data, at) < 0x80 => Ok((1, code_point(data, at))),
                Codec::Ascii => Err((at + 1, "ordinal not in range(128)")),
                Codec::Utf8 => utf8_sequence(byte, at),
            };
            let (end, reason) = match decoded {
                Ok((count, point)) => {
                    out.push(point);
                    at += count;
                    continue;
                }
                Err((end, reason)) => (end.min(whole), reason),
            };
            match handler {
                Handler::Ignore => {}
                Handler::Replace => out.push(0xfffd),
                Handler::BackslashReplace => {
                    (at..end).for_each(|index| {
                        escape(code_point(data, index), &mut |each| out.push(each));
                    });
                }
                Handler::SurrogateEscape => {
                    (at..end).for_each(|index| out.push(0xdc00 + code_point(data, index)));
                }
                Handler::Strict | Handler::XmlCharRefReplace => {
                    out.discard();
                    return Raising::error(
                        core::ptr::null_mut(),
                        decode_error(codec, data, at, end, reason),
                    );
                }
            }
            at = end;
        }
        Raising::ok(out.finish())
    }
}

#[cfg(test)]
mod tests;
