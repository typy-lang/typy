//! Tests of the sort against CPython 3.15.2: for each case, the digest of the pairs of items
//! CPython's `list.sort` passed to `__lt__`, in order, and of the order it left the items in,
//! which a script recorded with `python3.15`. The items are made by the same generators here.

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

/// Each case of ascending runs whose values interleave: the number of items, of runs, the
/// longest block of a run's values among the others', the seed, and the digests of the
/// compared pairs and of the sorted order.
const INTERLEAVED: [(usize, usize, u64, u64, u64, u64); 72] = [
    (64, 2, 3, 1, 0xc85d_c677_0d7c_2d6b, 0x1645_e05b_7c90_fc25),
    (64, 2, 3, 2, 0xa3b7_c145_eb54_e4f5, 0x6be2_a299_c490_85a5),
    (64, 2, 12, 1, 0x3e69_db1c_9c94_40be, 0xa088_5d85_bdfe_b365),
    (64, 2, 12, 2, 0xff71_9667_95e7_8938, 0x3d00_4422_a14f_8865),
    (64, 2, 40, 1, 0x5c92_8d90_fc7c_547f, 0x22bc_0942_bfd8_7305),
    (64, 2, 40, 2, 0xad3b_6ad1_e049_e642, 0xdbeb_34c6_f730_ec25),
    (64, 3, 3, 1, 0x6f03_f55c_a9e0_3645, 0x5dde_f18d_638b_aba5),
    (64, 3, 3, 2, 0x0d24_c4ef_accc_cafa, 0x542f_00e4_0ec9_8fc5),
    (64, 3, 12, 1, 0xe37a_61df_ecf3_2244, 0xfcec_89b3_e441_1f85),
    (64, 3, 12, 2, 0x44f2_4cea_609b_2e2f, 0x6560_0692_7d7d_cfa5),
    (64, 3, 40, 1, 0xd97a_3680_3c0b_a17a, 0x310e_42af_98fb_7125),
    (64, 3, 40, 2, 0xe2bc_156f_d350_99b8, 0xa348_712b_82cc_1fa5),
    (64, 5, 3, 1, 0x5ced_b526_f63d_7c86, 0x7ac3_2677_ceb9_2085),
    (64, 5, 3, 2, 0x4d53_ddb0_8d09_ace5, 0x7b63_1d89_59a6_a265),
    (64, 5, 12, 1, 0xf3bd_fa5c_cd98_9c1d, 0xc759_9a9b_84fa_8d85),
    (64, 5, 12, 2, 0x12e2_6161_eb8b_9717, 0xe200_0a75_59a9_01a5),
    (64, 5, 40, 1, 0xd97a_3680_3c0b_a17a, 0x310e_42af_98fb_7125),
    (64, 5, 40, 2, 0xd97a_3680_3c0b_a17a, 0x310e_42af_98fb_7125),
    (100, 2, 3, 1, 0x9602_8170_7c2f_d892, 0xdd12_8d22_bac7_cc05),
    (100, 2, 3, 2, 0xc95a_9858_0772_6e93, 0xb61c_0c6f_1418_ff25),
    (100, 2, 12, 1, 0x2ad6_f784_8cb3_5034, 0x87dc_94ec_f6cd_ea65),
    (100, 2, 12, 2, 0x554b_25c8_6ff2_dcdf, 0x9e49_7e1a_839d_04c5),
    (100, 2, 40, 1, 0xe10e_66bd_33e0_3ebf, 0xfff8_b189_c0c7_aac5),
    (100, 2, 40, 2, 0x6625_a691_24e1_ae6c, 0xda94_384b_cc47_30e5),
    (100, 3, 3, 1, 0x4051_93a4_8f08_8ff9, 0x909a_db31_9154_a765),
    (100, 3, 3, 2, 0x61a5_8b34_b58c_766f, 0xe300_11ed_6dd6_8385),
    (100, 3, 12, 1, 0x1525_9028_a0d0_2f93, 0xf42b_dd00_0863_cc45),
    (100, 3, 12, 2, 0x4d12_4c73_3095_000c, 0xe753_9730_31a3_ca25),
    (100, 3, 40, 1, 0xd488_e14e_398a_8931, 0xb2a2_8492_7ac1_4fc5),
    (100, 3, 40, 2, 0xd271_c892_0392_c62a, 0x1a57_76d3_9e04_b465),
    (100, 5, 3, 1, 0xe410_ccca_9ecd_e260, 0x978d_ebef_88f7_ae65),
    (100, 5, 3, 2, 0xbd55_c233_6f4a_492b, 0x79fb_3ff4_8dba_6a25),
    (100, 5, 12, 1, 0x9a94_171c_d493_229c, 0xeba7_23d2_2f3a_0385),
    (100, 5, 12, 2, 0x6694_eb7a_fe97_820a, 0xd9b6_28e6_7c9b_e6c5),
    (100, 5, 40, 1, 0x7ed2_42d4_595c_2b66, 0x610b_068d_9980_8fe5),
    (100, 5, 40, 2, 0xf52e_b351_54ed_9c19, 0x65b9_cbc5_899b_8dc5),
    (257, 2, 3, 1, 0x6e4f_feee_5330_0807, 0x6ce6_a1ab_0c00_2efe),
    (257, 2, 3, 2, 0x1362_23a9_494c_8e44, 0x4d7a_bc20_2168_777a),
    (257, 2, 12, 1, 0x9596_bdc4_ec26_a9f5, 0xc588_de37_bc73_3d4a),
    (257, 2, 12, 2, 0xa2a6_e7b6_8af3_000b, 0x0630_63eb_2f83_0d2a),
    (257, 2, 40, 1, 0x2301_bc30_64c4_d78f, 0x834b_8b41_58f1_a5ea),
    (257, 2, 40, 2, 0x3ffb_948e_df96_990e, 0xc6ad_d614_7182_aa22),
    (257, 3, 3, 1, 0x8535_93ae_26ee_da93, 0xf543_6d35_16e3_9302),
    (257, 3, 3, 2, 0x83d3_6ec7_238d_ea95, 0x807e_e21b_921d_104e),
    (257, 3, 12, 1, 0x1c80_cf1e_2ad5_cd2a, 0x06c0_4910_3bd5_28ce),
    (257, 3, 12, 2, 0x5b80_9dee_0a11_e3ac, 0x09d0_de0b_155f_eaea),
    (257, 3, 40, 1, 0x1b7c_7c6a_d28f_f5d0, 0x1b22_fe93_4e5c_7f0e),
    (257, 3, 40, 2, 0x6fa8_a16b_6d66_b0e1, 0xf791_7914_052e_9fae),
    (257, 5, 3, 1, 0xf236_d31c_ecc1_e8bd, 0xb6df_0efa_aa9a_88a2),
    (257, 5, 3, 2, 0x0575_f9ac_8f5f_15fb, 0xb3f8_5854_0559_8eca),
    (257, 5, 12, 1, 0x68af_340c_be7c_c17a, 0xdec3_a8f3_c67b_e07a),
    (257, 5, 12, 2, 0x1615_1d9b_789d_9698, 0x222f_ca5a_62e2_71e2),
    (257, 5, 40, 1, 0x1c38_eff0_4dff_4659, 0xbf27_62a5_5335_fb8a),
    (257, 5, 40, 2, 0x532c_755f_0d09_0bbe, 0xcdca_d012_32fe_1ae2),
    (1000, 2, 3, 1, 0x6a00_8016_67d6_76a7, 0x7bf4_1550_917f_dbb9),
    (1000, 2, 3, 2, 0xaafa_4141_8da6_f076, 0x16f4_7bde_2bd9_19f1),
    (1000, 2, 12, 1, 0x3947_ae3a_e188_d4d4, 0x36b1_a586_2bc1_1e05),
    (1000, 2, 12, 2, 0x1741_5e91_542c_55c9, 0xb5a3_f7c6_11b4_a431),
    (1000, 2, 40, 1, 0x73d8_9728_41f3_82bc, 0xeefe_4456_c018_0a55),
    (1000, 2, 40, 2, 0xd1c4_2729_223c_60a1, 0x5b0a_c052_a44f_9e49),
    (1000, 3, 3, 1, 0x48ac_3d3e_be15_8d44, 0xcbe6_0183_9690_5511),
    (1000, 3, 3, 2, 0x08f8_9b49_83b8_d0d1, 0xa6d1_57f9_10ed_2ae5),
    (1000, 3, 12, 1, 0xdbdd_55ec_8149_e06f, 0x923e_9278_c440_70a5),
    (1000, 3, 12, 2, 0xef8b_1640_cab1_25f7, 0xe154_1f45_2e6e_0739),
    (1000, 3, 40, 1, 0x7254_dc96_4ff4_cae8, 0x00c1_5318_955b_048d),
    (1000, 3, 40, 2, 0x9af5_6fde_859c_4762, 0x2cdf_5b44_e2e4_b5b5),
    (1000, 5, 3, 1, 0xc983_1e24_c6d4_66d5, 0x9978_45a2_c457_8a05),
    (1000, 5, 3, 2, 0x2cb3_ab60_bceb_341b, 0x4ebb_7366_e184_2325),
    (1000, 5, 12, 1, 0x4c2a_b6e6_c25d_1f09, 0x6431_8fbd_9465_b0e5),
    (1000, 5, 12, 2, 0x76a3_5ea6_417e_1cee, 0xaefc_e743_9102_c591),
    (1000, 5, 40, 1, 0xb3cd_d6a5_d3c3_a5c2, 0x7e25_a96e_beab_a4a5),
    (1000, 5, 40, 2, 0x1e12_edfb_9b1a_a2f0, 0x107c_88f0_43f5_1ea9),
];

