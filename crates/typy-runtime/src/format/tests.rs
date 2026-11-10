//! Tests of `format`: every case of a table whose results CPython 3.12 to 3.15 agree on, and
//! the cases the table does not reach.

mod cases;

use super::{
    typy_rt_ascii, typy_rt_format_bool, typy_rt_format_float, typy_rt_format_int,
    typy_rt_format_other, typy_rt_format_str,
};
use crate::Raising;
use crate::string::{StrObject, code_point, from_str, length};

/// A value of the table.
#[derive(Clone, Copy, Debug)]
pub(super) enum Value {
    /// An `int`, in decimal.
    Int(&'static str),
    /// A `bool`.
    Bool(bool),
    /// A `float`, by its bits.
    Float(u64),
    /// A `str`.
    Str(&'static str),
}

/// Returns the text of a runtime string, and releases it.
fn text(string: *mut StrObject) -> String {
    // SAFETY: the runtime returns a live string, and every index is in bounds.
    let text = (0..unsafe { length(string) })
        .map(|index| char::from_u32(unsafe { code_point(string, index) }).unwrap_or('?'))
        .collect();
    // SAFETY: the string is no longer used.
    unsafe { crate::refcount::typy_rt_decrement(string.cast()) };
    text
}

/// Returns `ok` and the text, or the exception's type and message, and releases them.
fn outcome(result: Raising<*mut StrObject>) -> (String, String) {
    if result.exception.is_null() {
        return ("ok".to_owned(), text(result.value));
    }
    // SAFETY: the runtime returned a live exception, released here.
    unsafe {
        let kind = (*result.exception).kind.name().to_owned();
        let message = (*result.exception).message;
        crate::refcount::typy_rt_increment(message.cast());
        crate::refcount::typy_rt_decrement(result.exception.cast());
        (kind, text(message))
    }
}

/// Returns `format(value, spec)`.
fn format(value: Value, spec: &str) -> (String, String) {
    let spec = from_str(spec);
    // SAFETY: each string and word is live until it is released at the end.
    let result = unsafe {
        match value {
            Value::Int(digits) => {
                let text = from_str(digits);
                let parsed = crate::int::typy_rt_int_from_str(text, 21);
                assert!(parsed.exception.is_null());
                crate::refcount::typy_rt_decrement(text.cast());
                let result = typy_rt_format_int(parsed.value, spec);
                crate::refcount::typy_rt_int_decrement(parsed.value);
                result
            }
            Value::Bool(value) => typy_rt_format_bool(if value { 3 } else { 1 }, spec),
            Value::Float(bits) => typy_rt_format_float(f64::from_bits(bits), spec),
            Value::Str(value) => {
                let text = from_str(value);
                let result = typy_rt_format_str(text, spec);
                crate::refcount::typy_rt_decrement(text.cast());
                result
            }
        }
    };
    // SAFETY: the specification is no longer used.
    unsafe { crate::refcount::typy_rt_decrement(spec.cast()) };
    outcome(result)
}

#[test]
fn every_value_formats_as_cpython_formats_it() {
    let mut failures = Vec::new();
    // Miri interprets every step of the large-integer arithmetic, so it checks a sample that
    // still reaches every formatter; the native run checks every case.
    let step = if cfg!(miri) { 97 } else { 1 };
    for (value, spec, kind, expected) in cases::CASES.iter().step_by(step) {
        let actual = format(*value, spec);
        if actual != ((*kind).to_owned(), (*expected).to_owned()) {
            failures.push(format!(
                "{value:?} {spec:?}: {actual:?}, not ({kind:?}, {expected:?})"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {}:\n{}",
        failures.len(),
        cases::CASES.len(),
        failures.join("\n")
    );
}

#[test]
fn other_types_format_only_with_an_empty_specification() {
    let value = from_str("[1, 2]");
    let name = from_str("list");
    let empty = from_str("");
    let width = from_str("5");
    // SAFETY: each string is live until it is released at the end.
    unsafe {
        assert_eq!(
            outcome(typy_rt_format_other(value, empty, name)),
            ("ok".to_owned(), "[1, 2]".to_owned())
        );
        assert_eq!(
            outcome(typy_rt_format_other(value, width, name)),
            (
                "TypeError".to_owned(),
                "unsupported format string passed to list.__format__".to_owned()
            )
        );
        for string in [value, name, empty, width] {
            crate::refcount::typy_rt_decrement(string.cast());
        }
    }
}

#[test]
fn ascii_escapes_every_code_point_outside_ascii() {
    let text = from_str("'a\u{e9}\u{3b1}\u{1f600}'");
    // SAFETY: the string is live until it is released at the end.
    let escaped = unsafe { typy_rt_ascii(text) };
    assert_eq!(super::tests::text(escaped), "'a\\xe9\\u03b1\\U0001f600'");
    // SAFETY: the string is no longer used.
    unsafe { crate::refcount::typy_rt_decrement(text.cast()) };
}
