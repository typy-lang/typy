//! Compares the literal keys of a mapping pattern as CPython's compiler does, and writes a
//! key as `repr` would for its `SyntaxError` (CST-06).

use std::fmt::Write as _;

use typy_source::syntax::{Constant, Expr, UnaryOperator};

/// A literal key after folding a leading `-` into the number.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Key {
    None,
    Bool(bool),
    /// Decimal digits with an optional leading `-`.
    Int(String),
    Float(f64),
    Str(String),
    Bytes(Vec<u8>),
}

/// Returns the literal key `key` stands for, or `None` for a dotted name, which CPython
/// compares only when the program runs.
pub(super) fn literal_key(key: &Expr) -> Option<Key> {
    match key {
        Expr::Constant { value, .. } => match value {
            Constant::None => Some(Key::None),
            Constant::Bool(value) => Some(Key::Bool(*value)),
            Constant::Int(digits) => Some(Key::Int(digits.clone())),
            Constant::Float(value) => Some(Key::Float(*value)),
            Constant::Str(text) => Some(Key::Str(text.clone())),
            Constant::Bytes(bytes) => Some(Key::Bytes(bytes.clone())),
            // CPython compares such keys too; TyPy leaves them to the program's run.
            Constant::StrPoints(_) | Constant::Complex { .. } | Constant::Ellipsis => None,
        },
        Expr::UnaryOp {
            operator: UnaryOperator::USub,
            operand,
            ..
        } => match literal_key(operand)? {
            Key::Int(digits) => Some(Key::Int(format!("-{digits}"))),
            Key::Float(value) => Some(Key::Float(-value)),
            _ => None,
        },
        _ => None,
    }
}

/// Returns whether two keys are equal in CPython, where `1`, `1.0` and `True` are one key.
pub(super) fn same_key(left: &Key, right: &Key) -> bool {
    match (number(left), number(right)) {
        (Some(left), Some(right)) => left.equals(&right),
        (None, None) => left == right,
        _ => false,
    }
}

/// A number key: an `int` by its digits, kept exact, or a `float`.
enum Number<'key> {
    Int(&'key str),
    Float(f64),
}

impl Number<'_> {
    #[expect(
        clippy::float_cmp,
        reason = "Python compares float keys exactly, so 0.0 and -0.0 are one key"
    )]
    fn equals(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int(left), Self::Int(right)) => canonical(left) == canonical(right),
            (Self::Float(left), Self::Float(right)) => left == right,
            (Self::Int(digits), Self::Float(value)) | (Self::Float(value), Self::Int(digits)) => {
                value.fract() == 0.0 && canonical(digits) == canonical(&format!("{value:.0}"))
            }
        }
    }
}

fn number(key: &Key) -> Option<Number<'_>> {
    match key {
        Key::Bool(false) => Some(Number::Int("0")),
        Key::Bool(true) => Some(Number::Int("1")),
        Key::Int(digits) => Some(Number::Int(digits)),
        Key::Float(value) => Some(Number::Float(*value)),
        _ => None,
    }
}

/// Returns the digits without leading zeros, and `0` without a sign.
fn canonical(digits: &str) -> String {
    let (sign, digits) = digits
        .strip_prefix('-')
        .map_or(("", digits), |rest| ("-", rest));
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        "0".to_owned()
    } else {
        format!("{sign}{digits}")
    }
}

/// Returns the key as CPython's `repr` writes it.
pub(super) fn key_repr(key: &Key) -> String {
    match key {
        Key::None => "None".to_owned(),
        Key::Bool(true) => "True".to_owned(),
        Key::Bool(false) => "False".to_owned(),
        Key::Int(digits) => canonical(digits),
        Key::Float(value) => float_repr(*value),
        Key::Str(text) => quoted(text.chars(), false),
        Key::Bytes(bytes) => quoted(bytes.iter().copied().map(char::from), true),
    }
}

