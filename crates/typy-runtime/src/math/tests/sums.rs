//! Tests of `gamma`, `lgamma`, `fsum`, `dist`, `prod` and `sumprod`, each case what CPython
//! gives (LIB-12).
#![allow(
    clippy::type_complexity,
    reason = "each table lists the cases of one function"
)]

use super::super::gamma::{typy_rt_math_gamma, typy_rt_math_lgamma};
use super::super::sums::{
    typy_rt_math_dist, typy_rt_math_fsum, typy_rt_math_has_float, typy_rt_math_prod_float,
    typy_rt_math_prod_int, typy_rt_math_prod_mixed, typy_rt_math_sumprod_float,
    typy_rt_math_sumprod_int,
};
use super::{check, check_int, int_word};
use crate::Raising;
use crate::list::{ItemKind, ListObject, new_list, push_retained};

/// Returns a new list of `float`s with these bits.
fn floats(bits: &[u64]) -> *mut ListObject {
    let list = new_list(ItemKind::Float, bits.len());
    for each in bits {
        // SAFETY: the list is live and takes the bits of a float.
        unsafe { push_retained(list, *each) };
    }
    list
}

/// Returns a new list of `int`s with these decimal digits.
fn ints(numbers: &[&str]) -> *mut ListObject {
    let list = new_list(ItemKind::Int, numbers.len());
    for each in numbers {
        let word = int_word(each);
        #[allow(clippy::cast_sign_loss, reason = "an item holds the bits of its word")]
        // SAFETY: the list is live and takes its own reference to the word, which is then
        // released.
        unsafe {
            push_retained(list, word as u64);
            crate::refcount::typy_rt_int_decrement(word);
        }
    }
    list
}

/// Returns a new list of `bool`s.
fn bools(values: &[bool]) -> *mut ListObject {
    let list = new_list(ItemKind::Bool, values.len());
    for each in values {
        // SAFETY: the list is live and takes 0 or 1.
        unsafe { push_retained(list, u64::from(*each)) };
    }
    list
}

fn release(list: *mut ListObject) {
    // SAFETY: the test holds the only reference.
    unsafe { crate::refcount::typy_rt_decrement(list.cast()) };
}

const GAMMA: [(u64, u64); 13] = [
    (0x3fe0_0000_0000_0000, 0x3ffc_5bf8_91b4_ef6a),
    (0x4014_0000_0000_0000, 0x4038_0000_0000_0000),
    (0x4037_0000_0000_0000, 0x444e_7752_6159_f06c),
    (0x4038_0000_0000_0000, 0x4495_e5c3_35f8_a4cc),
    (0xbff8_0000_0000_0000, 0x4002_e7fb_0bcd_f4f1),
    (0x4065_7000_0000_0000, 0x7fe0_e186_3dca_d78a),
    (0x01a5_6e1f_c2f8_f359, 0x7e37_e43c_8800_759b),
    (0xc06f_5000_0000_0000, 0x8000_0000_0000_0000),
    (0x4062_d000_0000_0000, 0x7642_f26c_34c6_c255),
    (0x400a_0000_0000_0000, 0x4004_64e0_d629_3ffb),
    (0xbfe0_0000_0000_0000, 0xc00c_5bf8_91b4_ef6b),
    (0x401e_0000_0000_0000, 0x409d_3d04_68bd_32bc),
    (0xc062_d000_0000_0000, 0x8942_0cf9_b3f0_8b2f),
];

const LGAMMA: [(u64, u64); 8] = [
    (0x3fe0_0000_0000_0000, 0x3fe2_50d0_48e7_a1c0),
    (0x4059_0000_0000_0000, 0x4076_7225_b487_9462),
    (0xc004_0000_0000_0000, 0xbfac_cbf9_f5ed_0f60),
    (0x39b4_484b_feeb_c2a0, 0x4051_44f6_9ff9_ffc4),
    (0x4008_0000_0000_0000, 0x3fe6_2e42_fefa_39ec),
    (0x7e37_e43c_8800_759c, 0x7ed0_17f3_8e7a_1ab5),
    (0xbfe8_0000_0000_0000, 0x3ff9_3616_060e_a5e2),
    (0x4012_0000_0000_0000, 0x4003_a140_a3a6_23cc),
];

