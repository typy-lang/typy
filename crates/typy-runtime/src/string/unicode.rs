//! Unicode properties the runtime needs: which code points `repr` shows unescaped, and the value
//! of a decimal digit in any script, from CPython 3.15's Unicode data (OI-15).

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

#[cfg(test)]
mod tests {
    use super::{decimal_value, is_printable};

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
}