/// Writes a finite float as `repr` does: the shortest digits that read back, with `.0` on a
/// whole number and an exponent from 1e16 up or below 1e-4.
fn float_repr(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_owned();
    }
    if value.is_infinite() {
        return if value < 0.0 { "-inf" } else { "inf" }.to_owned();
    }
    let size = value.abs();
    if size != 0.0 && !(1e-4..1e16).contains(&size) {
        let written = format!("{value:e}");
        let (mantissa, exponent) = written.split_once('e').unwrap_or((&written, "0"));
        let (sign, exponent) = exponent
            .strip_prefix('-')
            .map_or(("+", exponent), |rest| ("-", rest));
        return format!("{mantissa}e{sign}{exponent:0>2}");
    }
    let written = format!("{value}");
    if written.contains('.') {
        written
    } else {
        format!("{written}.0")
    }
}

/// Quotes text or bytes as `repr` does: single quotes unless the text holds a single quote
/// and no double quote, with escapes for the quote, backslash and control characters. Bytes
/// come as the characters U+0000 to U+00FF.
fn quoted(characters: impl Iterator<Item = char> + Clone, bytes: bool) -> String {
    let quote = if characters.clone().any(|each| each == '\'')
        && !characters.clone().any(|each| each == '"')
    {
        '"'
    } else {
        '\''
    };
    let mut written = String::from(if bytes { "b" } else { "" });
    written.push(quote);
    for each in characters {
        let code = u32::from(each);
        match each {
            '\\' => written.push_str("\\\\"),
            '\n' => written.push_str("\\n"),
            '\r' => written.push_str("\\r"),
            '\t' => written.push_str("\\t"),
            _ if each == quote => {
                written.push('\\');
                written.push(each);
            }
            _ if code < 0x20 || code == 0x7f || (bytes && code > 0x7f) => {
                // Writing to a `String` cannot fail.
                let _ = write!(written, "\\x{code:02x}");
            }
            _ => written.push(each),
        }
    }
    written.push(quote);
    written
}

#[cfg(test)]
mod tests {
    use super::{Key, float_repr, key_repr, same_key};

    #[test]
    fn one_true_and_one_point_zero_are_one_key() {
        let one = Key::Int("1".to_owned());
        assert!(same_key(&one, &Key::Bool(true)));
        assert!(same_key(&Key::Float(1.0), &one));
        assert!(same_key(&Key::Int("-0".to_owned()), &Key::Bool(false)));
        assert!(!same_key(&Key::Float(1.5), &one));
        assert!(!same_key(&Key::Str("1".to_owned()), &one));
        assert!(same_key(&Key::None, &Key::None));
    }

    #[test]
    fn keys_are_written_as_repr_writes_them() {
        let cases = [
            (Key::Str("a".to_owned()), "'a'"),
            (Key::Str("it's".to_owned()), "\"it's\""),
            (Key::Str("'\"\n".to_owned()), "'\\'\"\\n'"),
            (Key::Bytes(vec![b'a', 0xff, b'\\']), "b'a\\xff\\\\'"),
            (
                Key::Str("\u{1}\u{7f}\u{e9}".to_owned()),
                "'\\x01\\x7f\u{e9}'",
            ),
            (Key::Int("007".to_owned()), "7"),
            (Key::Bool(true), "True"),
            (Key::None, "None"),
        ];
        for (key, expected) in cases {
            assert_eq!(key_repr(&key), expected);
        }
    }

    #[test]
    fn floats_are_written_as_repr_writes_them() {
        let cases = [
            (1.0, "1.0"),
            (0.0, "0.0"),
            (-0.0, "-0.0"),
            (0.1, "0.1"),
            (-2.5, "-2.5"),
            (1e16, "1e+16"),
            (1.5e-5, "1.5e-05"),
            (f64::INFINITY, "inf"),
            (f64::NEG_INFINITY, "-inf"),
            (f64::NAN, "nan"),
        ];
        for (value, expected) in cases {
            assert_eq!(float_repr(value), expected);
        }
    }
}