#[test]
#[cfg_attr(miri, ignore = "Miri cannot call the C maths library")]
fn gamma_and_lgamma_give_cpythons_results() {
    for (x, expected) in GAMMA {
        let x = f64::from_bits(x);
        check(typy_rt_math_gamma(x), &Ok(expected), &format!("gamma({x})"));
    }
    for (x, expected) in LGAMMA {
        let x = f64::from_bits(x);
        check(
            typy_rt_math_lgamma(x),
            &Ok(expected),
            &format!("lgamma({x})"),
        );
    }
    for (x, expected) in [(f64::INFINITY, f64::INFINITY), (f64::NAN, f64::NAN)] {
        check(typy_rt_math_gamma(x), &Ok(expected.to_bits()), "gamma");
        check(typy_rt_math_lgamma(x), &Ok(expected.to_bits()), "lgamma");
    }
    check(
        typy_rt_math_lgamma(f64::NEG_INFINITY),
        &Ok(f64::INFINITY.to_bits()),
        "lgamma",
    );
    check(
        typy_rt_math_lgamma(1.0),
        &Ok(0.0_f64.to_bits()),
        "lgamma(1)",
    );
}

#[test]
#[cfg_attr(miri, ignore = "Miri cannot call the C maths library")]
fn gamma_and_lgamma_refuse_poles_and_overflow() {
    for (x, message) in [
        (0.0, "expected a noninteger or positive integer, got 0.0"),
        (-0.0, "expected a noninteger or positive integer, got -0.0"),
        (-2.0, "expected a noninteger or positive integer, got -2.0"),
        (
            f64::NEG_INFINITY,
            "expected a noninteger or positive integer, got -inf",
        ),
    ] {
        check(
            typy_rt_math_gamma(x),
            &Err(("ValueError", message)),
            &format!("gamma({x})"),
        );
    }
    for (x, message) in [
        (0.0, "expected a noninteger or positive integer, got 0.0"),
        (-3.0, "expected a noninteger or positive integer, got -3.0"),
    ] {
        check(
            typy_rt_math_lgamma(x),
            &Err(("ValueError", message)),
            &format!("lgamma({x})"),
        );
    }
    let overflow = Err(("OverflowError", "math range error"));
    check(typy_rt_math_gamma(200.0), &overflow, "gamma(200)");
    check(typy_rt_math_gamma(171.7), &overflow, "gamma(171.7)");
    check(typy_rt_math_gamma(250.5), &overflow, "gamma(250.5)");
    check(typy_rt_math_lgamma(1e308), &overflow, "lgamma(1e308)");
}

const FSUM: [(&[u64], u64); 10] = [
    (&[0x3fb9_9999_9999_999a; 10], 0x3ff0_0000_0000_0000),
    (
        &[
            0x54b2_49ad_2594_c37d,
            0x3ff0_0000_0000_0000,
            0xd4b2_49ad_2594_c37d,
            0x2b2b_ff2e_e48e_0530,
        ],
        0x3ff0_0000_0000_0000,
    ),
    (
        &[
            0x3c9c_d2b2_97d8_89bc,
            0x3ff0_0000_0000_0000,
            0x4341_c379_37e0_8000,
        ],
        0x4341_c379_37e0_8001,
    ),
    (
        &[
            0x4341_c379_37e0_8000,
            0xbff0_0000_0000_0000,
            0x3c9c_d2b2_97d8_89bc,
        ],
        0x4341_c379_37e0_8000,
    ),
    (
        &[
            0x3ff0_0000_0000_0000,
            0xbc9c_d2b2_97d8_89bc,
            0x4000_0000_0000_0000,
        ],
        0x4008_0000_0000_0000,
    ),
    (
        &[
            0xbff0_0000_0000_0000,
            0x3c9c_d2b2_97d8_89bc,
            0x4000_0000_0000_0000,
        ],
        0x3ff0_0000_0000_0000,
    ),
    (
        &[
            0x3ff0_0000_0000_0000,
            0x3c9c_d2b2_97d8_89bc,
            0x3c9c_d2b2_97d8_89bc,
        ],
        0x3ff0_0000_0000_0001,
    ),
    (&[], 0),
    (&[0x8000_0000_0000_0000], 0),
    (
        &[
            0x4340_0000_0000_0000,
            0x3ff0_0000_0000_0000,
            0x3ff0_0000_0000_0000,
        ],
        0x4340_0000_0000_0001,
    ),
];

#[test]
fn fsum_rounds_the_exact_sum_once() {
    for (values, expected) in FSUM {
        let list = floats(values);
        // SAFETY: the list is live.
        check(
            unsafe { typy_rt_math_fsum(list) },
            &Ok(expected),
            &format!("fsum{values:x?}"),
        );
        release(list);
    }
    let list = ints(&["1", "2", "3"]);
    // SAFETY: the list is live.
    check(
        unsafe { typy_rt_math_fsum(list) },
        &Ok(6.0_f64.to_bits()),
        "fsum of ints",
    );
    release(list);
    let list = bools(&[true, false, true]);
    // SAFETY: the list is live.
    check(
        unsafe { typy_rt_math_fsum(list) },
        &Ok(2.0_f64.to_bits()),
        "fsum of bools",
    );
    release(list);
}

