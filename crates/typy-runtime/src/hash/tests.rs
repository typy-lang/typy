//! Tests of the hashes, against the values CPython 3.15 gives.

use super::{NONE, float, int, text, tuple};
use crate::int::value::Integer;
use crate::string::from_str;

#[test]
fn ints_hash_as_cpython_hashes_them() {
    let cases: [(i128, i64); 12] = [
        (0, 0),
        (1, 1),
        (-1, -2),
        (-2, -2),
        ((1 << 61) - 2, 2_305_843_009_213_693_950),
        ((1 << 61) - 1, 0),
        (1 << 61, 1),
        (1 << 62, 2),
        (-(1 << 61), -2),
        ((1 << 64) + 5, 13),
        (-(1_i128 << 100) - 7, -549_755_813_895),
        (123_456_789, 123_456_789),
    ];
    for (value, expected) in cases {
        let word = Integer::from_i128(value).into_word();
        // SAFETY: the word is live.
        assert_eq!(unsafe { int(word) }, expected, "{value}");
    }
}

#[test]
fn floats_hash_as_equal_ints_do() {
    let cases: [(f64, i64); 13] = [
        (0.0, 0),
        (-0.0, 0),
        (1.0, 1),
        (-1.0, -2),
        (1.5, 1_152_921_504_606_846_977),
        (0.1, 230_584_300_921_369_408),
        (-2.5e-300, -52_920_977_297_143_526),
        (1e300, 1_224_995_262_755_759_164),
        (5e-324, 16_777_216),
        (f64::INFINITY, 314_159),
        (f64::NEG_INFINITY, -314_159),
        (core::f64::consts::PI, 326_490_430_436_040_707),
        (1e16, 10_000_000_000_000_000),
    ];
    for (value, expected) in cases {
        assert_eq!(float(value), expected, "{value:e}");
    }
    assert_eq!(float(f64::NAN), 0);
}

#[test]
fn tuples_combine_their_items_as_cpython_does() {
    let one = 1;
    let cases: [(&[i64], i64); 6] = [
        (&[], 5_740_354_900_026_072_187),
        (&[one], -6_644_214_454_873_602_895),
        (&[1, 2], -3_550_055_125_485_641_917),
        (&[NONE], 9_181_102_132_670_838_864_u64.cast_signed()),
        (&[1, 0], -5_164_621_852_614_943_976),
        (&[-2], 8_078_679_518_589_016_365),
    ];
    for (items, expected) in cases {
        assert_eq!(
            tuple(items.iter().copied(), items.len()),
            expected,
            "{items:?}"
        );
    }
    let inner = tuple([1, 2].into_iter(), 2);
    assert_eq!(tuple([inner, 3].into_iter(), 2), -333_907_151_259_015_829);
    assert_eq!(
        tuple([float(1.5), 2].into_iter(), 2),
        8_274_728_468_405_361_693
    );
}

#[test]
fn texts_hash_as_cpython_does_with_a_zero_seed() {
    let cases: [(&str, i64); 8] = [
        ("\u{ff}\0", 7_217_110_220_190_950_386),
        ("", 0),
        ("a", 4_644_417_185_603_328_019),
        ("abc", -4_594_863_902_769_663_758),
        ("abcdefgh", 4_574_395_652_268_504_554),
        ("\u{e9}t\u{e9}", -1_033_370_291_238_291_217),
        ("\u{20ac}1", -4_952_286_659_940_152_854),
        ("a\u{1f600}", -6_970_120_874_385_731_343),
    ];
    for (value, expected) in cases {
        // SAFETY: the string is live.
        assert_eq!(unsafe { text(from_str(value)) }, expected, "{value:?}");
    }
}