/// Each case of a comparison that answers at random, true once in `odds` calls: the number
/// of items, the seed, the odds, and the digests of the compared pairs and of the order.
const RANDOM: [(usize, u64, u64, u64, u64); 24] = [
    (100, 1, 2, 0x3c2d_f18b_280e_a6af, 0x873c_dae6_6dae_1ae5),
    (100, 2, 2, 0x330e_b9aa_0c93_3108, 0x50c5_e80c_16bc_3b45),
    (100, 1, 3, 0x469c_5a4e_38b2_6de9, 0xa75b_b8fd_19bb_6c05),
    (100, 2, 3, 0x91b0_bde3_cf54_f636, 0x2f71_a7f8_e865_f805),
    (100, 1, 7, 0xc478_cfd1_6a74_1c7c, 0x011c_a2c2_aa63_cd85),
    (100, 2, 7, 0xc1cf_e407_67d7_1020, 0x397c_08f0_a7ba_0dc5),
    (257, 1, 2, 0x5aad_3dd4_9c0d_1ec4, 0x8dfe_f8bb_43cb_5822),
    (257, 2, 2, 0xb3c6_18bf_dd4b_59f4, 0x21ad_609a_4674_8eaa),
    (257, 1, 3, 0x3b0d_1a9f_359b_b8f2, 0xd7e9_303a_4be4_6ba2),
    (257, 2, 3, 0xedb2_81ea_77eb_6886, 0xa8d4_5bbe_e2ae_93ea),
    (257, 1, 7, 0x9226_f28a_5732_f51b, 0x49fc_6935_0d01_8a0a),
    (257, 2, 7, 0x2f68_59cd_7bcc_7d71, 0x7f4c_ef9d_e0b9_6c2a),
    (1000, 1, 2, 0xac4b_1cc9_a00d_e351, 0x8ed5_7146_654e_64b9),
    (1000, 2, 2, 0x8e9e_d83a_c8f2_194a, 0xb19b_9dc7_aa99_14a5),
    (1000, 1, 3, 0x289d_d68c_e987_07b3, 0x9cd9_65d4_cc26_f1f5),
    (1000, 2, 3, 0x8eba_7062_12ae_39f6, 0xa389_ffbe_3783_12b9),
    (1000, 1, 7, 0x497b_5e94_33a9_7ad7, 0xf3b2_6e21_30e1_7b05),
    (1000, 2, 7, 0x8143_4058_4b63_7dc9, 0xbd6f_cbf1_a8ff_3e75),
    (3000, 1, 2, 0x0ac6_dffe_62e2_1c9d, 0x5d82_1e73_e940_7211),
    (3000, 2, 2, 0xd218_fe9e_8e11_22d0, 0x658b_5ec0_c7b5_2e95),
    (3000, 1, 3, 0xeb8e_5c63_8017_6cad, 0x37a3_1407_0cee_c5c1),
    (3000, 2, 3, 0xdf78_7cee_32bd_48ba, 0x24e6_295a_08e2_79fd),
    (3000, 1, 7, 0x81a8_adad_4620_29cd, 0x4d97_d498_69ed_ae3d),
    (3000, 2, 7, 0xbee6_6418_2601_38b4, 0x7394_8db8_8a0f_3eb5),
];