#[test]
fn fsum_reports_overflow_and_infinities_as_cpython_does() {
    let cases: [(&[f64], Result<u64, (&str, &str)>); 5] = [
        (
            &[1e308, 1e308],
            Err(("OverflowError", "intermediate overflow in fsum")),
        ),
        (
            &[f64::INFINITY, f64::NEG_INFINITY],
            Err(("ValueError", "-inf + inf in fsum")),
        ),
        (&[f64::INFINITY, 1.0], Ok(f64::INFINITY.to_bits())),
        (&[f64::NAN, 1.0], Ok(f64::NAN.to_bits())),
        (&[f64::INFINITY, f64::INFINITY], Ok(f64::INFINITY.to_bits())),
    ];
    for (values, expected) in cases {
        let bits: Vec<u64> = values.iter().map(|value| value.to_bits()).collect();
        let list = floats(&bits);
        // SAFETY: the list is live.
        check(
            unsafe { typy_rt_math_fsum(list) },
            &expected,
            &format!("fsum{values:?}"),
        );
        release(list);
    }
    let list = ints(&[&"9".repeat(400)]);
    // SAFETY: the list is live.
    let found = unsafe { typy_rt_math_fsum(list) };
    check(
        found,
        &Err(("OverflowError", "int too large to convert to float")),
        "fsum of a huge int",
    );
    release(list);
}

#[test]
#[cfg_attr(miri, ignore = "Miri cannot call the C maths library")]
fn dist_measures_the_distance_of_points_of_as_many_coordinates() {
    let (p, q) = (
        ints(&["0", "0"]),
        floats(&[3.0_f64.to_bits(), 4.0_f64.to_bits()]),
    );
    // SAFETY: both lists are live.
    check(
        unsafe { typy_rt_math_dist(p, q) },
        &Ok(5.0_f64.to_bits()),
        "dist",
    );
    let short = floats(&[1.0_f64.to_bits()]);
    // SAFETY: both lists are live.
    check(
        unsafe { typy_rt_math_dist(p, short) },
        &Err((
            "ValueError",
            "both points must have the same number of dimensions",
        )),
        "dist of different lengths",
    );
    let huge = ints(&["0", &"9".repeat(400)]);
    // SAFETY: both lists are live.
    check(
        unsafe { typy_rt_math_dist(p, huge) },
        &Err(("OverflowError", "int too large to convert to float")),
        "dist of a huge int",
    );
    for list in [p, q, short, huge] {
        release(list);
    }
}

#[test]
fn prod_multiplies_ints_exactly_and_floats_in_order() {
    let list = ints(&["2"; 70]);
    let start = int_word("3");
    // SAFETY: the list and the word are live.
    let product = unsafe { typy_rt_math_prod_int(list, start) };
    check_int(
        Raising::ok(product),
        &Ok("3541774862152233910272"),
        "prod of ints",
    );
    release(list);
    let list = bools(&[true, true, false]);
    // SAFETY: the list is live, and the word is inline.
    let product = unsafe { typy_rt_math_prod_int(list, start) };
    check_int(Raising::ok(product), &Ok("0"), "prod of bools");
    release(list);
    let list = floats(&[1.5_f64.to_bits(), 2.0_f64.to_bits()]);
    // SAFETY: the list is live.
    check(
        unsafe { typy_rt_math_prod_float(list, 3.0) },
        &Ok(9.0_f64.to_bits()),
        "prod",
    );
    release(list);
    let list = ints(&["2", &"9".repeat(400)]);
    // SAFETY: the list is live.
    check(
        unsafe { typy_rt_math_prod_float(list, 1.0) },
        &Err(("OverflowError", "int too large to convert to float")),
        "prod of a huge int",
    );
    release(list);
}

#[test]
fn sumprod_of_ints_is_exact_and_needs_equal_lengths() {
    let p = ints(&["100000000000000000000", "3"]);
    let q = ints(&["100000000000000000000", "4"]);
    // SAFETY: both lists are live.
    let found = unsafe { typy_rt_math_sumprod_int(p, q) };
    check_int(
        found,
        &Ok("10000000000000000000000000000000000000012"),
        "sumprod",
    );
    let short = bools(&[true]);
    // SAFETY: both lists are live.
    let found = unsafe { typy_rt_math_sumprod_int(p, short) };
    check_int(
        found,
        &Err(("ValueError", "Inputs are not the same length")),
        "sumprod of different lengths",
    );
    for list in [p, q, short] {
        release(list);
    }
}

