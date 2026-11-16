//! Tests of the sort against CPython 3.15.2: for each case, the digest of the pairs of items
//! CPython's `list.sort` passed to `__lt__`, in order, and of the order it left the items in,
//! which a script recorded with `python3.15`. The items are made by the same generator here.

use core::cell::RefCell;

use super::{Raised, sort};

/// Each case: the pattern of the items, their number, the seed, the digest of the compared
/// pairs, and the digest of the sorted order.
const CASES: [(u64, usize, u64, u64, u64); 110] = [
    (0, 2, 1, 0x3922_09f1_4dea_4c24, 0x6925_58b0_5610_1a44),
    (0, 2, 2, 0x3922_09f1_4dea_4c24, 0x6925_58b0_5610_1a44),
    (0, 3, 1, 0x82a3_a031_02dd_4c47, 0x00d3_a45e_1b77_0966),
    (0, 3, 2, 0xc420_502a_dbae_2fc7, 0x70c9_b821_0305_9f06),
    (0, 5, 1, 0xa2f8_5a70_bc89_6cc4, 0x4a4b_fab3_ba51_f741),
    (0, 5, 2, 0x2c94_bc04_9967_5200, 0xe202_c923_853e_01e1),
    (0, 8, 1, 0x30ab_4bf4_690a_ec02, 0xc506_984b_fd01_2ea5),
    (0, 8, 2, 0xfd2c_395b_1bd9_9780, 0xd4dd_d33e_e1b3_b405),
    (0, 31, 1, 0x647c_1a22_6076_d8b3, 0x2a67_4c18_a528_e51a),
    (0, 31, 2, 0x18c5_02e9_f15d_966c, 0xe92f_e3ad_4cc9_f3fa),
    (0, 64, 1, 0xa606_3de7_9d77_ba8f, 0x39fe_f37e_3f0a_f505),
    (0, 64, 2, 0xe39b_2618_a3bf_d636, 0x52ab_2e7f_5809_0d65),
    (0, 65, 1, 0xc4b7_4b1f_c597_3cd1, 0x92ce_0056_1d62_c8c5),
    (0, 65, 2, 0xdcc7_a40f_6eb8_3d4c, 0xd5d5_c354_5b32_9125),
    (0, 100, 1, 0xdf6c_ad45_1169_5afb, 0xa581_4489_8fee_0b65),
    (0, 100, 2, 0xf2dd_f20d_11d7_207e, 0x9215_ad5b_a5f8_c8a5),
    (0, 257, 1, 0x0dca_fcff_e095_e0ee, 0xc52d_7042_1b2d_cdfe),
    (0, 257, 2, 0xff01_7d0a_b3c3_6b01, 0x7b60_8ff2_4552_28fa),
    (0, 1000, 1, 0x8409_e195_7a8f_c499, 0x94b4_4eb1_f0ff_8ac9),
    (0, 1000, 2, 0xd4e4_d431_91a1_93d0, 0x93a3_cfd9_dca9_57dd),
    (0, 3000, 1, 0x6e6b_30e0_a6a8_36ec, 0x1973_9806_d0b4_9abd),
    (0, 3000, 2, 0x5b59_c333_5f88_8bfd, 0x0b89_8aae_bd65_825d),
    (1, 2, 1, 0x3922_09f1_4dea_4c24, 0x3922_09f1_4dea_4c24),
    (1, 2, 2, 0x3922_09f1_4dea_4c24, 0x6925_58b0_5610_1a44),
    (1, 3, 1, 0x26f9_e11e_1e51_ee86, 0x9920_24ad_58b0_cce6),
    (1, 3, 2, 0xc420_502a_dbae_2fc7, 0x70c9_b821_0305_9f06),
    (1, 5, 1, 0xfee6_5f27_2e6a_0660, 0x24a0_b5cd_ba25_55a1),
    (1, 5, 2, 0xbdf7_d686_509f_3b61, 0xbde4_0bb1_8a01_afc1),
    (1, 8, 1, 0x388b_5b0c_451b_6343, 0xbbf6_02fa_d755_0905),
    (1, 8, 2, 0x79c9_2c0d_7ec4_af25, 0x3706_59f1_9a52_e585),
    (1, 31, 1, 0xa1d9_d993_5cf0_de54, 0x6dc4_6507_eedf_289a),
    (1, 31, 2, 0x0059_781b_becb_bc52, 0xbe2f_0d43_520d_4d1a),
    (1, 64, 1, 0x22c4_e498_266a_5735, 0x2482_82ac_aed0_6b05),
    (1, 64, 2, 0x068e_64e6_3bbf_a605, 0x73aa_b1e8_8b79_3d05),
    (1, 65, 1, 0xd022_f475_aadb_8fe0, 0x2cbc_83fc_9f4a_c5e5),
    (1, 65, 2, 0x312d_2dcf_8e70_c790, 0x7f40_e44c_ebc1_d7e5),
    (1, 100, 1, 0x02cb_69fa_5d2b_6fef, 0xe4e2_cbb9_3ddb_8325),
    (1, 100, 2, 0x146c_f750_7c60_b510, 0x6fe1_3b36_265d_1585),
    (1, 257, 1, 0xcaaf_deab_86a6_4f9e, 0x4890_2790_d2f9_a86a),
    (1, 257, 2, 0xd927_5085_2e7e_d146, 0xc13e_2c3c_574c_e88a),
    (1, 1000, 1, 0x55f7_7991_8495_6cde, 0xc185_7c46_2e2a_f429),
    (1, 1000, 2, 0x0324_79b3_4968_a978, 0x1784_6172_22b2_72f5),
    (1, 3000, 1, 0xfd76_36df_840a_0b2c, 0xfbb0_5c4c_1161_7cb5),
    (1, 3000, 2, 0x19b0_235c_4437_e62a, 0xd62c_853f_008f_8401),
    (2, 2, 1, 0x3922_09f1_4dea_4c24, 0x3922_09f1_4dea_4c24),
    (2, 2, 2, 0x3922_09f1_4dea_4c24, 0x3922_09f1_4dea_4c24),
    (2, 3, 1, 0xc420_502a_dbae_2fc7, 0x8023_7c86_67fa_df86),
    (2, 3, 2, 0xc420_502a_dbae_2fc7, 0x8023_7c86_67fa_df86),
    (2, 5, 1, 0xbdf7_d686_509f_3b61, 0xcd6d_f11a_60e3_75c1),
    (2, 5, 2, 0x71e8_d63c_a0fb_38c1, 0xf306_2503_7a43_8ce1),
    (2, 8, 1, 0x8f12_cccd_d7aa_00e6, 0x2fb2_a5da_63db_3105),
    (2, 8, 2, 0xdc1f_88ba_0a91_a403, 0xc4f7_420d_175a_4545),
    (2, 31, 1, 0xb8eb_56cf_1576_9efb, 0x4705_b1be_725e_c8fa),
    (2, 31, 2, 0xe6ba_13f9_3d39_0344, 0xbcc1_420a_a18e_4d1a),
    (2, 64, 1, 0xd2a4_7bc1_7c28_5316, 0x7e2a_2630_50c0_3b45),
    (2, 64, 2, 0x9f95_6dac_5930_a3f7, 0xc833_a89b_4102_3645),
    (2, 65, 1, 0xe128_c670_ddaa_0053, 0x9c91_6a9b_11d2_b9a5),
    (2, 65, 2, 0xe43b_4230_fd2c_7508, 0xf5e3_4847_f0c5_37a5),
    (2, 100, 1, 0x298a_4572_c9f3_a58f, 0xb552_ede9_8edf_d9c5),
    (2, 100, 2, 0x46b6_5f40_8ad1_6f0e, 0xc21a_5614_fa06_13c5),
    (2, 257, 1, 0xb3e9_1397_777f_0038, 0x37e9_9dab_d92f_d11e),
    (2, 257, 2, 0x33c5_35c2_21b4_1524, 0x6b78_7f3e_56be_056a),
    (2, 1000, 1, 0xe38b_00f8_ef4c_7959, 0xca49_32f6_8a32_f6f9),
    (2, 1000, 2, 0xb90a_b813_eabd_c743, 0xf80e_ce2a_89ab_3921),
    (2, 3000, 1, 0x95ae_71e7_29f7_7f96, 0x13d6_d5f3_4e72_edb9),
    (2, 3000, 2, 0x38f2_fcd9_d403_e508, 0x3412_768f_66f0_af05),
    (3, 2, 1, 0x3922_09f1_4dea_4c24, 0x6925_58b0_5610_1a44),
    (3, 2, 2, 0x3922_09f1_4dea_4c24, 0x6925_58b0_5610_1a44),
    (3, 3, 1, 0xc420_502a_dbae_2fc7, 0x70c9_b821_0305_9f06),
    (3, 3, 2, 0xc420_502a_dbae_2fc7, 0x70c9_b821_0305_9f06),
    (3, 5, 1, 0xbdf7_d686_509f_3b61, 0xbde4_0bb1_8a01_afc1),
    (3, 5, 2, 0xbdf7_d686_509f_3b61, 0xbde4_0bb1_8a01_afc1),
    (3, 8, 1, 0xda25_3099_1392_83c2, 0xb009_9f96_9b54_6f25),
    (3, 8, 2, 0xda25_3099_1392_83c2, 0xb009_9f96_9b54_6f25),
    (3, 31, 1, 0x6a1f_cf1e_0c80_879b, 0xb205_4457_3d88_689a),
    (3, 31, 2, 0x6a1f_cf1e_0c80_879b, 0xb205_4457_3d88_689a),
    (3, 64, 1, 0xd97a_3680_3c0b_a17a, 0x310e_42af_98fb_7125),
    (3, 64, 2, 0xd97a_3680_3c0b_a17a, 0x310e_42af_98fb_7125),
    (3, 65, 1, 0xc10e_288c_9a49_06e5, 0xf8a5_7f95_367b_d005),
    (3, 65, 2, 0xc10e_288c_9a49_06e5, 0xf8a5_7f95_367b_d005),
    (3, 100, 1, 0x7ed2_42d4_595c_2b66, 0x610b_068d_9980_8fe5),
    (3, 100, 2, 0x7ed2_42d4_595c_2b66, 0x610b_068d_9980_8fe5),
    (3, 257, 1, 0x57b2_44c1_e1b7_5aea, 0xb672_247d_344a_9eea),
    (3, 257, 2, 0x57b2_44c1_e1b7_5aea, 0xb672_247d_344a_9eea),
    (3, 1000, 1, 0x8113_5cf7_b9ff_545b, 0x3a84_0aab_2742_da95),
    (3, 1000, 2, 0x8113_5cf7_b9ff_545b, 0x3a84_0aab_2742_da95),
    (3, 3000, 1, 0x82d7_97ac_1dd2_e203, 0x167d_4741_a770_83f5),
    (3, 3000, 2, 0x82d7_97ac_1dd2_e203, 0x167d_4741_a770_83f5),
    (4, 2, 1, 0x3922_09f1_4dea_4c24, 0x6925_58b0_5610_1a44),
    (4, 2, 2, 0x3922_09f1_4dea_4c24, 0x6925_58b0_5610_1a44),
    (4, 3, 1, 0xc420_502a_dbae_2fc7, 0x70c9_b821_0305_9f06),
    (4, 3, 2, 0xc420_502a_dbae_2fc7, 0x70c9_b821_0305_9f06),
    (4, 5, 1, 0xb517_24d4_48db_0e86, 0xec33_085e_cf4b_8841),
    (4, 5, 2, 0x5976_4289_ce1a_ae65, 0x1ec0_362e_837c_edc1),
    (4, 8, 1, 0x5e5b_3e59_983a_c0a1, 0x0c08_3ea6_9f5b_2fa5),
    (4, 8, 2, 0x303b_0b6d_eacc_f066, 0xa113_8f4e_37b2_ed25),
    (4, 31, 1, 0x810b_e50f_e3c2_755a, 0x4d58_a89a_e857_751a),
    (4, 31, 2, 0x4d81_29bb_4943_48f6, 0x89e8_c585_68c5_0b1a),
    (4, 64, 1, 0xbab8_c33a_15b6_2c2c, 0xab82_ee3b_84f5_a245),
    (4, 64, 2, 0xd5d3_7b47_3b22_6b9c, 0xdc31_6af6_b023_c885),
    (4, 65, 1, 0x39b1_ae1c_92f7_912b, 0x3c18_2466_4a17_9b05),
    (4, 65, 2, 0x3f54_a7f6_6677_1826, 0xdbcc_ec61_4707_e765),
    (4, 100, 1, 0x74f1_8e05_40e9_c780, 0x8262_d98c_1482_8585),
    (4, 100, 2, 0x7e4c_030f_6007_db91, 0x8f8b_01b5_4237_2485),
    (4, 257, 1, 0x115c_9576_353a_eaae, 0xdc08_9a54_1af0_149e),
    (4, 257, 2, 0x05ba_067b_dee8_b9f0, 0x6fd6_4926_417c_9922),
    (4, 1000, 1, 0x7270_f094_ffc8_9d15, 0x863a_f55b_6600_4c61),
    (4, 1000, 2, 0x7767_5493_8f2b_ca8b, 0xd354_e1a5_1c8a_0755),
    (4, 3000, 1, 0xa08d_5a5f_376a_16a5, 0xe0b6_6ae4_bfc5_fa0d),
    (4, 3000, 2, 0x31ef_de7e_937a_6d81, 0x0ecb_d939_3d92_6101),
];

