//! Unicode properties the runtime needs: which code points `repr` shows unescaped, the value of
//! a decimal digit in any script, the properties the `is` methods of `str` test, and the case
//! mappings, from CPython 3.15's Unicode data (OI-15).

#[rustfmt::skip]
mod case_tables;
#[rustfmt::skip]
mod tables;

use tables::{DECIMAL_ZEROS, PRINTABLE_BASIC, PRINTABLE_SUPPLEMENTARY};

/// Returns how many entries of a sorted list are at or below `point`.
fn count_at_or_below<T: Copy + Into<u32>>(boundaries: &[T], point: u32) -> usize {
    boundaries.partition_point(|boundary| (*boundary).into() <= point)
}

/// Returns true when `str.isprintable` holds for the code point, so `repr` shows it unescaped:
/// the categories Cc, Cf, Cs, Co, Cn, Zl, Zp and Zs other than the space are not printable.
pub(crate) fn is_printable(point: u32) -> bool {
    // Printability changes at each boundary, starting from not printable at U+0000.
    let changes = if point <= 0xFFFF {
        count_at_or_below(&PRINTABLE_BASIC, point)
    } else {
        PRINTABLE_BASIC.len() + count_at_or_below(&PRINTABLE_SUPPLEMENTARY, point)
    };
    changes % 2 == 1
}

/// Returns the value of a decimal digit (category Nd) of any script, as `int()` and `float()`
/// read it: `'7'`, `'\u{667}'` and `'\u{ff17}'` are all 7.
pub(crate) fn decimal_value(point: u32) -> Option<u32> {
    let below = count_at_or_below(&DECIMAL_ZEROS, point);
    let zero = DECIMAL_ZEROS.get(below.checked_sub(1)?)?;
    let value = point - zero;
    (value < 10).then_some(value)
}

/// A property of single code points that the `is` methods of `str` test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Property {
    /// `str.isalpha`.
    Alpha,
    /// `str.isdecimal`.
    Decimal,
    /// `str.isdigit`.
    Digit,
    /// `str.isnumeric`.
    Numeric,
    /// `str.isspace`.
    Space,
    /// An uppercase code point.
    Upper,
    /// A lowercase code point.
    Lower,
    /// A titlecase code point.
    Title,
    /// A code point that may start an identifier, `_` included.
    IdentifierStart,
    /// A code point that may continue an identifier.
    IdentifierContinue,
    /// A cased code point.
    Cased,
    /// A code point `str.lower` skips when it looks for a cased neighbour of a capital sigma.
    CaseIgnorable,
}

/// Returns whether the code point has the property.
pub(crate) fn has(property: Property, point: u32) -> bool {
    if let Ok(byte) = u8::try_from(point)
        && byte.is_ascii()
    {
        return ascii_has(property, byte);
    }
    table_has(property, point)
}

/// Returns whether an ASCII character has the property, as the tables say of it: most text is
/// ASCII, so the `is` and case methods test it without a search (NFR-21).
fn ascii_has(property: Property, byte: u8) -> bool {
    match property {
        Property::Alpha | Property::Cased => byte.is_ascii_alphabetic(),
        Property::Decimal | Property::Digit | Property::Numeric => byte.is_ascii_digit(),
        // CPython's `isspace` also holds for the separators U+001C to U+001F.
        Property::Space => matches!(byte, b'\t'..=b'\r' | 0x1c..=0x1f | b' '),
        Property::Upper => byte.is_ascii_uppercase(),
        Property::Lower => byte.is_ascii_lowercase(),
        Property::Title => false,
        Property::IdentifierStart => byte.is_ascii_alphabetic() || byte == b'_',
        Property::IdentifierContinue => byte.is_ascii_alphanumeric() || byte == b'_',
        Property::CaseIgnorable => matches!(byte, b'\'' | b'.' | b':' | b'^' | b'`'),
    }
}

/// Returns whether the code point has the property, by the tables.
fn table_has(property: Property, point: u32) -> bool {
    use case_tables::{
        ALPHA, CASE_IGNORABLE, CASED, DECIMAL, DIGIT, IDENTIFIER_CONTINUE, IDENTIFIER_START, LOWER,
        NUMERIC, SPACE, TITLE, UPPER,
    };
    let boundaries: &[u32] = match property {
        Property::Alpha => &ALPHA,
        Property::Decimal => &DECIMAL,
        Property::Digit => &DIGIT,
        Property::Numeric => &NUMERIC,
        Property::Space => &SPACE,
        Property::Upper => &UPPER,
        Property::Lower => &LOWER,
        Property::Title => &TITLE,
        Property::IdentifierStart => &IDENTIFIER_START,
        Property::IdentifierContinue => &IDENTIFIER_CONTINUE,
        Property::Cased => &CASED,
        Property::CaseIgnorable => &CASE_IGNORABLE,
    };
    count_at_or_below(boundaries, point) % 2 == 1
}

/// A case mapping of single code points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mapping {
    /// `str.upper`.
    Upper,
    /// `str.lower`, apart from a capital sigma, which depends on its neighbours.
    Lower,
    /// The titlecase of one code point.
    Title,
    /// `str.casefold`.
    Fold,
}

/// A run of a one-to-one case mapping: the first and last code point, the stride between the
/// code points it maps, and the difference it adds.
type Run = (u32, u32, u32, i32);

/// A code point a case mapping turns into more than one, and those, padded with 0.
type Multiple = (u32, [u32; 3]);

/// Gives each code point the mapping turns `point` into, one to three of them.
pub(crate) fn map(mapping: Mapping, point: u32, out: &mut impl FnMut(u32)) {
    if let Ok(byte) = u8::try_from(point)
        && byte.is_ascii()
    {
        out(u32::from(ascii_mapped(mapping, byte)));
        return;
    }
    table_map(mapping, point, out);
}

/// Returns the one character an ASCII character maps to, as the tables map it.
const fn ascii_mapped(mapping: Mapping, byte: u8) -> u8 {
    match mapping {
        Mapping::Upper | Mapping::Title => byte.to_ascii_uppercase(),
        Mapping::Lower | Mapping::Fold => byte.to_ascii_lowercase(),
    }
}

/// Writes the code points `point` maps to, by the tables.
fn table_map(mapping: Mapping, point: u32, out: &mut impl FnMut(u32)) {
    use case_tables::{
        FOLD_MULTIPLE, FOLD_RUNS, LOWER_MULTIPLE, LOWER_RUNS, TITLE_MULTIPLE, TITLE_RUNS,
        UPPER_MULTIPLE, UPPER_RUNS,
    };
    let (runs, multiple): (&[Run], &[Multiple]) = match mapping {
        Mapping::Upper => (&UPPER_RUNS, &UPPER_MULTIPLE),
        Mapping::Lower => (&LOWER_RUNS, &LOWER_MULTIPLE),
        Mapping::Title => (&TITLE_RUNS, &TITLE_MULTIPLE),
        Mapping::Fold => (&FOLD_RUNS, &FOLD_MULTIPLE),
    };
    if let Ok(found) = multiple.binary_search_by_key(&point, |(first, _)| *first) {
        multiple[found]
            .1
            .iter()
            .filter(|each| **each != 0)
            .for_each(|each| out(*each));
        return;
    }
    let after = runs.partition_point(|(first, ..)| *first <= point);
    let mapped = after.checked_sub(1).and_then(|index| {
        let (first, last, stride, delta) = runs[index];
        (point <= last && (point - first).is_multiple_of(stride))
            .then(|| point.wrapping_add_signed(delta))
    });
    out(mapped.unwrap_or(point));
}

#[cfg(test)]
mod tests;