const SUMPROD: [(&[u64], &[u64], u64); 6] = [
    (
        &[0x3fb9_9999_9999_999a, 0x3fc9_9999_9999_999a],
        &[0x4008_0000_0000_0000, 0x4010_0000_0000_0000],
        0x3ff1_9999_9999_999a,
    ),
    (
        &[
            0x4341_c379_37e0_8000,
            0x3ff0_0000_0000_0000,
            0xc341_c379_37e0_8000,
        ],
        &[0x3ff0_0000_0000_0000; 3],
        0x3ff0_0000_0000_0000,
    ),
    (
        &[0x3fb9_9999_9999_999a; 5],
        &[0x3fd3_3333_3333_3333; 5],
        0x3fc3_3333_3333_3333,
    ),
    (
        &[0x7fe1_ccf3_85eb_c8a0, 0x7fe1_ccf3_85eb_c8a0],
        &[0x4024_0000_0000_0000, 0xc024_0000_0000_0000],
        0xfff8_0000_0000_0000,
    ),
    (&[0x8000_0000_0000_0000], &[0x3ff0_0000_0000_0000], 0),
    (
        &[0x3ff0_0000_0000_0000, 0x7fe1_ccf3_85eb_c8a0],
        &[0x3ff0_0000_0000_0000, 0x4024_0000_0000_0000],
        0x7ff0_0000_0000_0000,
    ),
];

#[test]
#[cfg_attr(miri, ignore = "Miri cannot call the C maths library")]
fn sumprod_of_floats_keeps_the_rounding_errors_of_its_products() {
    for (p, q, expected) in SUMPROD {
        let (left, right) = (floats(p), floats(q));
        // SAFETY: both lists are live.
        check(
            unsafe { typy_rt_math_sumprod_float(left, right) },
            &Ok(expected),
            &format!("sumprod({p:x?}, {q:x?})"),
        );
        release(left);
        release(right);
    }
    let (p, q) = (floats(&[1.0_f64.to_bits()]), ints(&[&"9".repeat(400)]));
    // SAFETY: both lists are live.
    check(
        unsafe { typy_rt_math_sumprod_float(p, q) },
        &Err(("OverflowError", "int too large to convert to float")),
        "sumprod of a huge int",
    );
    // SAFETY: both lists are live.
    check(
        unsafe { typy_rt_math_sumprod_float(q, p) },
        &Err(("OverflowError", "int too large to convert to float")),
        "sumprod of a huge int first",
    );
    let short = floats(&[]);
    // SAFETY: both lists are live.
    check(
        unsafe { typy_rt_math_sumprod_float(p, short) },
        &Err(("ValueError", "Inputs are not the same length")),
        "sumprod of different lengths",
    );
    for list in [p, q, short] {
        release(list);
    }
}

#[test]
fn has_float_finds_a_float_item_only_in_a_list_that_holds_one() {
    let (none, some, empty) = (ints(&["1", "2"]), floats(&[1.5_f64.to_bits()]), ints(&[]));
    // SAFETY: the lists are live.
    unsafe {
        assert!(!typy_rt_math_has_float(none));
        assert!(typy_rt_math_has_float(some));
        assert!(!typy_rt_math_has_float(empty));
    }
    for list in [none, some, empty] {
        release(list);
    }
}

#[test]
fn prod_of_an_int_start_and_floats_converts_the_exact_product_once() {
    let list = floats(&[1.0_f64.to_bits(), 0.5_f64.to_bits()]);
    let start = int_word(&(3_i128 * ((1 << 60) + 1)).to_string());
    // SAFETY: the list is live and the start is an `int` word.
    check(
        unsafe { typy_rt_math_prod_mixed(list, start) },
        &Ok(1_729_382_256_910_270_465.5_f64.to_bits()),
        "prod of a large start and floats",
    );
    let huge = int_word(&"9".repeat(400));
    // SAFETY: as above.
    check(
        unsafe { typy_rt_math_prod_mixed(list, huge) },
        &Err(("OverflowError", "int too large to convert to float")),
        "prod of a huge start and a float",
    );
    // SAFETY: the test owns both words.
    unsafe {
        crate::refcount::typy_rt_int_decrement(start);
        crate::refcount::typy_rt_int_decrement(huge);
    }
    release(list);
}

#[test]
fn sumprod_adds_exact_int_pairs_before_a_float_joins_the_total() {
    // `sumprod([2**62, 2**62], [4, 4])` overflows a machine integer, so its exact sum is an
    // `int`, which the `float` of the last pair then joins.
    let (p, q) = (
        ints(&["4611686018427387904", "4611686018427387904"]),
        ints(&["4", "4"]),
    );
    let (x, y) = (floats(&[1.5_f64.to_bits()]), bools(&[true]));
    // SAFETY: the lists are live.
    unsafe {
        let whole = typy_rt_math_sumprod_float(p, q);
        assert!(whole.exception.is_null());
        assert_eq!(whole.value, 36_893_488_147_419_103_232.0);
        let mixed = typy_rt_math_sumprod_float(x, y);
        assert!(mixed.exception.is_null());
        assert_eq!(mixed.value, 1.5);
    }
    for list in [p, q, x, y] {
        release(list);
    }
}
