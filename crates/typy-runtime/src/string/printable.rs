//! Which code points `repr` shows as they are, rather than escaped, as `str.isprintable` decides.

/// Returns true when `repr` shows the code point unescaped. ASCII follows Python exactly; above ASCII, the C0 and C1 controls, the separators other than the space, and the surrogates are escaped.
pub(crate) fn is_printable(point: u32) -> bool {
    !matches!(
        point,
        0..=0x1f
            | 0x7f..=0xa0
            | 0xad
            | 0x1680
            | 0x2000..=0x200f
            | 0x2028..=0x202f
            | 0x205f..=0x206f
            | 0x3000
            | 0xd800..=0xf8ff
            | 0xfeff
            | 0xf_0000..=0x10_ffff
    )
}

#[cfg(test)]
mod tests {
    use super::is_printable;

    #[test]
    fn controls_separators_and_surrogates_are_not_printable() {
        for point in [
            0x0, 0x1f, 0x7f, 0x85, 0xa0, 0xad, 0x2028, 0xd800, 0x3000, 0xfeff, 0xe000,
        ] {
            assert!(!is_printable(point), "{point:#x}");
        }
        for point in [0x20, 0x41, 0x7e, 0xe9, 0x3b1, 0x4e2d, 0x1f600] {
            assert!(is_printable(point), "{point:#x}");
        }
    }
}