/// Each case of interleaved runs whose comparison raises: the number of items, of runs, the
/// longest block and the seed, as in `INTERLEAVED`, the number of comparisons the sort makes,
/// and the digest of the orders it leaves when the comparison after each number of them
/// raises, one after another.
const RAISING: [(usize, usize, u64, u64, usize, u64); 16] = [
    (100, 2, 12, 1, 254, 0x46b6_a7a7_1d8c_3105),
    (100, 2, 12, 2, 191, 0xf783_2213_017d_a2e5),
    (100, 2, 40, 1, 132, 0x660d_4640_38b0_c025),
    (100, 2, 40, 2, 149, 0xdcc5_5fbd_7427_9765),
    (100, 3, 12, 1, 398, 0x1ff3_147e_acf7_8005),
    (100, 3, 12, 2, 366, 0x010b_97da_a3d9_4ee5),
    (100, 3, 40, 1, 221, 0x128a_171f_8ec2_fa25),
    (100, 3, 40, 2, 287, 0x1d4d_5030_a506_4745),
    (257, 2, 12, 1, 500, 0x491e_3ff6_18d1_9a05),
    (257, 2, 12, 2, 488, 0x9b5e_e8b8_1613_eee5),
    (257, 2, 40, 1, 351, 0x12d0_bfd9_8c5f_b9ea),
    (257, 2, 40, 2, 367, 0x0303_5dca_28d4_f052),
    (257, 3, 12, 1, 591, 0xa066_c8c3_057a_c832),
    (257, 3, 12, 2, 597, 0x9abf_88a5_832f_4f56),
    (257, 3, 40, 1, 389, 0x9d20_1d49_4a67_4c22),
    (257, 3, 40, 2, 402, 0xbff5_e178_6b1e_3565),
];

