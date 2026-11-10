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
mod tests {
    use super::{Mapping, Property, decimal_value, has, is_printable, map};

    #[test]
    fn printability_matches_cpython() {
        let not_printable = [
            0x0, 0x1f, 0x7f, 0x85, 0xa0, 0xad, 0x378, 0x61c, 0x200b, 0x2028, 0x3000, 0xd800,
            0xe000, 0xfeff, 0xffff, 0xe0001, 0xf0000, 0x10_ffff,
        ];
        for point in not_printable {
            assert!(!is_printable(point), "{point:#x}");
        }
        let printable = [
            0x20, 0x41, 0x7e, 0xa1, 0xe9, 0x3b1, 0x4e2d, 0xfffd, 0x1_0000, 0x1_1f00, 0x1_f600,
            0x1_fae8, 0x2_0000,
        ];
        for point in printable {
            assert!(is_printable(point), "{point:#x}");
        }
    }

    #[test]
    fn decimal_digits_of_every_script_have_their_value() {
        let cases = [
            (0x30, Some(0)),
            (0x39, Some(9)),
            (0x660, Some(0)),
            (0x667, Some(7)),
            (0xff17, Some(7)),
            (0x1_d7ff, Some(9)),
            (0x2f, None),
            (0x3a, None),
            (0x41, None),
            (0xb2, None),
            (0x0, None),
            (0x10_ffff, None),
        ];
        for (point, value) in cases {
            assert_eq!(decimal_value(point), value, "{point:#x}");
        }
    }

    fn mapped(mapping: Mapping, point: u32) -> Vec<u32> {
        let mut out = Vec::new();
        map(mapping, point, &mut |each| out.push(each));
        out
    }

    #[test]
    fn case_mappings_follow_cpython() {
        let cases: [(Mapping, u32, &[u32]); 12] = [
            (Mapping::Upper, 0x61, &[0x41]),
            (Mapping::Upper, 0x41, &[0x41]),
            (Mapping::Upper, 0xdf, &[0x53, 0x53]),
            (Mapping::Upper, 0x101, &[0x100]),
            (Mapping::Upper, 0x100, &[0x100]),
            (Mapping::Lower, 0x130, &[0x69, 0x307]),
            (Mapping::Lower, 0x41, &[0x61]),
            (Mapping::Title, 0x1c6, &[0x1c5]),
            (Mapping::Title, 0x1c4, &[0x1c5]),
            (Mapping::Fold, 0xdf, &[0x73, 0x73]),
            (Mapping::Fold, 0x3a3, &[0x3c3]),
            (Mapping::Upper, 0x10_ffff, &[0x10_ffff]),
        ];
        for (mapping, point, expected) in cases {
            assert_eq!(mapped(mapping, point), expected, "{mapping:?} {point:#x}");
        }
    }

    #[test]
    fn properties_follow_cpython() {
        let cases = [
            (Property::Alpha, 0x41, true),
            (Property::Alpha, 0x31, false),
            (Property::Decimal, 0x661, true),
            (Property::Digit, 0xb2, true),
            (Property::Decimal, 0xb2, false),
            (Property::Numeric, 0xbd, true),
            (Property::Space, 0x1f, true),
            (Property::Space, 0x200b, false),
            (Property::Upper, 0x41, true),
            (Property::Lower, 0x61, true),
            (Property::Title, 0x1c5, true),
            (Property::IdentifierStart, 0x5f, true),
            (Property::IdentifierStart, 0x31, false),
            (Property::IdentifierContinue, 0x31, true),
            (Property::Cased, 0x345, true),
            (Property::CaseIgnorable, 0x345, true),
            (Property::CaseIgnorable, 0x27, true),
            (Property::Cased, 0x27, false),
        ];
        for (property, point, expected) in cases {
            assert_eq!(has(property, point), expected, "{property:?} {point:#x}");
        }
    }
}
