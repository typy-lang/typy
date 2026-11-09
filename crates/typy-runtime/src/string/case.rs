//! The case methods and the `is` methods of `str`, from CPython 3.15's Unicode data, with
//! CPython's rules for a capital sigma, for title case and for empty strings (LIB-03, OI-15).

use super::builder::StrBuilder;
use super::unicode::{Mapping, Property, has, map};
use super::{StrObject, code_point, length};
use crate::word::inline_value;

const CAPITAL_SIGMA: u32 = 0x3a3;
const FINAL_SIGMA: u32 = 0x3c2;
const SMALL_SIGMA: u32 = 0x3c3;

/// Returns the lowercase of a capital sigma at `index`: final, U+03C2, when a cased code point
/// comes before it and none after it, skipping case-ignorable ones, as CPython's
/// `handle_capital_sigma` decides; U+03C3 otherwise.
///
/// # Safety
///
/// `text` points to a live string, and `index` is below its length.
unsafe fn lower_sigma(text: *const StrObject, index: usize) -> u32 {
    // SAFETY: the caller keeps every index inside the string.
    unsafe {
        let before = (0..index)
            .rev()
            .map(|at| code_point(text, at))
            .find(|point| !has(Property::CaseIgnorable, *point));
        let after = (index + 1..length(text))
            .map(|at| code_point(text, at))
            .find(|point| !has(Property::CaseIgnorable, *point));
        let ends_word = before.is_some_and(|point| has(Property::Cased, point))
            && !after.is_some_and(|point| has(Property::Cased, point));
        if ends_word { FINAL_SIGMA } else { SMALL_SIGMA }
    }
}

/// Writes the lowercase of the code point at `index`, a capital sigma by its neighbours.
///
/// # Safety
///
/// `text` points to a live string, and `index` is below its length.
unsafe fn lower_at(text: *const StrObject, index: usize, builder: &mut StrBuilder) {
    // SAFETY: the caller's guarantee.
    unsafe {
        let point = code_point(text, index);
        if point == CAPITAL_SIGMA {
            builder.push(lower_sigma(text, index));
        } else {
            map(Mapping::Lower, point, &mut |each| builder.push(each));
        }
    }
}

/// Returns a new string with the case the `mode`, an `int` word, asks for: 0 `upper`, 1
/// `lower`, 2 `casefold`, 3 `swapcase`, 4 `capitalize`, 5 `title`.
///
/// # Safety
///
/// `text` points to a live string, and `mode` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_case(text: *const StrObject, mode: i64) -> *mut StrObject {
    let mode = inline_value(mode);
    // SAFETY: the caller passes a live string; every index stays inside it.
    unsafe {
        let whole = length(text);
        let mut builder = StrBuilder::with_capacity(whole);
        let mut previous_cased = false;
        for index in 0..whole {
            let point = code_point(text, index);
            let mut push = |each: u32| builder.push(each);
            match mode {
                0 => map(Mapping::Upper, point, &mut push),
                2 => map(Mapping::Fold, point, &mut push),
                3 if has(Property::Lower, point) => map(Mapping::Upper, point, &mut push),
                3 if !has(Property::Upper, point) => push(point),
                4 if index == 0 => map(Mapping::Title, point, &mut push),
                1 | 3 | 4 => lower_at(text, index, &mut builder),
                _ => {
                    if previous_cased {
                        lower_at(text, index, &mut builder);
                    } else {
                        map(Mapping::Title, point, &mut push);
                    }
                    previous_cased = has(Property::Cased, point);
                }
            }
        }
        builder.finish()
    }
}

/// Returns the answer to the test `question`, an `int` word, names: 0 `isalpha`, 1 `isalnum`, 2 `isdecimal`,
/// 3 `isdigit`, 4 `isnumeric`, 5 `isspace`, 6 `isupper`, 7 `islower`, 8 `istitle`,
/// 9 `isidentifier`, 10 `isprintable`. An empty string fails each, except `isprintable`.
///
/// # Safety
///
/// `text` points to a live string, and `test` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_test(text: *const StrObject, question: i64) -> bool {
    // SAFETY: the caller passes a live string; every index stays inside it.
    unsafe {
        let whole = length(text);
        let points = (0..whole).map(|index| code_point(text, index));
        let every = |property: Property| {
            whole > 0 && (0..whole).all(|index| has(property, code_point(text, index)))
        };
        match inline_value(question) {
            0 => every(Property::Alpha),
            1 => {
                whole > 0
                    && points.clone().all(|point| {
                        [
                            Property::Alpha,
                            Property::Decimal,
                            Property::Digit,
                            Property::Numeric,
                        ]
                        .iter()
                        .any(|property| has(*property, point))
                    })
            }
            2 => every(Property::Decimal),
            3 => every(Property::Digit),
            4 => every(Property::Numeric),
            5 => every(Property::Space),
            6 => one_case(points, Property::Upper, Property::Lower),
            7 => one_case(points, Property::Lower, Property::Upper),
            8 => is_title(points),
            9 => {
                let mut points = points;
                points
                    .next()
                    .is_some_and(|first| has(Property::IdentifierStart, first))
                    && points.all(|point| has(Property::IdentifierContinue, point))
            }
            _ => points.clone().all(super::unicode::is_printable),
        }
    }
}

/// Returns `isupper`, or `islower` with the properties swapped: at least one code point of the
/// case, and none of the other case or titlecase.
fn one_case(points: impl Iterator<Item = u32>, wanted: Property, other: Property) -> bool {
    let mut found = false;
    for point in points {
        if has(other, point) || has(Property::Title, point) {
            return false;
        }
        found |= has(wanted, point);
    }
    found
}

/// Returns `istitle`: an uppercase or titlecase code point only after one that is not cased,
/// a lowercase one only after a cased one, and at least one cased code point.
fn is_title(points: impl Iterator<Item = u32>) -> bool {
    let (mut found, mut previous_cased) = (false, false);
    for point in points {
        if has(Property::Upper, point) || has(Property::Title, point) {
            if previous_cased {
                return false;
            }
            previous_cased = true;
            found = true;
        } else if has(Property::Lower, point) {
            if !previous_cased {
                return false;
            }
            previous_cased = true;
            found = true;
        } else {
            previous_cased = false;
        }
    }
    found
}

#[cfg(test)]
mod tests;
