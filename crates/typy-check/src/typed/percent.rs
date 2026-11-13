//! `format % values` with a format known when the program is checked (LIB-10).

use crate::types::Type;

/// One piece of a formatted string: literal text, or one value formatted.
#[derive(Clone, Debug, PartialEq)]
pub enum PercentPart {
    /// Text written as it is.
    Literal(String),
    /// One conversion of one value.
    Field(PercentField),
}

/// One conversion: where its value comes from, its flags, width and precision, and how it
/// turns the value into text.
#[derive(Clone, Debug, PartialEq)]
pub struct PercentField {
    /// Where the value comes from.
    pub value: PercentValue,
    /// The flags as written, from `-+ #0`.
    pub flags: String,
    /// The least number of characters.
    pub width: PercentNumber,
    /// The digits after the point, the least digits of an integer, or the characters kept
    /// of a text.
    pub precision: PercentNumber,
    /// How the value becomes text.
    pub conversion: PercentConversion,
    /// The type of the value.
    pub ty: Type,
    /// How an error names the value: `format argument`, `format argument 2` or
    /// `format argument 'key'`, as CPython 3.15 names it.
    pub label: String,
}

/// Where a conversion's value comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PercentValue {
    /// The right operand itself, when it is not a tuple.
    Whole,
    /// An item of the right operand, a tuple.
    Item(usize),
    /// The value of a key of the right operand, a dict: `%(key)s`.
    Key(String),
}

/// A width or a precision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PercentNumber {
    /// Not written.
    Absent,
    /// Written as digits.
    Given(usize),
    /// `*`: an `int` item of the right operand gives it.
    Item(usize),
}

/// How a conversion turns its value into text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PercentConversion {
    /// `%s`: `str(value)`.
    Str,
    /// `%r`: `repr(value)`.
    Repr,
    /// `%a`: `ascii(value)`.
    Ascii,
    /// `%d`, `%i`, `%u`, `%x`, `%X` or `%o` of an `int`, with the character `d`, `x`, `X` or
    /// `o`.
    Integer(u8),
    /// `%e`, `%E`, `%f`, `%F`, `%g` or `%G` of a `float`, with its character.
    Float(u8),
    /// `%c`: the character of a code point, or a one-character `str`.
    Char,
}