/// Returns the next state of the generator, a 64-bit linear congruential one.
const fn next(state: u64) -> u64 {
    state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
}

/// Returns the values of a case: random with repeats, nearly ascending, nearly descending,
/// all equal, or a sawtooth of both signs.
#[allow(clippy::cast_possible_wrap, reason = "every value is small")]
fn values(pattern: u64, size: usize, seed: u64) -> Vec<i64> {
    let mut state = seed;
    (0..size)
        .map(|index| {
            state = next(state);
            let noise = (state >> 33) as i64;
            let (index, size) = (index as i64, size as i64);
            match pattern {
                0 => noise % (size / 2 + 1),
                1 => index + noise % 3,
                2 => size - index + noise % 3,
                3 => 7,
                _ => (index % 17) * if noise % 5 == 0 { -1 } else { 1 },
            }
        })
        .collect()
}

/// Returns the FNV-1a digest of the little-endian bytes of `numbers`.
fn fnv(numbers: &[u64]) -> u64 {
    let mut digest = 0xcbf2_9ce4_8422_2325_u64;
    for number in numbers {
        for byte in number.to_le_bytes() {
            digest ^= u64::from(byte);
            digest = digest.wrapping_mul(0x0100_0000_01b3);
        }
    }
    digest
}

#[test]
fn the_sort_compares_the_items_cpython_3_15_compares_in_its_order() {
    for (pattern, size, seed, compared, sorted) in CASES {
        let values = values(pattern, size, seed);
        let mut items: Vec<usize> = (0..size).collect();
        let mut scratch = vec![0; size / 2 + 1];
        let log = RefCell::new(Vec::new());
        let less = |left: usize, right: usize| -> Result<bool, Raised> {
            log.borrow_mut().extend([left as u64, right as u64]);
            Ok(values[left] < values[right])
        };
        sort(&mut items, &mut scratch, &less).unwrap();
        let order: Vec<u64> = items.iter().map(|index| *index as u64).collect();
        assert_eq!(
            fnv(&log.borrow()),
            compared,
            "pairs of {pattern} {size} {seed}"
        );
        assert_eq!(fnv(&order), sorted, "order of {pattern} {size} {seed}");
    }
}

#[test]
fn a_comparison_that_raises_ends_the_sort_with_every_item_kept() {
    for (pattern, size, seed, ..) in CASES {
        let values = values(pattern, size, seed);
        for limit in [0, 1, 5, 40, 500] {
            let mut items: Vec<usize> = (0..size).collect();
            let mut scratch = vec![0; size / 2 + 1];
            let calls = RefCell::new(0);
            let less = |left: usize, right: usize| -> Result<bool, Raised> {
                *calls.borrow_mut() += 1;
                if *calls.borrow() > limit {
                    return Err(Raised);
                }
                Ok(values[left] < values[right])
            };
            let result = sort(&mut items, &mut scratch, &less);
            assert!(result.is_err() || *calls.borrow() <= limit);
            // The comparison after the one that raised is never made.
            assert!(*calls.borrow() <= limit + 1);
            let mut kept = items.clone();
            kept.sort_unstable();
            assert_eq!(
                kept,
                (0..size).collect::<Vec<_>>(),
                "{pattern} {size} {seed}"
            );
        }
    }
}
