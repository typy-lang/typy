//! Tests of the Unicode properties the runtime needs.

use super::{
    Mapping, Property, ascii_has, ascii_mapped, decimal_value, has, is_printable, map, table_has,
    table_map,
};

#[test]
fn ascii_answers_match_the_tables() {
    let properties = [
        Property::Alpha,
        Property::Decimal,
        Property::Digit,
        Property::Numeric,
        Property::Space,
        Property::Upper,
        Property::Lower,
        Property::Title,
        Property::IdentifierStart,
        Property::IdentifierContinue,
        Property::Cased,
        Property::CaseIgnorable,
    ];
    for byte in 0..0x80_u8 {
        for property in properties {
            assert_eq!(
                ascii_has(property, byte),
                table_has(property, u32::from(byte)),
                "{property:?} {byte:#x}"
            );
        }
        for mapping in [
            Mapping::Upper,
            Mapping::Lower,
            Mapping::Title,
            Mapping::Fold,
        ] {
            let mut out = Vec::new();
            table_map(mapping, u32::from(byte), &mut |each| out.push(each));
            assert_eq!(
                out,
                [u32::from(ascii_mapped(mapping, byte))],
                "{mapping:?} {byte:#x}"
            );
        }
    }
}

#[test]
fn printability_matches_cpython() {
    let not_printable = [
        0x0, 0x1f, 0x7f, 0x85, 0xa0, 0xad, 0x378, 0x61c, 0x200b, 0x2028, 0x3000, 0xd800, 0xe000,
        0xfeff, 0xffff, 0xe0001, 0xf0000, 0x10_ffff,
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