/// The natural runs of a list whose last merges join the run below the top two first, since
/// it is shorter than the top one, with the digests of the compared pairs and of the order.
const RUNS: [(&[usize], u64, u64); 1] = [(
    &[135, 2, 86, 34],
    0x0366_dda3_d95a_1dcf,
    0x5b37_b6f3_6dbf_ef4e,
)];

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

/// Returns ascending runs, one after another, whose values interleave: each takes the next
/// block of 1 to `longest` values in turn.
#[allow(clippy::cast_possible_truncation, reason = "a block is short")]
fn interleaved(size: usize, runs: usize, longest: u64, seed: u64) -> Vec<i64> {
    let mut state = seed;
    let mut parts = vec![Vec::new(); runs];
    let (mut value, mut turn) = (0, 0);
    while value < size {
        state = next(state);
        let block = 1 + ((state >> 33) % longest) as usize;
        for _ in 0..block.min(size - value) {
            parts[turn].push(i64::try_from(value).unwrap());
            value += 1;
        }
        turn = (turn + 1) % runs;
    }
    parts.concat()
}

/// Returns ascending runs of these lengths, each starting below where the one before ends.
#[allow(clippy::cast_possible_wrap, reason = "every length is small")]
fn runs_of(lengths: &[usize]) -> Vec<i64> {
    let mut base = 10 * lengths.iter().sum::<usize>() as i64;
    let mut values = Vec::new();
    for length in lengths {
        let length = *length as i64;
        base -= 7 * length / 2 + 3;
        values.extend((0..length).map(|index| base + 5 * index));
    }
    values
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

/// Sorts the positions of `values` by them, and returns the digests of the compared pairs and
/// of the sorted order.
fn digests(values: &[i64]) -> (u64, u64) {
    let size = values.len();
    let mut items: Vec<usize> = (0..size).collect();
    let mut scratch = vec![0; size / 2 + 1];
    let log = RefCell::new(Vec::new());
    let less = |left: usize, right: usize| -> Result<bool, Raised> {
        log.borrow_mut().extend([left as u64, right as u64]);
        Ok(values[left] < values[right])
    };
    sort(&mut items, &mut scratch, &less).unwrap();
    let order: Vec<u64> = items.iter().map(|index| *index as u64).collect();
    (fnv(&log.borrow()), fnv(&order))
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

#[test]
fn fewer_than_two_items_are_left_as_they_are_without_a_comparison() {
    for size in [0, 1] {
        let mut items: Vec<usize> = (0..size).collect();
        let less = |_: usize, _: usize| -> Result<bool, Raised> { Err(Raised) };
        assert_eq!(sort(&mut items, &mut [], &less), Ok(()));
        assert_eq!(items, (0..size).collect::<Vec<_>>());
    }
}

#[test]
fn interleaved_runs_merge_with_the_comparisons_cpython_3_15_makes() {
    for (size, runs, longest, seed, compared, sorted) in INTERLEAVED {
        let found = digests(&interleaved(size, runs, longest, seed));
        assert_eq!(found, (compared, sorted), "{size} {runs} {longest} {seed}");
    }
}

#[test]
fn the_last_merges_take_the_shorter_neighbour_of_the_top_run_first() {
    for (lengths, compared, sorted) in RUNS {
        assert_eq!(
            digests(&runs_of(lengths)),
            (compared, sorted),
            "{lengths:?}"
        );
    }
}

#[test]
fn a_comparison_that_answers_at_random_makes_the_calls_cpython_3_15_makes() {
    for (size, seed, odds, compared, sorted) in RANDOM {
        let mut items: Vec<usize> = (0..size).collect();
        let mut scratch = vec![0; size / 2 + 1];
        let log = RefCell::new(Vec::new());
        let state = core::cell::Cell::new(seed);
        let less = |left: usize, right: usize| -> Result<bool, Raised> {
            log.borrow_mut().extend([left as u64, right as u64]);
            state.set(next(state.get()));
            Ok((state.get() >> 33) % odds == 0)
        };
        sort(&mut items, &mut scratch, &less).unwrap();
        let order: Vec<u64> = items.iter().map(|index| *index as u64).collect();
        assert_eq!(
            fnv(&log.borrow()),
            compared,
            "pairs of {size} {seed} {odds}"
        );
        assert_eq!(fnv(&order), sorted, "order of {size} {seed} {odds}");
    }
}

#[test]
fn a_comparison_that_raises_leaves_the_items_in_the_order_cpython_3_15_leaves() {
    for (size, runs, longest, seed, total, digest) in RAISING {
        let values = interleaved(size, runs, longest, seed);
        let mut orders = Vec::new();
        for limit in 0..=total {
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
            // Only a limit of every comparison the sort makes lets it finish.
            assert_eq!(
                result.is_ok(),
                limit == total,
                "{size} {runs} {longest} {seed}"
            );
            if limit < total {
                orders.extend(items.iter().map(|index| *index as u64));
            }
        }
        assert_eq!(fnv(&orders), digest, "{size} {runs} {longest} {seed}");
    }
}
