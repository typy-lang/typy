//! Tests of `math`: each case gives what CPython 3.15 gives for it.
#![allow(
    clippy::type_complexity,
    reason = "each table lists the cases of one function"
)]

use super::integer::{
    typy_rt_math_comb, typy_rt_math_factorial, typy_rt_math_gcd, typy_rt_math_isqrt,
    typy_rt_math_lcm, typy_rt_math_perm,
};
use super::log::{
    typy_rt_math_log_divide, typy_rt_math_log_float, typy_rt_math_log_int, typy_rt_math_log2_float,
    typy_rt_math_log2_int, typy_rt_math_log10_float, typy_rt_math_log10_int,
};
use super::real::*;
use super::special::*;
use crate::Raising;

type Outcome = Result<u64, (&'static str, &'static str)>;

/// `(function, x, outcome)` for each function of one `float`.
const ONES: &[(&str, u64, Outcome)] = &[
    ("acos", 0x0000_0000_0000_0000, Ok(0x3ff9_21fb_5444_2d18)),
    ("acos", 0x8000_0000_0000_0000, Ok(0x3ff9_21fb_5444_2d18)),
    ("acos", 0x3fe0_0000_0000_0000, Ok(0x3ff0_c152_382d_7366)),
    ("acos", 0xbfe0_0000_0000_0000, Ok(0x4000_c152_382d_7366)),
    ("acos", 0x3ff0_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    ("acos", 0xbff0_0000_0000_0000, Ok(0x4009_21fb_5444_2d18)),
    (
        "acos",
        0x4000_0000_0000_0000,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got 2.0",
        )),
    ),
    (
        "acos",
        0xc000_0000_0000_0000,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got -2.0",
        )),
    ),
    ("acos", 0x01a5_6e1f_c2f8_f359, Ok(0x3ff9_21fb_5444_2d18)),
    ("acos", 0x0000_0000_0000_0001, Ok(0x3ff9_21fb_5444_2d18)),
    (
        "acos",
        0x7e37_e43c_8800_759c,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got 1e+300",
        )),
    ),
    (
        "acos",
        0xfe37_e43c_8800_759c,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got -1e+300",
        )),
    ),
    (
        "acos",
        0x4086_3000_0000_0000,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got 710.0",
        )),
    ),
    (
        "acos",
        0xc087_4c00_0000_0000,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got -745.5",
        )),
    ),
    (
        "acos",
        0x4009_21fb_5444_2d18,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got 3.141592653589793",
        )),
    ),
    (
        "acos",
        0x7ff0_0000_0000_0000,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got inf",
        )),
    ),
    (
        "acos",
        0xfff0_0000_0000_0000,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got -inf",
        )),
    ),
    ("acos", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("acos", 0x3fb9_9999_9999_999a, Ok(0x3ff7_87b2_2ce3_f590)),
    (
        "acos",
        0x4059_0000_0000_0000,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got 100.0",
        )),
    ),
    (
        "acos",
        0x3ff8_0000_0000_0000,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got 1.5",
        )),
    ),
    (
        "acosh",
        0x0000_0000_0000_0000,
        Err((
            "ValueError",
            "expected argument value not less than 1, got 0.0",
        )),
    ),
    (
        "acosh",
        0x8000_0000_0000_0000,
        Err((
            "ValueError",
            "expected argument value not less than 1, got -0.0",
        )),
    ),
    (
        "acosh",
        0x3fe0_0000_0000_0000,
        Err((
            "ValueError",
            "expected argument value not less than 1, got 0.5",
        )),
    ),
    (
        "acosh",
        0xbfe0_0000_0000_0000,
        Err((
            "ValueError",
            "expected argument value not less than 1, got -0.5",
        )),
    ),
    ("acosh", 0x3ff0_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    (
        "acosh",
        0xbff0_0000_0000_0000,
        Err((
            "ValueError",
            "expected argument value not less than 1, got -1.0",
        )),
    ),
    ("acosh", 0x4000_0000_0000_0000, Ok(0x3ff5_1242_7198_0434)),
    (
        "acosh",
        0xc000_0000_0000_0000,
        Err((
            "ValueError",
            "expected argument value not less than 1, got -2.0",
        )),
    ),
    (
        "acosh",
        0x01a5_6e1f_c2f8_f359,
        Err((
            "ValueError",
            "expected argument value not less than 1, got 1e-300",
        )),
    ),
    (
        "acosh",
        0x0000_0000_0000_0001,
        Err((
            "ValueError",
            "expected argument value not less than 1, got 5e-324",
        )),
    ),
    ("acosh", 0x7e37_e43c_8800_759c, Ok(0x4085_9bbf_d8b8_3e43)),
    (
        "acosh",
        0xfe37_e43c_8800_759c,
        Err((
            "ValueError",
            "expected argument value not less than 1, got -1e+300",
        )),
    ),
    ("acosh", 0x4086_3000_0000_0000, Ok(0x401d_089d_1096_b506)),
    (
        "acosh",
        0xc087_4c00_0000_0000,
        Err((
            "ValueError",
            "expected argument value not less than 1, got -745.5",
        )),
    ),
    ("acosh", 0x4009_21fb_5444_2d18, Ok(0x3ffc_fc02_f901_06c1)),
    ("acosh", 0x7ff0_0000_0000_0000, Ok(0x7ff0_0000_0000_0000)),
    (
        "acosh",
        0xfff0_0000_0000_0000,
        Err((
            "ValueError",
            "expected argument value not less than 1, got -inf",
        )),
    ),
    ("acosh", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    (
        "acosh",
        0x3fb9_9999_9999_999a,
        Err((
            "ValueError",
            "expected argument value not less than 1, got 0.1",
        )),
    ),
    ("acosh", 0x4059_0000_0000_0000, Ok(0x4015_3173_8dcb_c626)),
    ("acosh", 0x3ff8_0000_0000_0000, Ok(0x3fee_cc2c_aec5_160a)),
    ("asin", 0x0000_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    ("asin", 0x8000_0000_0000_0000, Ok(0x8000_0000_0000_0000)),
    ("asin", 0x3fe0_0000_0000_0000, Ok(0x3fe0_c152_382d_7366)),
    ("asin", 0xbfe0_0000_0000_0000, Ok(0xbfe0_c152_382d_7366)),
    ("asin", 0x3ff0_0000_0000_0000, Ok(0x3ff9_21fb_5444_2d18)),
    ("asin", 0xbff0_0000_0000_0000, Ok(0xbff9_21fb_5444_2d18)),
    (
        "asin",
        0x4000_0000_0000_0000,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got 2.0",
        )),
    ),
    (
        "asin",
        0xc000_0000_0000_0000,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got -2.0",
        )),
    ),
    ("asin", 0x01a5_6e1f_c2f8_f359, Ok(0x01a5_6e1f_c2f8_f359)),
    ("asin", 0x0000_0000_0000_0001, Ok(0x0000_0000_0000_0001)),
    (
        "asin",
        0x7e37_e43c_8800_759c,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got 1e+300",
        )),
    ),
    (
        "asin",
        0xfe37_e43c_8800_759c,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got -1e+300",
        )),
    ),
    (
        "asin",
        0x4086_3000_0000_0000,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got 710.0",
        )),
    ),
    (
        "asin",
        0xc087_4c00_0000_0000,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got -745.5",
        )),
    ),
    (
        "asin",
        0x4009_21fb_5444_2d18,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got 3.141592653589793",
        )),
    ),
    (
        "asin",
        0x7ff0_0000_0000_0000,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got inf",
        )),
    ),
    (
        "asin",
        0xfff0_0000_0000_0000,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got -inf",
        )),
    ),
    ("asin", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("asin", 0x3fb9_9999_9999_999a, Ok(0x3fb9_a492_7603_7884)),
    (
        "asin",
        0x4059_0000_0000_0000,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got 100.0",
        )),
    ),
    (
        "asin",
        0x3ff8_0000_0000_0000,
        Err((
            "ValueError",
            "expected a number in range from -1 up to 1, got 1.5",
        )),
    ),
    ("asinh", 0x0000_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    ("asinh", 0x8000_0000_0000_0000, Ok(0x8000_0000_0000_0000)),
    ("asinh", 0x3fe0_0000_0000_0000, Ok(0x3fde_cc2c_aec5_160a)),
    ("asinh", 0xbfe0_0000_0000_0000, Ok(0xbfde_cc2c_aec5_160a)),
    ("asinh", 0x3ff0_0000_0000_0000, Ok(0x3fec_3436_6179_d427)),
    ("asinh", 0xbff0_0000_0000_0000, Ok(0xbfec_3436_6179_d427)),
    ("asinh", 0x4000_0000_0000_0000, Ok(0x3ff7_1921_8313_d087)),
    ("asinh", 0xc000_0000_0000_0000, Ok(0xbff7_1921_8313_d087)),
    ("asinh", 0x01a5_6e1f_c2f8_f359, Ok(0x01a5_6e1f_c2f8_f359)),
    ("asinh", 0x0000_0000_0000_0001, Ok(0x0000_0000_0000_0001)),
    ("asinh", 0x7e37_e43c_8800_759c, Ok(0x4085_9bbf_d8b8_3e43)),
    ("asinh", 0xfe37_e43c_8800_759c, Ok(0xc085_9bbf_d8b8_3e43)),
    ("asinh", 0x4086_3000_0000_0000, Ok(0x401d_089d_5326_d8df)),
    ("asinh", 0xc087_4c00_0000_0000, Ok(0xc01d_3a93_5c8c_a131)),
    ("asinh", 0x4009_21fb_5444_2d18, Ok(0x3ffd_cbf6_9f10_006d)),
    ("asinh", 0x7ff0_0000_0000_0000, Ok(0x7ff0_0000_0000_0000)),
    ("asinh", 0xfff0_0000_0000_0000, Ok(0xfff0_0000_0000_0000)),
    ("asinh", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("asinh", 0x3fb9_9999_9999_999a, Ok(0x3fb9_8eb9_e7e5_fc3f)),
    ("asinh", 0x4059_0000_0000_0000, Ok(0x4015_3180_a93d_3c2a)),
    ("asinh", 0x3ff8_0000_0000_0000, Ok(0x3ff3_1dc0_090b_63d9)),
    ("atan", 0x0000_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    ("atan", 0x8000_0000_0000_0000, Ok(0x8000_0000_0000_0000)),
    ("atan", 0x3fe0_0000_0000_0000, Ok(0x3fdd_ac67_0561_bb4f)),
    ("atan", 0xbfe0_0000_0000_0000, Ok(0xbfdd_ac67_0561_bb4f)),
    ("atan", 0x3ff0_0000_0000_0000, Ok(0x3fe9_21fb_5444_2d18)),
    ("atan", 0xbff0_0000_0000_0000, Ok(0xbfe9_21fb_5444_2d18)),
    ("atan", 0x4000_0000_0000_0000, Ok(0x3ff1_b6e1_92eb_be44)),
    ("atan", 0xc000_0000_0000_0000, Ok(0xbff1_b6e1_92eb_be44)),
    ("atan", 0x01a5_6e1f_c2f8_f359, Ok(0x01a5_6e1f_c2f8_f359)),
    ("atan", 0x0000_0000_0000_0001, Ok(0x0000_0000_0000_0001)),
    ("atan", 0x7e37_e43c_8800_759c, Ok(0x3ff9_21fb_5444_2d18)),
    ("atan", 0xfe37_e43c_8800_759c, Ok(0xbff9_21fb_5444_2d18)),
    ("atan", 0x4086_3000_0000_0000, Ok(0x3ff9_1c36_7668_c617)),
    ("atan", 0xc087_4c00_0000_0000, Ok(0xbff9_1c7c_ca18_38ac)),
    ("atan", 0x4009_21fb_5444_2d18, Ok(0x3ff4_33b8_a322_ddd3)),
    ("atan", 0x7ff0_0000_0000_0000, Ok(0x3ff9_21fb_5444_2d18)),
    ("atan", 0xfff0_0000_0000_0000, Ok(0xbff9_21fb_5444_2d18)),
    ("atan", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("atan", 0x3fb9_9999_9999_999a, Ok(0x3fb9_83e2_82e2_cc4d)),
    ("atan", 0x4059_0000_0000_0000, Ok(0x3ff8_f905_eb2d_ef22)),
    ("atan", 0x3ff8_0000_0000_0000, Ok(0x3fef_730b_d281_f69b)),
    ("atanh", 0x0000_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    ("atanh", 0x8000_0000_0000_0000, Ok(0x8000_0000_0000_0000)),
    ("atanh", 0x3fe0_0000_0000_0000, Ok(0x3fe1_93ea_7aad_030a)),
    ("atanh", 0xbfe0_0000_0000_0000, Ok(0xbfe1_93ea_7aad_030a)),
    (
        "atanh",
        0x3ff0_0000_0000_0000,
        Err(("ValueError", "expected a number between -1 and 1, got 1.0")),
    ),
    (
        "atanh",
        0xbff0_0000_0000_0000,
        Err(("ValueError", "expected a number between -1 and 1, got -1.0")),
    ),
    (
        "atanh",
        0x4000_0000_0000_0000,
        Err(("ValueError", "expected a number between -1 and 1, got 2.0")),
    ),
    (
        "atanh",
        0xc000_0000_0000_0000,
        Err(("ValueError", "expected a number between -1 and 1, got -2.0")),
    ),
    ("atanh", 0x01a5_6e1f_c2f8_f359, Ok(0x01a5_6e1f_c2f8_f359)),
    ("atanh", 0x0000_0000_0000_0001, Ok(0x0000_0000_0000_0001)),
    (
        "atanh",
        0x7e37_e43c_8800_759c,
        Err((
            "ValueError",
            "expected a number between -1 and 1, got 1e+300",
        )),
    ),
    (
        "atanh",
        0xfe37_e43c_8800_759c,
        Err((
            "ValueError",
            "expected a number between -1 and 1, got -1e+300",
        )),
    ),
    (
        "atanh",
        0x4086_3000_0000_0000,
        Err((
            "ValueError",
            "expected a number between -1 and 1, got 710.0",
        )),
    ),
    (
        "atanh",
        0xc087_4c00_0000_0000,
        Err((
            "ValueError",
            "expected a number between -1 and 1, got -745.5",
        )),
    ),
    (
        "atanh",
        0x4009_21fb_5444_2d18,
        Err((
            "ValueError",
            "expected a number between -1 and 1, got 3.141592653589793",
        )),
    ),
    (
        "atanh",
        0x7ff0_0000_0000_0000,
        Err(("ValueError", "expected a number between -1 and 1, got inf")),
    ),
    (
        "atanh",
        0xfff0_0000_0000_0000,
        Err(("ValueError", "expected a number between -1 and 1, got -inf")),
    ),
    ("atanh", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("atanh", 0x3fb9_9999_9999_999a, Ok(0x3fb9_af93_cd23_4412)),
    (
        "atanh",
        0x4059_0000_0000_0000,
        Err((
            "ValueError",
            "expected a number between -1 and 1, got 100.0",
        )),
    ),
    (
        "atanh",
        0x3ff8_0000_0000_0000,
        Err(("ValueError", "expected a number between -1 and 1, got 1.5")),
    ),
    ("cbrt", 0x0000_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    ("cbrt", 0x8000_0000_0000_0000, Ok(0x8000_0000_0000_0000)),
    ("cbrt", 0x3fe0_0000_0000_0000, Ok(0x3fe9_65fe_a53d_6e3d)),
    ("cbrt", 0xbfe0_0000_0000_0000, Ok(0xbfe9_65fe_a53d_6e3d)),
    ("cbrt", 0x3ff0_0000_0000_0000, Ok(0x3ff0_0000_0000_0000)),
    ("cbrt", 0xbff0_0000_0000_0000, Ok(0xbff0_0000_0000_0000)),
    ("cbrt", 0x4000_0000_0000_0000, Ok(0x3ff4_28a2_f98d_728c)),
    ("cbrt", 0xc000_0000_0000_0000, Ok(0xbff4_28a2_f98d_728c)),
    ("cbrt", 0x01a5_6e1f_c2f8_f359, Ok(0x2b2b_ff2e_e48e_0530)),
    ("cbrt", 0x0000_0000_0000_0001, Ok(0x298f_ffff_ffff_ffff)),
    ("cbrt", 0x7e37_e43c_8800_759c, Ok(0x54b2_49ad_2594_c37d)),
    ("cbrt", 0xfe37_e43c_8800_759c, Ok(0xd4b2_49ad_2594_c37d)),
    ("cbrt", 0x4086_3000_0000_0000, Ok(0x4021_d79d_3987_923a)),
    ("cbrt", 0xc087_4c00_0000_0000, Ok(0xc022_2281_a2d0_ebf3)),
    ("cbrt", 0x4009_21fb_5444_2d18, Ok(0x3ff7_6ef7_e731_04b7)),
    ("cbrt", 0x7ff0_0000_0000_0000, Ok(0x7ff0_0000_0000_0000)),
    ("cbrt", 0xfff0_0000_0000_0000, Ok(0xfff0_0000_0000_0000)),
    ("cbrt", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("cbrt", 0x3fb9_9999_9999_999a, Ok(0x3fdd_b4c7_760b_cff2)),
    ("cbrt", 0x4059_0000_0000_0000, Ok(0x4012_90fc_a9c7_61f7)),
    ("cbrt", 0x3ff8_0000_0000_0000, Ok(0x3ff2_50bf_e1b0_82f4)),
    ("cos", 0x0000_0000_0000_0000, Ok(0x3ff0_0000_0000_0000)),
    ("cos", 0x8000_0000_0000_0000, Ok(0x3ff0_0000_0000_0000)),
    ("cos", 0x3fe0_0000_0000_0000, Ok(0x3fec_1528_065b_7d50)),
    ("cos", 0xbfe0_0000_0000_0000, Ok(0x3fec_1528_065b_7d50)),
    ("cos", 0x3ff0_0000_0000_0000, Ok(0x3fe1_4a28_0fb5_068c)),
    ("cos", 0xbff0_0000_0000_0000, Ok(0x3fe1_4a28_0fb5_068c)),
    ("cos", 0x4000_0000_0000_0000, Ok(0xbfda_a226_5753_7205)),
    ("cos", 0xc000_0000_0000_0000, Ok(0xbfda_a226_5753_7205)),
    ("cos", 0x01a5_6e1f_c2f8_f359, Ok(0x3ff0_0000_0000_0000)),
    ("cos", 0x0000_0000_0000_0001, Ok(0x3ff0_0000_0000_0000)),
    ("cos", 0x7e37_e43c_8800_759c, Ok(0xbfe2_6990_22ad_c4c1)),
    ("cos", 0xfe37_e43c_8800_759c, Ok(0xbfe2_6990_22ad_c4c1)),
    ("cos", 0x4086_3000_0000_0000, Ok(0x3fef_ffff_ff06_3930)),
    ("cos", 0xc087_4c00_0000_0000, Ok(0xbfe2_ceb7_9ce2_b3ac)),
    ("cos", 0x4009_21fb_5444_2d18, Ok(0xbff0_0000_0000_0000)),
    (
        "cos",
        0x7ff0_0000_0000_0000,
        Err(("ValueError", "expected a finite input, got inf")),
    ),
    (
        "cos",
        0xfff0_0000_0000_0000,
        Err(("ValueError", "expected a finite input, got -inf")),
    ),
    ("cos", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("cos", 0x3fb9_9999_9999_999a, Ok(0x3fef_d712_f9a8_17c1)),
    ("cos", 0x4059_0000_0000_0000, Ok(0x3feb_981d_bf66_5fdf)),
    ("cos", 0x3ff8_0000_0000_0000, Ok(0x3fb2_1bd5_4fc5_f9a7)),
    ("cosh", 0x0000_0000_0000_0000, Ok(0x3ff0_0000_0000_0000)),
    ("cosh", 0x8000_0000_0000_0000, Ok(0x3ff0_0000_0000_0000)),
    ("cosh", 0x3fe0_0000_0000_0000, Ok(0x3ff2_0ac1_862a_e8d0)),
    ("cosh", 0xbfe0_0000_0000_0000, Ok(0x3ff2_0ac1_862a_e8d0)),
    ("cosh", 0x3ff0_0000_0000_0000, Ok(0x3ff8_b075_51d9_f550)),
    ("cosh", 0xbff0_0000_0000_0000, Ok(0x3ff8_b075_51d9_f550)),
    ("cosh", 0x4000_0000_0000_0000, Ok(0x400e_18fa_0df2_d9bc)),
    ("cosh", 0xc000_0000_0000_0000, Ok(0x400e_18fa_0df2_d9bc)),
    ("cosh", 0x01a5_6e1f_c2f8_f359, Ok(0x3ff0_0000_0000_0000)),
    ("cosh", 0x0000_0000_0000_0001, Ok(0x3ff0_0000_0000_0000)),
    (
        "cosh",
        0x7e37_e43c_8800_759c,
        Err(("OverflowError", "math range error")),
    ),
    (
        "cosh",
        0xfe37_e43c_8800_759c,
        Err(("OverflowError", "math range error")),
    ),
    ("cosh", 0x4086_3000_0000_0000, Ok(0x7fe3_e21a_4645_07fa)),
    (
        "cosh",
        0xc087_4c00_0000_0000,
        Err(("OverflowError", "math range error")),
    ),
    ("cosh", 0x4009_21fb_5444_2d18, Ok(0x4027_2f14_7fee_4000)),
    ("cosh", 0x7ff0_0000_0000_0000, Ok(0x7ff0_0000_0000_0000)),
    ("cosh", 0xfff0_0000_0000_0000, Ok(0x7ff0_0000_0000_0000)),
    ("cosh", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("cosh", 0x3fb9_9999_9999_999a, Ok(0x3ff0_147f_4022_4b38)),
    ("cosh", 0x4059_0000_0000_0000, Ok(0x48e3_494a_9b17_1bf5)),
    ("cosh", 0x3ff8_0000_0000_0000, Ok(0x4002_d1bc_21e2_2022)),
    ("erf", 0x0000_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    ("erf", 0x8000_0000_0000_0000, Ok(0x8000_0000_0000_0000)),
    ("erf", 0x3fe0_0000_0000_0000, Ok(0x3fe0_a7ef_5c18_edd2)),
    ("erf", 0xbfe0_0000_0000_0000, Ok(0xbfe0_a7ef_5c18_edd2)),
    ("erf", 0x3ff0_0000_0000_0000, Ok(0x3fea_f767_a741_088b)),
    ("erf", 0xbff0_0000_0000_0000, Ok(0xbfea_f767_a741_088b)),
    ("erf", 0x4000_0000_0000_0000, Ok(0x3fef_d9ae_1427_95e3)),
    ("erf", 0xc000_0000_0000_0000, Ok(0xbfef_d9ae_1427_95e3)),
    ("erf", 0x01a5_6e1f_c2f8_f359, Ok(0x01a8_2e6d_9871_1d3a)),
    ("erf", 0x0000_0000_0000_0001, Ok(0x0000_0000_0000_0001)),
    ("erf", 0x7e37_e43c_8800_759c, Ok(0x3ff0_0000_0000_0000)),
    ("erf", 0xfe37_e43c_8800_759c, Ok(0xbff0_0000_0000_0000)),
    ("erf", 0x4086_3000_0000_0000, Ok(0x3ff0_0000_0000_0000)),
    ("erf", 0xc087_4c00_0000_0000, Ok(0xbff0_0000_0000_0000)),
    ("erf", 0x4009_21fb_5444_2d18, Ok(0x3fef_ffed_62a7_bb7c)),
    ("erf", 0x7ff0_0000_0000_0000, Ok(0x3ff0_0000_0000_0000)),
    ("erf", 0xfff0_0000_0000_0000, Ok(0xbff0_0000_0000_0000)),
    ("erf", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("erf", 0x3fb9_9999_9999_999a, Ok(0x3fbc_ca5e_a24f_b334)),
    ("erf", 0x4059_0000_0000_0000, Ok(0x3ff0_0000_0000_0000)),
    ("erf", 0x3ff8_0000_0000_0000, Ok(0x3fee_ea55_5713_7ae0)),
    ("erfc", 0x0000_0000_0000_0000, Ok(0x3ff0_0000_0000_0000)),
    ("erfc", 0x8000_0000_0000_0000, Ok(0x3ff0_0000_0000_0000)),
    ("erfc", 0x3fe0_0000_0000_0000, Ok(0x3fde_b021_47ce_245c)),
    ("erfc", 0xbfe0_0000_0000_0000, Ok(0x3ff8_53f7_ae0c_76e9)),
    ("erfc", 0x3ff0_0000_0000_0000, Ok(0x3fc4_2261_62fb_ddd5)),
    ("erfc", 0xbff0_0000_0000_0000, Ok(0x3ffd_7bb3_d3a0_8446)),
    ("erfc", 0x4000_0000_0000_0000, Ok(0x3f73_28f5_ec35_0e66)),
    ("erfc", 0xc000_0000_0000_0000, Ok(0x3fff_ecd7_0a13_caf2)),
    ("erfc", 0x01a5_6e1f_c2f8_f359, Ok(0x3ff0_0000_0000_0000)),
    ("erfc", 0x0000_0000_0000_0001, Ok(0x3ff0_0000_0000_0000)),
    ("erfc", 0x7e37_e43c_8800_759c, Ok(0x0000_0000_0000_0000)),
    ("erfc", 0xfe37_e43c_8800_759c, Ok(0x4000_0000_0000_0000)),
    ("erfc", 0x4086_3000_0000_0000, Ok(0x0000_0000_0000_0000)),
    ("erfc", 0xc087_4c00_0000_0000, Ok(0x4000_0000_0000_0000)),
    ("erfc", 0x4009_21fb_5444_2d18, Ok(0x3ee2_9d58_4483_97ef)),
    ("erfc", 0x7ff0_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    ("erfc", 0xfff0_0000_0000_0000, Ok(0x4000_0000_0000_0000)),
    ("erfc", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("erfc", 0x3fb9_9999_9999_999a, Ok(0x3fec_66b4_2bb6_099a)),
    ("erfc", 0x4059_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    ("erfc", 0x3ff8_0000_0000_0000, Ok(0x3fa1_5aaa_8ec8_5205)),
    ("exp", 0x0000_0000_0000_0000, Ok(0x3ff0_0000_0000_0000)),
    ("exp", 0x8000_0000_0000_0000, Ok(0x3ff0_0000_0000_0000)),
    ("exp", 0x3fe0_0000_0000_0000, Ok(0x3ffa_6129_8e1e_069c)),
    ("exp", 0xbfe0_0000_0000_0000, Ok(0x3fe3_68b2_fc6f_960a)),
    ("exp", 0x3ff0_0000_0000_0000, Ok(0x4005_bf0a_8b14_5769)),
    ("exp", 0xbff0_0000_0000_0000, Ok(0x3fd7_8b56_362c_ef38)),
    ("exp", 0x4000_0000_0000_0000, Ok(0x401d_8e64_b8d4_ddae)),
    ("exp", 0xc000_0000_0000_0000, Ok(0x3fc1_52aa_a3bf_81cc)),
    ("exp", 0x01a5_6e1f_c2f8_f359, Ok(0x3ff0_0000_0000_0000)),
    ("exp", 0x0000_0000_0000_0001, Ok(0x3ff0_0000_0000_0000)),
    (
        "exp",
        0x7e37_e43c_8800_759c,
        Err(("OverflowError", "math range error")),
    ),
    ("exp", 0xfe37_e43c_8800_759c, Ok(0x0000_0000_0000_0000)),
    (
        "exp",
        0x4086_3000_0000_0000,
        Err(("OverflowError", "math range error")),
    ),
    ("exp", 0xc087_4c00_0000_0000, Ok(0x0000_0000_0000_0000)),
    ("exp", 0x4009_21fb_5444_2d18, Ok(0x4037_2404_6eb0_9339)),
    ("exp", 0x7ff0_0000_0000_0000, Ok(0x7ff0_0000_0000_0000)),
    ("exp", 0xfff0_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    ("exp", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("exp", 0x3fb9_9999_9999_999a, Ok(0x3ff1_aec7_b35a_00d4)),
    ("exp", 0x4059_0000_0000_0000, Ok(0x48f3_494a_9b17_1bf5)),
    ("exp", 0x3ff8_0000_0000_0000, Ok(0x4011_ed3f_e64f_c541)),
    ("exp2", 0x0000_0000_0000_0000, Ok(0x3ff0_0000_0000_0000)),
    ("exp2", 0x8000_0000_0000_0000, Ok(0x3ff0_0000_0000_0000)),
    ("exp2", 0x3fe0_0000_0000_0000, Ok(0x3ff6_a09e_667f_3bcd)),
    ("exp2", 0xbfe0_0000_0000_0000, Ok(0x3fe6_a09e_667f_3bcd)),
    ("exp2", 0x3ff0_0000_0000_0000, Ok(0x4000_0000_0000_0000)),
    ("exp2", 0xbff0_0000_0000_0000, Ok(0x3fe0_0000_0000_0000)),
    ("exp2", 0x4000_0000_0000_0000, Ok(0x4010_0000_0000_0000)),
    ("exp2", 0xc000_0000_0000_0000, Ok(0x3fd0_0000_0000_0000)),
    ("exp2", 0x01a5_6e1f_c2f8_f359, Ok(0x3ff0_0000_0000_0000)),
    ("exp2", 0x0000_0000_0000_0001, Ok(0x3ff0_0000_0000_0000)),
    (
        "exp2",
        0x7e37_e43c_8800_759c,
        Err(("OverflowError", "math range error")),
    ),
    ("exp2", 0xfe37_e43c_8800_759c, Ok(0x0000_0000_0000_0000)),
    ("exp2", 0x4086_3000_0000_0000, Ok(0x6c50_0000_0000_0000)),
    ("exp2", 0xc087_4c00_0000_0000, Ok(0x1156_a09e_667f_3bcd)),
    ("exp2", 0x4009_21fb_5444_2d18, Ok(0x4021_a663_7e66_6f83)),
    ("exp2", 0x7ff0_0000_0000_0000, Ok(0x7ff0_0000_0000_0000)),
    ("exp2", 0xfff0_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    ("exp2", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("exp2", 0x3fb9_9999_9999_999a, Ok(0x3ff1_25fb_ee25_0664)),
    ("exp2", 0x4059_0000_0000_0000, Ok(0x4630_0000_0000_0000)),
    ("exp2", 0x3ff8_0000_0000_0000, Ok(0x4006_a09e_667f_3bcd)),
    ("expm1", 0x0000_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    ("expm1", 0x8000_0000_0000_0000, Ok(0x8000_0000_0000_0000)),
    ("expm1", 0x3fe0_0000_0000_0000, Ok(0x3fe4_c253_1c3c_0d38)),
    ("expm1", 0xbfe0_0000_0000_0000, Ok(0xbfd9_2e9a_0720_d3ec)),
    ("expm1", 0x3ff0_0000_0000_0000, Ok(0x3ffb_7e15_1628_aed2)),
    ("expm1", 0xbff0_0000_0000_0000, Ok(0xbfe4_3a54_e4e9_8864)),
    ("expm1", 0x4000_0000_0000_0000, Ok(0x4019_8e64_b8d4_ddae)),
    ("expm1", 0xc000_0000_0000_0000, Ok(0xbfeb_ab55_5710_1f8d)),
    ("expm1", 0x01a5_6e1f_c2f8_f359, Ok(0x01a5_6e1f_c2f8_f359)),
    ("expm1", 0x0000_0000_0000_0001, Ok(0x0000_0000_0000_0001)),
    (
        "expm1",
        0x7e37_e43c_8800_759c,
        Err(("OverflowError", "math range error")),
    ),
    ("expm1", 0xfe37_e43c_8800_759c, Ok(0xbff0_0000_0000_0000)),
    (
        "expm1",
        0x4086_3000_0000_0000,
        Err(("OverflowError", "math range error")),
    ),
    ("expm1", 0xc087_4c00_0000_0000, Ok(0xbff0_0000_0000_0000)),
    ("expm1", 0x4009_21fb_5444_2d18, Ok(0x4036_2404_6eb0_9339)),
    ("expm1", 0x7ff0_0000_0000_0000, Ok(0x7ff0_0000_0000_0000)),
    ("expm1", 0xfff0_0000_0000_0000, Ok(0xbff0_0000_0000_0000)),
    ("expm1", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("expm1", 0x3fb9_9999_9999_999a, Ok(0x3fba_ec7b_35a0_0d3a)),
    ("expm1", 0x4059_0000_0000_0000, Ok(0x48f3_494a_9b17_1bf5)),
    ("expm1", 0x3ff8_0000_0000_0000, Ok(0x400b_da7f_cc9f_8a83)),
    ("log1p", 0x0000_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    ("log1p", 0x8000_0000_0000_0000, Ok(0x8000_0000_0000_0000)),
    ("log1p", 0x3fe0_0000_0000_0000, Ok(0x3fd9_f323_ecbf_984c)),
    ("log1p", 0xbfe0_0000_0000_0000, Ok(0xbfe6_2e42_fefa_39ef)),
    ("log1p", 0x3ff0_0000_0000_0000, Ok(0x3fe6_2e42_fefa_39ef)),
    (
        "log1p",
        0xbff0_0000_0000_0000,
        Err(("ValueError", "expected argument value > -1, got -1.0")),
    ),
    ("log1p", 0x4000_0000_0000_0000, Ok(0x3ff1_93ea_7aad_030a)),
    (
        "log1p",
        0xc000_0000_0000_0000,
        Err(("ValueError", "expected argument value > -1, got -2.0")),
    ),
    ("log1p", 0x01a5_6e1f_c2f8_f359, Ok(0x01a5_6e1f_c2f8_f359)),
    ("log1p", 0x0000_0000_0000_0001, Ok(0x0000_0000_0000_0001)),
    ("log1p", 0x7e37_e43c_8800_759c, Ok(0x4085_9634_47f8_7fb5)),
    (
        "log1p",
        0xfe37_e43c_8800_759c,
        Err(("ValueError", "expected argument value > -1, got -1e+300")),
    ),
    ("log1p", 0x4086_3000_0000_0000, Ok(0x401a_4445_c706_32f8)),
    (
        "log1p",
        0xc087_4c00_0000_0000,
        Err(("ValueError", "expected argument value > -1, got -745.5")),
    ),
    ("log1p", 0x4009_21fb_5444_2d18, Ok(0x3ff6_bcbe_d09f_00af)),
    ("log1p", 0x7ff0_0000_0000_0000, Ok(0x7ff0_0000_0000_0000)),
    (
        "log1p",
        0xfff0_0000_0000_0000,
        Err(("ValueError", "expected argument value > -1, got -inf")),
    ),
    ("log1p", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("log1p", 0x3fb9_9999_9999_999a, Ok(0x3fb8_663f_793c_46c7)),
    ("log1p", 0x4059_0000_0000_0000, Ok(0x4012_75e2_271b_ba31)),
    ("log1p", 0x3ff8_0000_0000_0000, Ok(0x3fed_5240_f0e0_e078)),
    ("sin", 0x0000_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    ("sin", 0x8000_0000_0000_0000, Ok(0x8000_0000_0000_0000)),
    ("sin", 0x3fe0_0000_0000_0000, Ok(0x3fde_aee8_744b_05f0)),
    ("sin", 0xbfe0_0000_0000_0000, Ok(0xbfde_aee8_744b_05f0)),
    ("sin", 0x3ff0_0000_0000_0000, Ok(0x3fea_ed54_8f09_0cee)),
    ("sin", 0xbff0_0000_0000_0000, Ok(0xbfea_ed54_8f09_0cee)),
    ("sin", 0x4000_0000_0000_0000, Ok(0x3fed_18f6_ead1_b446)),
    ("sin", 0xc000_0000_0000_0000, Ok(0xbfed_18f6_ead1_b446)),
    ("sin", 0x01a5_6e1f_c2f8_f359, Ok(0x01a5_6e1f_c2f8_f359)),
    ("sin", 0x0000_0000_0000_0001, Ok(0x0000_0000_0000_0001)),
    ("sin", 0x7e37_e43c_8800_759c, Ok(0xbfea_2c16_b010_e385)),
    ("sin", 0xfe37_e43c_8800_759c, Ok(0x3fea_2c16_b010_e385)),
    ("sin", 0x4086_3000_0000_0000, Ok(0x3f0f_9bd0_303f_6faf)),
    ("sin", 0xc087_4c00_0000_0000, Ok(0x3fe9_e3c5_a2f7_8a36)),
    ("sin", 0x4009_21fb_5444_2d18, Ok(0x3ca1_a626_3314_5c07)),
    (
        "sin",
        0x7ff0_0000_0000_0000,
        Err(("ValueError", "expected a finite input, got inf")),
    ),
    (
        "sin",
        0xfff0_0000_0000_0000,
        Err(("ValueError", "expected a finite input, got -inf")),
    ),
    ("sin", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("sin", 0x3fb9_9999_9999_999a, Ok(0x3fb9_8eae_cb8b_cb2c)),
    ("sin", 0x4059_0000_0000_0000, Ok(0xbfe0_3425_b78c_4db8)),
    ("sin", 0x3ff8_0000_0000_0000, Ok(0x3fef_eb7a_9b2c_6d8b)),
    ("sinh", 0x0000_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    ("sinh", 0x8000_0000_0000_0000, Ok(0x8000_0000_0000_0000)),
    ("sinh", 0x3fe0_0000_0000_0000, Ok(0x3fe0_acd0_0fe6_3b97)),
    ("sinh", 0xbfe0_0000_0000_0000, Ok(0xbfe0_acd0_0fe6_3b97)),
    ("sinh", 0x3ff0_0000_0000_0000, Ok(0x3ff2_cd9f_c44e_b982)),
    ("sinh", 0xbff0_0000_0000_0000, Ok(0xbff2_cd9f_c44e_b982)),
    ("sinh", 0x4000_0000_0000_0000, Ok(0x400d_03cf_63b6_e1a0)),
    ("sinh", 0xc000_0000_0000_0000, Ok(0xc00d_03cf_63b6_e1a0)),
    ("sinh", 0x01a5_6e1f_c2f8_f359, Ok(0x01a5_6e1f_c2f8_f359)),
    ("sinh", 0x0000_0000_0000_0001, Ok(0x0000_0000_0000_0001)),
    (
        "sinh",
        0x7e37_e43c_8800_759c,
        Err(("OverflowError", "math range error")),
    ),
    (
        "sinh",
        0xfe37_e43c_8800_759c,
        Err(("OverflowError", "math range error")),
    ),
    ("sinh", 0x4086_3000_0000_0000, Ok(0x7fe3_e21a_4645_07fa)),
    (
        "sinh",
        0xc087_4c00_0000_0000,
        Err(("OverflowError", "math range error")),
    ),
    ("sinh", 0x4009_21fb_5444_2d18, Ok(0x4027_18f4_5d72_e672)),
    ("sinh", 0x7ff0_0000_0000_0000, Ok(0x7ff0_0000_0000_0000)),
    ("sinh", 0xfff0_0000_0000_0000, Ok(0xfff0_0000_0000_0000)),
    ("sinh", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("sinh", 0x3fb9_9999_9999_999a, Ok(0x3fb9_a487_337b_59b3)),
    ("sinh", 0x4059_0000_0000_0000, Ok(0x48e3_494a_9b17_1bf5)),
    ("sinh", 0x3ff8_0000_0000_0000, Ok(0x4001_08c3_aabd_6a60)),
    ("sqrt", 0x0000_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    ("sqrt", 0x8000_0000_0000_0000, Ok(0x8000_0000_0000_0000)),
    ("sqrt", 0x3fe0_0000_0000_0000, Ok(0x3fe6_a09e_667f_3bcd)),
    (
        "sqrt",
        0xbfe0_0000_0000_0000,
        Err(("ValueError", "expected a nonnegative input, got -0.5")),
    ),
    ("sqrt", 0x3ff0_0000_0000_0000, Ok(0x3ff0_0000_0000_0000)),
    (
        "sqrt",
        0xbff0_0000_0000_0000,
        Err(("ValueError", "expected a nonnegative input, got -1.0")),
    ),
    ("sqrt", 0x4000_0000_0000_0000, Ok(0x3ff6_a09e_667f_3bcd)),
    (
        "sqrt",
        0xc000_0000_0000_0000,
        Err(("ValueError", "expected a nonnegative input, got -2.0")),
    ),
    ("sqrt", 0x01a5_6e1f_c2f8_f359, Ok(0x20ca_2fe7_6a3f_9475)),
    ("sqrt", 0x0000_0000_0000_0001, Ok(0x1e60_0000_0000_0000)),
    ("sqrt", 0x7e37_e43c_8800_759c, Ok(0x5f13_8d35_2e50_96af)),
    (
        "sqrt",
        0xfe37_e43c_8800_759c,
        Err(("ValueError", "expected a nonnegative input, got -1e+300")),
    ),
    ("sqrt", 0x4086_3000_0000_0000, Ok(0x403a_a554_ccb1_7776)),
    (
        "sqrt",
        0xc087_4c00_0000_0000,
        Err(("ValueError", "expected a nonnegative input, got -745.5")),
    ),
    ("sqrt", 0x4009_21fb_5444_2d18, Ok(0x3ffc_5bf8_91b4_ef6a)),
    ("sqrt", 0x7ff0_0000_0000_0000, Ok(0x7ff0_0000_0000_0000)),
    (
        "sqrt",
        0xfff0_0000_0000_0000,
        Err(("ValueError", "expected a nonnegative input, got -inf")),
    ),
    ("sqrt", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("sqrt", 0x3fb9_9999_9999_999a, Ok(0x3fd4_3d13_6248_490f)),
    ("sqrt", 0x4059_0000_0000_0000, Ok(0x4024_0000_0000_0000)),
    ("sqrt", 0x3ff8_0000_0000_0000, Ok(0x3ff3_988e_1409_212e)),
    ("tan", 0x0000_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    ("tan", 0x8000_0000_0000_0000, Ok(0x8000_0000_0000_0000)),
    ("tan", 0x3fe0_0000_0000_0000, Ok(0x3fe1_7b4f_5bf3_474a)),
    ("tan", 0xbfe0_0000_0000_0000, Ok(0xbfe1_7b4f_5bf3_474a)),
    ("tan", 0x3ff0_0000_0000_0000, Ok(0x3ff8_eb24_5cbe_e3a6)),
    ("tan", 0xbff0_0000_0000_0000, Ok(0xbff8_eb24_5cbe_e3a6)),
    ("tan", 0x4000_0000_0000_0000, Ok(0xc001_7af6_2e09_50f8)),
    ("tan", 0xc000_0000_0000_0000, Ok(0x4001_7af6_2e09_50f8)),
    ("tan", 0x01a5_6e1f_c2f8_f359, Ok(0x01a5_6e1f_c2f8_f359)),
    ("tan", 0x0000_0000_0000_0001, Ok(0x0000_0000_0000_0001)),
    ("tan", 0x7e37_e43c_8800_759c, Ok(0x3ff6_be41_1f37_ac77)),
    ("tan", 0xfe37_e43c_8800_759c, Ok(0xbff6_be41_1f37_ac77)),
    ("tan", 0x4086_3000_0000_0000, Ok(0x3f0f_9bd0_3136_287d)),
    ("tan", 0xc087_4c00_0000_0000, Ok(0xbff6_0669_312e_c1ee)),
    ("tan", 0x4009_21fb_5444_2d18, Ok(0xbca1_a626_3314_5c07)),
    (
        "tan",
        0x7ff0_0000_0000_0000,
        Err(("ValueError", "expected a finite input, got inf")),
    ),
    (
        "tan",
        0xfff0_0000_0000_0000,
        Err(("ValueError", "expected a finite input, got -inf")),
    ),
    ("tan", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("tan", 0x3fb9_9999_9999_999a, Ok(0x3fb9_af88_7743_0b80)),
    ("tan", 0x4059_0000_0000_0000, Ok(0xbfe2_ca74_d62b_5d38)),
    ("tan", 0x3ff8_0000_0000_0000, Ok(0x402c_33ed_50b8_8777)),
    ("tanh", 0x0000_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    ("tanh", 0x8000_0000_0000_0000, Ok(0x8000_0000_0000_0000)),
    ("tanh", 0x3fe0_0000_0000_0000, Ok(0x3fdd_9353_d756_8af3)),
    ("tanh", 0xbfe0_0000_0000_0000, Ok(0xbfdd_9353_d756_8af3)),
    ("tanh", 0x3ff0_0000_0000_0000, Ok(0x3fe8_5efa_b514_f394)),
    ("tanh", 0xbff0_0000_0000_0000, Ok(0xbfe8_5efa_b514_f394)),
    ("tanh", 0x4000_0000_0000_0000, Ok(0x3fee_d950_5e1b_c3d4)),
    ("tanh", 0xc000_0000_0000_0000, Ok(0xbfee_d950_5e1b_c3d4)),
    ("tanh", 0x01a5_6e1f_c2f8_f359, Ok(0x01a5_6e1f_c2f8_f359)),
    ("tanh", 0x0000_0000_0000_0001, Ok(0x0000_0000_0000_0001)),
    ("tanh", 0x7e37_e43c_8800_759c, Ok(0x3ff0_0000_0000_0000)),
    ("tanh", 0xfe37_e43c_8800_759c, Ok(0xbff0_0000_0000_0000)),
    ("tanh", 0x4086_3000_0000_0000, Ok(0x3ff0_0000_0000_0000)),
    ("tanh", 0xc087_4c00_0000_0000, Ok(0xbff0_0000_0000_0000)),
    ("tanh", 0x4009_21fb_5444_2d18, Ok(0x3fef_e175_fa29_2810)),
    ("tanh", 0x7ff0_0000_0000_0000, Ok(0x3ff0_0000_0000_0000)),
    ("tanh", 0xfff0_0000_0000_0000, Ok(0xbff0_0000_0000_0000)),
    ("tanh", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("tanh", 0x3fb9_9999_9999_999a, Ok(0x3fb9_83d7_795f_413a)),
    ("tanh", 0x4059_0000_0000_0000, Ok(0x3ff0_0000_0000_0000)),
    ("tanh", 0x3ff8_0000_0000_0000, Ok(0x3fec_f6f9_786d_f577)),
];

/// `(function, x, y, outcome)` for each function of two `float`s.
const TWOS: &[(&str, u64, u64, Outcome)] = &[
    (
        "atan2",
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        Ok(0x0000_0000_0000_0000),
    ),
    (
        "atan2",
        0x3ff0_0000_0000_0000,
        0x0000_0000_0000_0000,
        Ok(0x3ff9_21fb_5444_2d18),
    ),
    (
        "atan2",
        0x0000_0000_0000_0000,
        0xbff0_0000_0000_0000,
        Ok(0x4009_21fb_5444_2d18),
    ),
    (
        "atan2",
        0x8000_0000_0000_0000,
        0xc008_0000_0000_0000,
        Ok(0xc009_21fb_5444_2d18),
    ),
    (
        "atan2",
        0x4000_0000_0000_0000,
        0x4024_0000_0000_0000,
        Ok(0x3fc9_4441_f8f7_260b),
    ),
    (
        "atan2",
        0x4000_0000_0000_0000,
        0x4091_3000_0000_0000,
        Ok(0x3f5d_c9ff_b5e8_2f6e),
    ),
    (
        "atan2",
        0xc020_0000_0000_0000,
        0x3fd5_5555_5555_5555,
        Ok(0xbff8_7769_eb8e_956b),
    ),
    (
        "atan2",
        0x4024_0000_0000_0000,
        0x4008_0000_0000_0000,
        Ok(0x3ff4_782c_babc_8157),
    ),
    (
        "atan2",
        0xc024_0000_0000_0000,
        0x4008_0000_0000_0000,
        Ok(0xbff4_782c_babc_8157),
    ),
    (
        "atan2",
        0x4014_0000_0000_0000,
        0xc008_0000_0000_0000,
        Ok(0x4000_e3c5_1fa0_59c4),
    ),
    (
        "atan2",
        0x7ff0_0000_0000_0000,
        0x4000_0000_0000_0000,
        Ok(0x3ff9_21fb_5444_2d18),
    ),
    (
        "atan2",
        0x4000_0000_0000_0000,
        0x7ff0_0000_0000_0000,
        Ok(0x0000_0000_0000_0000),
    ),
    (
        "atan2",
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        Ok(0x4002_d97c_7f33_21d2),
    ),
    (
        "atan2",
        0x7ff8_0000_0000_0000,
        0x0000_0000_0000_0000,
        Ok(0x7ff8_0000_0000_0000),
    ),
    (
        "atan2",
        0x3ff0_0000_0000_0000,
        0x7ff8_0000_0000_0000,
        Ok(0x7ff8_0000_0000_0000),
    ),
    (
        "atan2",
        0x3fe0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        Ok(0x4009_21fb_5444_2d18),
    ),
    (
        "atan2",
        0xfff0_0000_0000_0000,
        0x4008_0000_0000_0000,
        Ok(0xbff9_21fb_5444_2d18),
    ),
    (
        "atan2",
        0xfff0_0000_0000_0000,
        0xc008_0000_0000_0000,
        Ok(0xbff9_21fb_5444_2d18),
    ),
    (
        "atan2",
        0x7fe1_ccf3_85eb_c8a0,
        0x4024_0000_0000_0000,
        Ok(0x3ff9_21fb_5444_2d18),
    ),
    (
        "atan2",
        0x401e_0000_0000_0000,
        0x4004_0000_0000_0000,
        Ok(0x3ff3_fc17_6b7a_8560),
    ),
    (
        "atan2",
        0x4018_0000_0000_0000,
        0x4010_0000_0000_0000,
        Ok(0x3fef_730b_d281_f69b),
    ),
    (
        "atan2",
        0xc000_0000_0000_0000,
        0x3fe0_0000_0000_0000,
        Ok(0xbff5_368c_951e_9cfd),
    ),
    (
        "copysign",
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        Ok(0x0000_0000_0000_0000),
    ),
    (
        "copysign",
        0x3ff0_0000_0000_0000,
        0x0000_0000_0000_0000,
        Ok(0x3ff0_0000_0000_0000),
    ),
    (
        "copysign",
        0x0000_0000_0000_0000,
        0xbff0_0000_0000_0000,
        Ok(0x8000_0000_0000_0000),
    ),
    (
        "copysign",
        0x8000_0000_0000_0000,
        0xc008_0000_0000_0000,
        Ok(0x8000_0000_0000_0000),
    ),
    (
        "copysign",
        0x4000_0000_0000_0000,
        0x4024_0000_0000_0000,
        Ok(0x4000_0000_0000_0000),
    ),
    (
        "copysign",
        0x4000_0000_0000_0000,
        0x4091_3000_0000_0000,
        Ok(0x4000_0000_0000_0000),
    ),
    (
        "copysign",
        0xc020_0000_0000_0000,
        0x3fd5_5555_5555_5555,
        Ok(0x4020_0000_0000_0000),
    ),
    (
        "copysign",
        0x4024_0000_0000_0000,
        0x4008_0000_0000_0000,
        Ok(0x4024_0000_0000_0000),
    ),
    (
        "copysign",
        0xc024_0000_0000_0000,
        0x4008_0000_0000_0000,
        Ok(0x4024_0000_0000_0000),
    ),
    (
        "copysign",
        0x4014_0000_0000_0000,
        0xc008_0000_0000_0000,
        Ok(0xc014_0000_0000_0000),
    ),
    (
        "copysign",
        0x7ff0_0000_0000_0000,
        0x4000_0000_0000_0000,
        Ok(0x7ff0_0000_0000_0000),
    ),
    (
        "copysign",
        0x4000_0000_0000_0000,
        0x7ff0_0000_0000_0000,
        Ok(0x4000_0000_0000_0000),
    ),
    (
        "copysign",
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        Ok(0xfff0_0000_0000_0000),
    ),
    (
        "copysign",
        0x7ff8_0000_0000_0000,
        0x0000_0000_0000_0000,
        Ok(0x7ff8_0000_0000_0000),
    ),
    (
        "copysign",
        0x3ff0_0000_0000_0000,
        0x7ff8_0000_0000_0000,
        Ok(0x3ff0_0000_0000_0000),
    ),
    (
        "copysign",
        0x3fe0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        Ok(0xbfe0_0000_0000_0000),
    ),
    (
        "copysign",
        0xfff0_0000_0000_0000,
        0x4008_0000_0000_0000,
        Ok(0x7ff0_0000_0000_0000),
    ),
    (
        "copysign",
        0xfff0_0000_0000_0000,
        0xc008_0000_0000_0000,
        Ok(0xfff0_0000_0000_0000),
    ),
    (
        "copysign",
        0x7fe1_ccf3_85eb_c8a0,
        0x4024_0000_0000_0000,
        Ok(0x7fe1_ccf3_85eb_c8a0),
    ),
    (
        "copysign",
        0x401e_0000_0000_0000,
        0x4004_0000_0000_0000,
        Ok(0x401e_0000_0000_0000),
    ),
    (
        "copysign",
        0x4018_0000_0000_0000,
        0x4010_0000_0000_0000,
        Ok(0x4018_0000_0000_0000),
    ),
    (
        "copysign",
        0xc000_0000_0000_0000,
        0x3fe0_0000_0000_0000,
        Ok(0x4000_0000_0000_0000),
    ),
    (
        "fmod",
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        Err(("ValueError", "math domain error")),
    ),
    (
        "fmod",
        0x3ff0_0000_0000_0000,
        0x0000_0000_0000_0000,
        Err(("ValueError", "math domain error")),
    ),
    (
        "fmod",
        0x0000_0000_0000_0000,
        0xbff0_0000_0000_0000,
        Ok(0x0000_0000_0000_0000),
    ),
    (
        "fmod",
        0x8000_0000_0000_0000,
        0xc008_0000_0000_0000,
        Ok(0x8000_0000_0000_0000),
    ),
    (
        "fmod",
        0x4000_0000_0000_0000,
        0x4024_0000_0000_0000,
        Ok(0x4000_0000_0000_0000),
    ),
    (
        "fmod",
        0x4000_0000_0000_0000,
        0x4091_3000_0000_0000,
        Ok(0x4000_0000_0000_0000),
    ),
    (
        "fmod",
        0xc020_0000_0000_0000,
        0x3fd5_5555_5555_5555,
        Ok(0xbcc0_0000_0000_0000),
    ),
    (
        "fmod",
        0x4024_0000_0000_0000,
        0x4008_0000_0000_0000,
        Ok(0x3ff0_0000_0000_0000),
    ),
    (
        "fmod",
        0xc024_0000_0000_0000,
        0x4008_0000_0000_0000,
        Ok(0xbff0_0000_0000_0000),
    ),
    (
        "fmod",
        0x4014_0000_0000_0000,
        0xc008_0000_0000_0000,
        Ok(0x4000_0000_0000_0000),
    ),
    (
        "fmod",
        0x7ff0_0000_0000_0000,
        0x4000_0000_0000_0000,
        Err(("ValueError", "math domain error")),
    ),
    (
        "fmod",
        0x4000_0000_0000_0000,
        0x7ff0_0000_0000_0000,
        Ok(0x4000_0000_0000_0000),
    ),
    (
        "fmod",
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        Err(("ValueError", "math domain error")),
    ),
    (
        "fmod",
        0x7ff8_0000_0000_0000,
        0x0000_0000_0000_0000,
        Ok(0x7ff8_0000_0000_0000),
    ),
    (
        "fmod",
        0x3ff0_0000_0000_0000,
        0x7ff8_0000_0000_0000,
        Ok(0x7ff8_0000_0000_0000),
    ),
    (
        "fmod",
        0x3fe0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        Ok(0x3fe0_0000_0000_0000),
    ),
    (
        "fmod",
        0xfff0_0000_0000_0000,
        0x4008_0000_0000_0000,
        Err(("ValueError", "math domain error")),
    ),
    (
        "fmod",
        0xfff0_0000_0000_0000,
        0xc008_0000_0000_0000,
        Err(("ValueError", "math domain error")),
    ),
    (
        "fmod",
        0x7fe1_ccf3_85eb_c8a0,
        0x4024_0000_0000_0000,
        Ok(0x4018_0000_0000_0000),
    ),
    (
        "fmod",
        0x401e_0000_0000_0000,
        0x4004_0000_0000_0000,
        Ok(0x0000_0000_0000_0000),
    ),
    (
        "fmod",
        0x4018_0000_0000_0000,
        0x4010_0000_0000_0000,
        Ok(0x4000_0000_0000_0000),
    ),
    (
        "fmod",
        0xc000_0000_0000_0000,
        0x3fe0_0000_0000_0000,
        Ok(0x8000_0000_0000_0000),
    ),
    (
        "remainder",
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        Err(("ValueError", "math domain error")),
    ),
    (
        "remainder",
        0x3ff0_0000_0000_0000,
        0x0000_0000_0000_0000,
        Err(("ValueError", "math domain error")),
    ),
    (
        "remainder",
        0x0000_0000_0000_0000,
        0xbff0_0000_0000_0000,
        Ok(0x0000_0000_0000_0000),
    ),
    (
        "remainder",
        0x8000_0000_0000_0000,
        0xc008_0000_0000_0000,
        Ok(0x8000_0000_0000_0000),
    ),
    (
        "remainder",
        0x4000_0000_0000_0000,
        0x4024_0000_0000_0000,
        Ok(0x4000_0000_0000_0000),
    ),
    (
        "remainder",
        0x4000_0000_0000_0000,
        0x4091_3000_0000_0000,
        Ok(0x4000_0000_0000_0000),
    ),
    (
        "remainder",
        0xc020_0000_0000_0000,
        0x3fd5_5555_5555_5555,
        Ok(0xbcc0_0000_0000_0000),
    ),
    (
        "remainder",
        0x4024_0000_0000_0000,
        0x4008_0000_0000_0000,
        Ok(0x3ff0_0000_0000_0000),
    ),
    (
        "remainder",
        0xc024_0000_0000_0000,
        0x4008_0000_0000_0000,
        Ok(0xbff0_0000_0000_0000),
    ),
    (
        "remainder",
        0x4014_0000_0000_0000,
        0xc008_0000_0000_0000,
        Ok(0xbff0_0000_0000_0000),
    ),
    (
        "remainder",
        0x7ff0_0000_0000_0000,
        0x4000_0000_0000_0000,
        Err(("ValueError", "math domain error")),
    ),
    (
        "remainder",
        0x4000_0000_0000_0000,
        0x7ff0_0000_0000_0000,
        Ok(0x4000_0000_0000_0000),
    ),
    (
        "remainder",
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        Err(("ValueError", "math domain error")),
    ),
    (
        "remainder",
        0x7ff8_0000_0000_0000,
        0x0000_0000_0000_0000,
        Ok(0x7ff8_0000_0000_0000),
    ),
    (
        "remainder",
        0x3ff0_0000_0000_0000,
        0x7ff8_0000_0000_0000,
        Ok(0x7ff8_0000_0000_0000),
    ),
    (
        "remainder",
        0x3fe0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        Ok(0x3fe0_0000_0000_0000),
    ),
    (
        "remainder",
        0xfff0_0000_0000_0000,
        0x4008_0000_0000_0000,
        Err(("ValueError", "math domain error")),
    ),
    (
        "remainder",
        0xfff0_0000_0000_0000,
        0xc008_0000_0000_0000,
        Err(("ValueError", "math domain error")),
    ),
    (
        "remainder",
        0x7fe1_ccf3_85eb_c8a0,
        0x4024_0000_0000_0000,
        Ok(0xc010_0000_0000_0000),
    ),
    (
        "remainder",
        0x401e_0000_0000_0000,
        0x4004_0000_0000_0000,
        Ok(0x0000_0000_0000_0000),
    ),
    (
        "remainder",
        0x4018_0000_0000_0000,
        0x4010_0000_0000_0000,
        Ok(0xc000_0000_0000_0000),
    ),
    (
        "remainder",
        0xc000_0000_0000_0000,
        0x3fe0_0000_0000_0000,
        Ok(0x8000_0000_0000_0000),
    ),
    (
        "pow",
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0000,
        Ok(0x3ff0_0000_0000_0000),
    ),
    (
        "pow",
        0x3ff0_0000_0000_0000,
        0x0000_0000_0000_0000,
        Ok(0x3ff0_0000_0000_0000),
    ),
    (
        "pow",
        0x0000_0000_0000_0000,
        0xbff0_0000_0000_0000,
        Err(("ValueError", "math domain error")),
    ),
    (
        "pow",
        0x8000_0000_0000_0000,
        0xc008_0000_0000_0000,
        Err(("ValueError", "math domain error")),
    ),
    (
        "pow",
        0x4000_0000_0000_0000,
        0x4024_0000_0000_0000,
        Ok(0x4090_0000_0000_0000),
    ),
    (
        "pow",
        0x4000_0000_0000_0000,
        0x4091_3000_0000_0000,
        Err(("OverflowError", "math range error")),
    ),
    (
        "pow",
        0xc020_0000_0000_0000,
        0x3fd5_5555_5555_5555,
        Err(("ValueError", "math domain error")),
    ),
    (
        "pow",
        0x4024_0000_0000_0000,
        0x4008_0000_0000_0000,
        Ok(0x408f_4000_0000_0000),
    ),
    (
        "pow",
        0xc024_0000_0000_0000,
        0x4008_0000_0000_0000,
        Ok(0xc08f_4000_0000_0000),
    ),
    (
        "pow",
        0x4014_0000_0000_0000,
        0xc008_0000_0000_0000,
        Ok(0x3f80_624d_d2f1_a9fc),
    ),
    (
        "pow",
        0x7ff0_0000_0000_0000,
        0x4000_0000_0000_0000,
        Ok(0x7ff0_0000_0000_0000),
    ),
    (
        "pow",
        0x4000_0000_0000_0000,
        0x7ff0_0000_0000_0000,
        Ok(0x7ff0_0000_0000_0000),
    ),
    (
        "pow",
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        Ok(0x0000_0000_0000_0000),
    ),
    (
        "pow",
        0x7ff8_0000_0000_0000,
        0x0000_0000_0000_0000,
        Ok(0x3ff0_0000_0000_0000),
    ),
    (
        "pow",
        0x3ff0_0000_0000_0000,
        0x7ff8_0000_0000_0000,
        Ok(0x3ff0_0000_0000_0000),
    ),
    (
        "pow",
        0x3fe0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        Ok(0x7ff0_0000_0000_0000),
    ),
    (
        "pow",
        0xfff0_0000_0000_0000,
        0x4008_0000_0000_0000,
        Ok(0xfff0_0000_0000_0000),
    ),
    (
        "pow",
        0xfff0_0000_0000_0000,
        0xc008_0000_0000_0000,
        Ok(0x8000_0000_0000_0000),
    ),
    (
        "pow",
        0x7fe1_ccf3_85eb_c8a0,
        0x4024_0000_0000_0000,
        Err(("OverflowError", "math range error")),
    ),
    (
        "pow",
        0x401e_0000_0000_0000,
        0x4004_0000_0000_0000,
        Ok(0x4063_4180_c5c2_0783),
    ),
    (
        "pow",
        0x4018_0000_0000_0000,
        0x4010_0000_0000_0000,
        Ok(0x4094_4000_0000_0000),
    ),
    (
        "pow",
        0xc000_0000_0000_0000,
        0x3fe0_0000_0000_0000,
        Err(("ValueError", "math domain error")),
    ),
];

/// `(function, x, outcome)` for the logarithms of a `float`.
const LOGS: &[(&str, u64, Outcome)] = &[
    (
        "log",
        0x0000_0000_0000_0000,
        Err(("ValueError", "expected a positive input, got 0.0")),
    ),
    (
        "log",
        0x8000_0000_0000_0000,
        Err(("ValueError", "expected a positive input, got -0.0")),
    ),
    ("log", 0x3fe0_0000_0000_0000, Ok(0xbfe6_2e42_fefa_39ef)),
    (
        "log",
        0xbfe0_0000_0000_0000,
        Err(("ValueError", "expected a positive input, got -0.5")),
    ),
    ("log", 0x3ff0_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    (
        "log",
        0xbff0_0000_0000_0000,
        Err(("ValueError", "expected a positive input, got -1.0")),
    ),
    ("log", 0x4000_0000_0000_0000, Ok(0x3fe6_2e42_fefa_39ef)),
    (
        "log",
        0xc000_0000_0000_0000,
        Err(("ValueError", "expected a positive input, got -2.0")),
    ),
    ("log", 0x01a5_6e1f_c2f8_f359, Ok(0xc085_9634_47f8_7fb5)),
    ("log", 0x0000_0000_0000_0001, Ok(0xc087_4385_446d_71c3)),
    ("log", 0x7e37_e43c_8800_759c, Ok(0x4085_9634_47f8_7fb5)),
    (
        "log",
        0xfe37_e43c_8800_759c,
        Err(("ValueError", "expected a positive input, got -1e+300")),
    ),
    ("log", 0x4086_3000_0000_0000, Ok(0x401a_42d4_d1ff_8154)),
    (
        "log",
        0xc087_4c00_0000_0000,
        Err(("ValueError", "expected a positive input, got -745.5")),
    ),
    ("log", 0x4009_21fb_5444_2d18, Ok(0x3ff2_50d0_48e7_a1bd)),
    ("log", 0x7ff0_0000_0000_0000, Ok(0x7ff0_0000_0000_0000)),
    (
        "log",
        0xfff0_0000_0000_0000,
        Err(("ValueError", "expected a positive input, got -inf")),
    ),
    ("log", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("log", 0x3fb9_9999_9999_999a, Ok(0xc002_6bb1_bbb5_5515)),
    ("log", 0x4059_0000_0000_0000, Ok(0x4012_6bb1_bbb5_5516)),
    ("log", 0x3ff8_0000_0000_0000, Ok(0x3fd9_f323_ecbf_984c)),
    (
        "log2",
        0x0000_0000_0000_0000,
        Err(("ValueError", "expected a positive input, got 0.0")),
    ),
    (
        "log2",
        0x8000_0000_0000_0000,
        Err(("ValueError", "expected a positive input, got -0.0")),
    ),
    ("log2", 0x3fe0_0000_0000_0000, Ok(0xbff0_0000_0000_0000)),
    (
        "log2",
        0xbfe0_0000_0000_0000,
        Err(("ValueError", "expected a positive input, got -0.5")),
    ),
    ("log2", 0x3ff0_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    (
        "log2",
        0xbff0_0000_0000_0000,
        Err(("ValueError", "expected a positive input, got -1.0")),
    ),
    ("log2", 0x4000_0000_0000_0000, Ok(0x3ff0_0000_0000_0000)),
    (
        "log2",
        0xc000_0000_0000_0000,
        Err(("ValueError", "expected a positive input, got -2.0")),
    ),
    ("log2", 0x01a5_6e1f_c2f8_f359, Ok(0xc08f_24a0_9f1a_8b89)),
    ("log2", 0x0000_0000_0000_0001, Ok(0xc090_c800_0000_0000)),
    ("log2", 0x7e37_e43c_8800_759c, Ok(0x408f_24a0_9f1a_8b89)),
    (
        "log2",
        0xfe37_e43c_8800_759c,
        Err(("ValueError", "expected a positive input, got -1e+300")),
    ),
    ("log2", 0x4086_3000_0000_0000, Ok(0x4022_f17f_69e8_4b95)),
    (
        "log2",
        0xc087_4c00_0000_0000,
        Err(("ValueError", "expected a positive input, got -745.5")),
    ),
    ("log2", 0x4009_21fb_5444_2d18, Ok(0x3ffa_6c87_3498_ddf7)),
    ("log2", 0x7ff0_0000_0000_0000, Ok(0x7ff0_0000_0000_0000)),
    (
        "log2",
        0xfff0_0000_0000_0000,
        Err(("ValueError", "expected a positive input, got -inf")),
    ),
    ("log2", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("log2", 0x3fb9_9999_9999_999a, Ok(0xc00a_934f_0979_a371)),
    ("log2", 0x4059_0000_0000_0000, Ok(0x401a_934f_0979_a371)),
    ("log2", 0x3ff8_0000_0000_0000, Ok(0x3fe2_b803_473f_7ad1)),
    (
        "log10",
        0x0000_0000_0000_0000,
        Err(("ValueError", "expected a positive input, got 0.0")),
    ),
    (
        "log10",
        0x8000_0000_0000_0000,
        Err(("ValueError", "expected a positive input, got -0.0")),
    ),
    ("log10", 0x3fe0_0000_0000_0000, Ok(0xbfd3_4413_509f_79ff)),
    (
        "log10",
        0xbfe0_0000_0000_0000,
        Err(("ValueError", "expected a positive input, got -0.5")),
    ),
    ("log10", 0x3ff0_0000_0000_0000, Ok(0x0000_0000_0000_0000)),
    (
        "log10",
        0xbff0_0000_0000_0000,
        Err(("ValueError", "expected a positive input, got -1.0")),
    ),
    ("log10", 0x4000_0000_0000_0000, Ok(0x3fd3_4413_509f_79ff)),
    (
        "log10",
        0xc000_0000_0000_0000,
        Err(("ValueError", "expected a positive input, got -2.0")),
    ),
    ("log10", 0x01a5_6e1f_c2f8_f359, Ok(0xc072_c000_0000_0000)),
    ("log10", 0x0000_0000_0000_0001, Ok(0xc074_34e6_420f_4374)),
    ("log10", 0x7e37_e43c_8800_759c, Ok(0x4072_c000_0000_0000)),
    (
        "log10",
        0xfe37_e43c_8800_759c,
        Err(("ValueError", "expected a positive input, got -1e+300")),
    ),
    ("log10", 0x4086_3000_0000_0000, Ok(0x4006_cf60_8981_902a)),
    (
        "log10",
        0xc087_4c00_0000_0000,
        Err(("ValueError", "expected a positive input, got -745.5")),
    ),
    ("log10", 0x4009_21fb_5444_2d18, Ok(0x3fdf_d14d_b31b_a3ba)),
    ("log10", 0x7ff0_0000_0000_0000, Ok(0x7ff0_0000_0000_0000)),
    (
        "log10",
        0xfff0_0000_0000_0000,
        Err(("ValueError", "expected a positive input, got -inf")),
    ),
    ("log10", 0x7ff8_0000_0000_0000, Ok(0x7ff8_0000_0000_0000)),
    ("log10", 0x3fb9_9999_9999_999a, Ok(0xbff0_0000_0000_0000)),
    ("log10", 0x4059_0000_0000_0000, Ok(0x4000_0000_0000_0000)),
    ("log10", 0x3ff8_0000_0000_0000, Ok(0x3fc6_8a28_8b60_b7fc)),
];

/// `(function, n, outcome)` for the logarithms of an `int`.
const INT_LOGS: &[(&str, &str, Outcome)] = &[
    ("log", "1", Ok(0x0000_0000_0000_0000)),
    ("log", "2", Ok(0x3fe6_2e42_fefa_39ef)),
    ("log", "8", Ok(0x4000_a2b2_3f3b_ab73)),
    ("log", "10", Ok(0x4002_6bb1_bbb5_5516)),
    ("log", "0", Err(("ValueError", "expected a positive input"))),
    (
        "log",
        "-3",
        Err(("ValueError", "expected a positive input")),
    ),
    (
        "log",
        "89884656743115795386465259539451236680898848947115328636715040578866337902750481566354238661203768010560056939935696678829394884407208311246423715319737062188883946712432742638151109800623047059726541476042502884419075341171231440736956555270413618581675255342293149119973622969239858152417678164812112068608",
        Ok(0x4086_28b7_6e3a_7b61),
    ),
    (
        "log",
        "179769313486231590772930519078902473361797697894230657273430081157732675805500963132708477322407536021120113879871393357658789768814416622492847430639474124377767893424865485276302219601246094119453082952085005768838150682342462881473913110540827237163350510684586298239947245938479716304835356329624224137216",
        Ok(0x4086_2e42_fefa_39ef),
    ),
    (
        "log",
        "10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        Ok(0x408c_c845_b54b_54f1),
    ),
    (
        "log",
        "13582985290493858492773514283592667786034938469317445497485196697278130927542418487205392083207560592298578262953847383475038725543234929971155548342800628721885763499406390331782864144164680730766837160526223176512798435772129956553355286032203080380775759732320198985094884004069116123084147875437183658467465148948790552744165377",
        Ok(0x4087_d3b1_f7e6_cc3c),
    ),
    (
        "log",
        "1322070819480806636890455259752144365965422032752148167664920368226828597346704899540778313850608061963909777696872582355950954582100618911865342725257953674027620225198320803878014774228964841274390400117588618041128947815623094438061566173054086674490506178125480344405547054397038895817465368254916136220830268563778582290228416398307887896918556404084898937609373242171846359938695516765018940588109060426089671438864102814350385648747165832010614366132173102768902855220001",
        Ok(0x4091_2a72_fbcc_f4f8),
    ),
    ("log2", "1", Ok(0x0000_0000_0000_0000)),
    ("log2", "2", Ok(0x3ff0_0000_0000_0000)),
    ("log2", "8", Ok(0x4008_0000_0000_0000)),
    ("log2", "10", Ok(0x400a_934f_0979_a371)),
    (
        "log2",
        "0",
        Err(("ValueError", "expected a positive input")),
    ),
    (
        "log2",
        "-3",
        Err(("ValueError", "expected a positive input")),
    ),
    (
        "log2",
        "89884656743115795386465259539451236680898848947115328636715040578866337902750481566354238661203768010560056939935696678829394884407208311246423715319737062188883946712432742638151109800623047059726541476042502884419075341171231440736956555270413618581675255342293149119973622969239858152417678164812112068608",
        Ok(0x408f_f800_0000_0000),
    ),
    (
        "log2",
        "179769313486231590772930519078902473361797697894230657273430081157732675805500963132708477322407536021120113879871393357658789768814416622492847430639474124377767893424865485276302219601246094119453082952085005768838150682342462881473913110540827237163350510684586298239947245938479716304835356329624224137216",
        Ok(0x4090_0000_0000_0000),
    ),
    (
        "log2",
        "10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        Ok(0x4094_c315_bf67_07b1),
    ),
    (
        "log2",
        "13582985290493858492773514283592667786034938469317445497485196697278130927542418487205392083207560592298578262953847383475038725543234929971155548342800628721885763499406390331782864144164680730766837160526223176512798435772129956553355286032203080380775759732320198985094884004069116123084147875437183658467465148948790552744165377",
        Ok(0x4091_3000_0000_0000),
    ),
    (
        "log2",
        "1322070819480806636890455259752144365965422032752148167664920368226828597346704899540778313850608061963909777696872582355950954582100618911865342725257953674027620225198320803878014774228964841274390400117588618041128947815623094438061566173054086674490506178125480344405547054397038895817465368254916136220830268563778582290228416398307887896918556404084898937609373242171846359938695516765018940588109060426089671438864102814350385648747165832010614366132173102768902855220001",
        Ok(0x4098_c3d9_99c9_fef8),
    ),
    ("log10", "1", Ok(0x0000_0000_0000_0000)),
    ("log10", "2", Ok(0x3fd3_4413_509f_79ff)),
    ("log10", "8", Ok(0x3fec_e61c_f8ef_36fe)),
    ("log10", "10", Ok(0x3ff0_0000_0000_0000)),
    (
        "log10",
        "0",
        Err(("ValueError", "expected a positive input")),
    ),
    (
        "log10",
        "-3",
        Err(("ValueError", "expected a positive input")),
    ),
    (
        "log10",
        "89884656743115795386465259539451236680898848947115328636715040578866337902750481566354238661203768010560056939935696678829394884407208311246423715319737062188883946712432742638151109800623047059726541476042502884419075341171231440736956555270413618581675255342293149119973622969239858152417678164812112068608",
        Ok(0x4073_3f42_4bcb_5220),
    ),
    (
        "log10",
        "179769313486231590772930519078902473361797697894230657273430081157732675805500963132708477322407536021120113879871393357658789768814416622492847430639474124377767893424865485276302219601246094119453082952085005768838150682342462881473913110540827237163350510684586298239947245938479716304835356329624224137216",
        Ok(0x4073_4413_509f_79ff),
    ),
    (
        "log10",
        "10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        Ok(0x4079_0000_0000_0000),
    ),
    (
        "log10",
        "13582985290493858492773514283592667786034938469317445497485196697278130927542418487205392083207560592298578262953847383475038725543234929971155548342800628721885763499406390331782864144164680730766837160526223176512798435772129956553355286032203080380775759732320198985094884004069116123084147875437183658467465148948790552744165377",
        Ok(0x4074_b220_bf9b_500d),
    ),
    (
        "log10",
        "1322070819480806636890455259752144365965422032752148167664920368226828597346704899540778313850608061963909777696872582355950954582100618911865342725257953674027620225198320803878014774228964841274390400117588618041128947815623094438061566173054086674490506178125480344405547054397038895817465368254916136220830268563778582290228416398307887896918556404084898937609373242171846359938695516765018940588109060426089671438864102814350385648747165832010614366132173102768902855220001",
        Ok(0x407d_d1f0_a8c9_f6f9),
    ),
];

/// `(function, n, outcome)` for `factorial` and `isqrt`.
const INT_ONES: &[(&str, &str, Result<&str, (&str, &str)>)] = &[
    ("factorial", "0", Ok("1")),
    ("factorial", "1", Ok("1")),
    ("factorial", "5", Ok("120")),
    ("factorial", "20", Ok("2432902008176640000")),
    ("factorial", "21", Ok("51090942171709440000")),
    (
        "factorial",
        "100",
        Ok(
            "93326215443944152681699238856266700490715968264381621468592963895217599993229915608941463976156518286253697920827223758251185210916864000000000000000000000000",
        ),
    ),
    (
        "factorial",
        "-1",
        Err(("ValueError", "factorial() not defined for negative values")),
    ),
    (
        "factorial",
        "-5",
        Err(("ValueError", "factorial() not defined for negative values")),
    ),
    (
        "factorial",
        "10000000000000000000000000000000",
        Err((
            "OverflowError",
            "factorial() argument should not exceed 9223372036854775807",
        )),
    ),
    ("isqrt", "0", Ok("0")),
    ("isqrt", "1", Ok("1")),
    ("isqrt", "5", Ok("2")),
    ("isqrt", "20", Ok("4")),
    ("isqrt", "21", Ok("4")),
    ("isqrt", "100", Ok("10")),
    (
        "isqrt",
        "-1",
        Err(("ValueError", "isqrt() argument must be nonnegative")),
    ),
    (
        "isqrt",
        "-5",
        Err(("ValueError", "isqrt() argument must be nonnegative")),
    ),
    (
        "isqrt",
        "10000000000000000000000000000000",
        Ok("3162277660168379"),
    ),
    ("isqrt", "9223372036854775808", Ok("3037000499")),
];

/// `(function, n, k, outcome)` for `comb`, `perm`, `gcd` and `lcm`.
const INT_TWOS: &[(&str, &str, &str, Result<&str, (&str, &str)>)] = &[
    ("comb", "5", "2", Ok("10")),
    ("comb", "10", "3", Ok("120")),
    ("comb", "50", "25", Ok("126410606437752")),
    ("comb", "5", "6", Ok("0")),
    ("comb", "5", "0", Ok("1")),
    ("comb", "0", "0", Ok("1")),
    (
        "comb",
        "-1",
        "2",
        Err(("ValueError", "n must be a non-negative integer")),
    ),
    (
        "comb",
        "5",
        "-1",
        Err(("ValueError", "k must be a non-negative integer")),
    ),
    (
        "comb",
        "1000000000000000000000000000000",
        "2",
        Ok("499999999999999999999999999999500000000000000000000000000000"),
    ),
    ("comb", "100", "98", Ok("4950")),
    (
        "comb",
        "-12",
        "18",
        Err(("ValueError", "n must be a non-negative integer")),
    ),
    ("comb", "0", "7", Ok("0")),
    (
        "comb",
        "12",
        "-18",
        Err(("ValueError", "k must be a non-negative integer")),
    ),
    ("comb", "0", "0", Ok("1")),
    ("perm", "5", "2", Ok("20")),
    ("perm", "10", "3", Ok("720")),
    (
        "perm",
        "50",
        "25",
        Ok("1960781468160819415703172080467968000000"),
    ),
    ("perm", "5", "6", Ok("0")),
    ("perm", "5", "0", Ok("1")),
    ("perm", "0", "0", Ok("1")),
    (
        "perm",
        "-1",
        "2",
        Err(("ValueError", "n must be a non-negative integer")),
    ),
    (
        "perm",
        "5",
        "-1",
        Err(("ValueError", "k must be a non-negative integer")),
    ),
    (
        "perm",
        "1000000000000000000000000000000",
        "2",
        Ok("999999999999999999999999999999000000000000000000000000000000"),
    ),
    (
        "perm",
        "100",
        "98",
        Ok(
            "46663107721972076340849619428133350245357984132190810734296481947608799996614957804470731988078259143126848960413611879125592605458432000000000000000000000000",
        ),
    ),
    (
        "perm",
        "-12",
        "18",
        Err(("ValueError", "n must be a non-negative integer")),
    ),
    ("perm", "0", "7", Ok("0")),
    (
        "perm",
        "12",
        "-18",
        Err(("ValueError", "k must be a non-negative integer")),
    ),
    ("perm", "0", "0", Ok("1")),
    ("gcd", "5", "2", Ok("1")),
    ("gcd", "10", "3", Ok("1")),
    ("gcd", "50", "25", Ok("25")),
    ("gcd", "5", "6", Ok("1")),
    ("gcd", "5", "0", Ok("5")),
    ("gcd", "0", "0", Ok("0")),
    ("gcd", "-1", "2", Ok("1")),
    ("gcd", "5", "-1", Ok("1")),
    ("gcd", "1000000000000000000000000000000", "2", Ok("2")),
    ("gcd", "100", "98", Ok("2")),
    ("gcd", "-12", "18", Ok("6")),
    ("gcd", "0", "7", Ok("7")),
    ("gcd", "12", "-18", Ok("6")),
    ("gcd", "0", "0", Ok("0")),
    ("lcm", "5", "2", Ok("10")),
    ("lcm", "10", "3", Ok("30")),
    ("lcm", "50", "25", Ok("50")),
    ("lcm", "5", "6", Ok("30")),
    ("lcm", "5", "0", Ok("0")),
    ("lcm", "0", "0", Ok("0")),
    ("lcm", "-1", "2", Ok("2")),
    ("lcm", "5", "-1", Ok("5")),
    (
        "lcm",
        "1000000000000000000000000000000",
        "2",
        Ok("1000000000000000000000000000000"),
    ),
    ("lcm", "100", "98", Ok("4900")),
    ("lcm", "-12", "18", Ok("36")),
    ("lcm", "0", "7", Ok("0")),
    ("lcm", "12", "-18", Ok("36")),
    ("lcm", "0", "0", Ok("0")),
];

/// Returns the class name and message of an exception, and releases it.
fn exception(error: *mut crate::exception::ExceptionObject) -> (String, String) {
    // SAFETY: the runtime returned a live exception, released here; its message is a live
    // string read before the release.
    unsafe {
        let kind = (*error).class().name().to_owned();
        let message = (*error).message;
        let count = crate::string::length(message);
        let text = (0..count)
            .map(|index| char::from_u32(crate::string::code_point(message, index)).unwrap_or('?'))
            .collect();
        crate::refcount::typy_rt_decrement(error.cast());
        (kind, text)
    }
}

/// Compares a result with the outcome CPython gives; NaNs compare as NaNs.
fn check(found: Raising<f64>, expected: &Outcome, case: &str) {
    match expected {
        Ok(bits) => {
            assert!(
                found.exception.is_null(),
                "{case}: {:?}",
                exception(found.exception)
            );
            let wanted = f64::from_bits(*bits);
            if wanted.is_nan() {
                assert!(found.value.is_nan(), "{case}: {}", found.value);
            } else {
                assert_eq!(
                    found.value.to_bits(),
                    *bits,
                    "{case}: {} for {wanted}",
                    found.value
                );
            }
        }
        Err((kind, message)) => {
            assert!(!found.exception.is_null(), "{case}: gave {}", found.value);
            let (found_kind, found_message) = exception(found.exception);
            assert_eq!(
                (found_kind.as_str(), found_message.as_str()),
                (*kind, *message),
                "{case}"
            );
        }
    }
}

fn one(name: &str, x: f64) -> Raising<f64> {
    match name {
        "acos" => typy_rt_math_acos(x),
        "acosh" => typy_rt_math_acosh(x),
        "asin" => typy_rt_math_asin(x),
        "asinh" => typy_rt_math_asinh(x),
        "atan" => typy_rt_math_atan(x),
        "atanh" => typy_rt_math_atanh(x),
        "cbrt" => typy_rt_math_cbrt(x),
        "cos" => typy_rt_math_cos(x),
        "cosh" => typy_rt_math_cosh(x),
        "erf" => typy_rt_math_erf(x),
        "erfc" => typy_rt_math_erfc(x),
        "exp" => typy_rt_math_exp(x),
        "exp2" => typy_rt_math_exp2(x),
        "expm1" => typy_rt_math_expm1(x),
        "log1p" => typy_rt_math_log1p(x),
        "sin" => typy_rt_math_sin(x),
        "sinh" => typy_rt_math_sinh(x),
        "sqrt" => typy_rt_math_sqrt(x),
        "tan" => typy_rt_math_tan(x),
        "tanh" => typy_rt_math_tanh(x),
        "log" => typy_rt_math_log_float(x),
        "log2" => typy_rt_math_log2_float(x),
        _ => typy_rt_math_log10_float(x),
    }
}

fn two(name: &str, x: f64, y: f64) -> Raising<f64> {
    match name {
        "atan2" => typy_rt_math_atan2(x, y),
        "copysign" => typy_rt_math_copysign(x, y),
        "fmod" => typy_rt_math_fmod(x, y),
        "remainder" => typy_rt_math_remainder(x, y),
        _ => typy_rt_math_pow(x, y),
    }
}

/// Returns the `int` word of decimal digits.
fn int_word(digits: &str) -> i64 {
    let text = crate::string::from_str(digits);
    // SAFETY: the string is live; the base is the `int` word of 10.
    let parsed = unsafe { crate::int::typy_rt_int_from_str(text, 21) };
    // SAFETY: the string is no longer used.
    unsafe { crate::refcount::typy_rt_decrement(text.cast()) };
    assert!(parsed.exception.is_null(), "{digits} is an int");
    parsed.value
}

/// Returns the decimal digits of an `int` word, and releases it.
fn digits(word: i64) -> String {
    // SAFETY: the word is live until it is released here.
    let text = unsafe { crate::int::typy_rt_int_to_str(word) }.value;
    // SAFETY: the string is live, and each index is inside it.
    let written = unsafe {
        (0..crate::string::length(text))
            .map(|index| char::from_u32(crate::string::code_point(text, index)).unwrap_or('?'))
            .collect()
    };
    // SAFETY: neither is used again.
    unsafe {
        crate::refcount::typy_rt_decrement(text.cast());
        crate::refcount::typy_rt_int_decrement(word);
    }
    written
}

fn check_int(found: Raising<i64>, expected: &Result<&str, (&str, &str)>, case: &str) {
    match expected {
        Ok(wanted) => {
            assert!(
                found.exception.is_null(),
                "{case}: {:?}",
                exception(found.exception)
            );
            assert_eq!(digits(found.value), *wanted, "{case}");
        }
        Err((kind, message)) => {
            assert!(!found.exception.is_null(), "{case}: gave a value");
            let (found_kind, found_message) = exception(found.exception);
            assert_eq!(
                (found_kind.as_str(), found_message.as_str()),
                (*kind, *message),
                "{case}"
            );
        }
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri cannot call the C maths library")]
fn every_function_of_one_float_gives_cpythons_result() {
    for (name, x, expected) in ONES.iter().chain(LOGS) {
        let x = f64::from_bits(*x);
        check(one(name, x), expected, &format!("{name}({x})"));
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri cannot call the C maths library")]
fn every_function_of_two_floats_gives_cpythons_result() {
    for (name, x, y, expected) in TWOS {
        let (x, y) = (f64::from_bits(*x), f64::from_bits(*y));
        check(two(name, x, y), expected, &format!("{name}({x}, {y})"));
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri cannot call the C maths library")]
fn the_logarithm_of_an_int_of_any_size_gives_cpythons_result() {
    for (name, value, expected) in INT_LOGS {
        let word = int_word(value);
        // SAFETY: the word is live.
        let found = unsafe {
            match *name {
                "log" => typy_rt_math_log_int(word),
                "log2" => typy_rt_math_log2_int(word),
                _ => typy_rt_math_log10_int(word),
            }
        };
        // SAFETY: the word is no longer used.
        unsafe { crate::refcount::typy_rt_int_decrement(word) };
        check(found, expected, &format!("{name}({value})"));
    }
}

#[test]
fn every_integer_function_gives_cpythons_result() {
    for (name, value, expected) in INT_ONES {
        let word = int_word(value);
        // SAFETY: the word is live.
        let found = unsafe {
            if *name == "factorial" {
                typy_rt_math_factorial(word)
            } else {
                typy_rt_math_isqrt(word)
            }
        };
        // SAFETY: the word is no longer used.
        unsafe { crate::refcount::typy_rt_int_decrement(word) };
        check_int(found, expected, &format!("{name}({value})"));
    }
    for (name, a, b, expected) in INT_TWOS {
        let (left, right) = (int_word(a), int_word(b));
        // SAFETY: the words are live.
        let found = unsafe {
            match *name {
                "comb" => typy_rt_math_comb(left, right),
                "perm" => typy_rt_math_perm(left, right),
                "gcd" => Raising::ok(typy_rt_math_gcd(left, right)),
                _ => Raising::ok(typy_rt_math_lcm(left, right)),
            }
        };
        // SAFETY: the words are no longer used.
        unsafe {
            crate::refcount::typy_rt_int_decrement(left);
            crate::refcount::typy_rt_int_decrement(right);
        }
        check_int(found, expected, &format!("{name}({a}, {b})"));
    }
}

#[test]
fn the_logarithm_of_a_base_divides_by_its_logarithm() {
    check(
        typy_rt_math_log_divide(3.0, 1.5),
        &Ok(2.0_f64.to_bits()),
        "log(x, base)",
    );
    check(
        typy_rt_math_log_divide(1.0, 0.0),
        &Err(("ZeroDivisionError", "division by zero")),
        "log(x, 1)",
    );
}

mod more;

fn check_flag(found: Raising<bool>, expected: &Result<bool, (&str, &str)>, case: &str) {
    match expected {
        Ok(wanted) => {
            assert!(
                found.exception.is_null(),
                "{case}: {:?}",
                exception(found.exception)
            );
            assert_eq!(found.value, *wanted, "{case}");
        }
        Err((kind, message)) => {
            assert!(!found.exception.is_null(), "{case}: gave {}", found.value);
            let (found_kind, found_message) = exception(found.exception);
            assert_eq!(
                (found_kind.as_str(), found_message.as_str()),
                (*kind, *message),
                "{case}"
            );
        }
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri cannot call the C maths library")]
fn hypot_gives_cpythons_correctly_rounded_norm() {
    for (coordinates, expected) in more::HYPOT {
        let mut vector: Vec<f64> = coordinates
            .iter()
            .map(|bits| crate::libm::absolute(f64::from_bits(*bits)))
            .collect();
        let max = vector
            .iter()
            .copied()
            .fold(0.0_f64, |max, x| if x > max { x } else { max });
        let found_nan = vector.iter().any(|x| x.is_nan());
        let found = super::special::vector_norm(&mut vector, max, found_nan);
        check(
            Raising::ok(found),
            &Ok(*expected),
            &format!("hypot{coordinates:x?}"),
        );
    }
    let list = crate::list::new_list(crate::list::ItemKind::Float, 2);
    // SAFETY: the list is live and takes the bits of each float.
    unsafe {
        crate::list::push_retained(list, 3.0_f64.to_bits());
        crate::list::push_retained(list, (-4.0_f64).to_bits());
        assert_eq!(typy_rt_math_hypot(list), 5.0);
        crate::refcount::typy_rt_decrement(list.cast());
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri cannot call the C maths library")]
fn ldexp_takes_an_int_of_any_size() {
    for (x, exponent, expected) in more::LDEXP {
        let word = int_word(exponent);
        // SAFETY: the word is live.
        let found = unsafe { typy_rt_math_ldexp(f64::from_bits(*x), word) };
        // SAFETY: the word is no longer used.
        unsafe { crate::refcount::typy_rt_int_decrement(word) };
        let expected = expected.map_err(|(kind, message)| (kind, message));
        check(found, &expected, &format!("ldexp({x:#x}, {exponent})"));
    }
}

#[test]
fn nextafter_takes_an_int_of_any_size() {
    for (x, y, steps, expected) in more::NEXTAFTER {
        let word = int_word(steps);
        // SAFETY: the word is live.
        let found = unsafe { typy_rt_math_nextafter(f64::from_bits(*x), f64::from_bits(*y), word) };
        // SAFETY: the word is no longer used.
        unsafe { crate::refcount::typy_rt_int_decrement(word) };
        check(
            found,
            expected,
            &format!("nextafter({x:#x}, {y:#x}, steps={steps})"),
        );
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri cannot call the C maths library")]
fn the_functions_that_never_fail_give_cpythons_bits() {
    for (name, x, expected) in more::PLAIN {
        let x = f64::from_bits(*x);
        let found = match *name {
            "fabs" => typy_rt_math_fabs(x),
            "degrees" => typy_rt_math_degrees(x),
            "radians" => typy_rt_math_radians(x),
            "ulp" => typy_rt_math_ulp(x),
            "floor" => typy_rt_math_floor(x),
            "ceil" => typy_rt_math_ceil(x),
            "frexp_mantissa" => typy_rt_math_frexp_mantissa(x),
            "modf_fraction" => typy_rt_math_modf_fraction(x),
            _ => typy_rt_math_modf_integral(x),
        };
        check(Raising::ok(found), &Ok(*expected), &format!("{name}({x})"));
    }
    for (x, exponent) in more::EXPONENTS {
        let word = typy_rt_math_frexp_exponent(f64::from_bits(*x));
        assert_eq!(crate::word::inline_value(word), *exponent, "frexp({x:#x})");
    }
    for (name, x, y, expected) in more::EXTREMES {
        let (x, y) = (f64::from_bits(*x), f64::from_bits(*y));
        let found = if *name == "fmax" {
            typy_rt_math_fmax(x, y)
        } else {
            typy_rt_math_fmin(x, y)
        };
        check(
            Raising::ok(found),
            &Ok(*expected),
            &format!("{name}({x}, {y})"),
        );
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri cannot call the C maths library")]
fn tests_isclose_and_fma_give_cpythons_result() {
    for (name, x, expected) in more::TESTS {
        let x = f64::from_bits(*x);
        let found = match *name {
            "isfinite" => typy_rt_math_isfinite(x),
            "isinf" => typy_rt_math_isinf(x),
            "isnan" => typy_rt_math_isnan(x),
            "isnormal" => typy_rt_math_isnormal(x),
            "issubnormal" => typy_rt_math_issubnormal(x),
            _ => typy_rt_math_signbit(x),
        };
        assert_eq!(found, *expected, "{name}({x})");
    }
    for (a, b, relative, absolute, expected) in more::ISCLOSE {
        let found = typy_rt_math_isclose(
            f64::from_bits(*a),
            f64::from_bits(*b),
            f64::from_bits(*relative),
            f64::from_bits(*absolute),
        );
        check_flag(found, expected, &format!("isclose({a:#x}, {b:#x})"));
    }
    for (x, y, z, expected) in more::FMA {
        let found = typy_rt_math_fma(f64::from_bits(*x), f64::from_bits(*y), f64::from_bits(*z));
        check(found, expected, &format!("fma({x:#x}, {y:#x}, {z:#x})"));
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri cannot call the C maths library")]
fn every_pair_of_special_values_gives_cpythons_result() {
    for (name, x, y, expected) in more::SPECIAL_TWOS {
        let (x, y) = (f64::from_bits(*x), f64::from_bits(*y));
        check(two(name, x, y), expected, &format!("{name}({x}, {y})"));
    }
}
