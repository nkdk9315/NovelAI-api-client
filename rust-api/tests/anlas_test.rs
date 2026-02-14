//! NovelAI Anlas Cost Calculation Tests
//! Ported from ts-api/tests/anlas.test.ts
//!
//! Validates cost calculation logic based on the NovelAI official frontend.

use novelai_api::anlas::{
    calc_char_ref_cost, calc_inpaint_size_correction, calc_v4_base_cost, calc_vibe_batch_cost,
    calculate_augment_cost, calculate_generation_cost, calculate_upscale_cost,
    clamp_to_max_pixels, expand_to_min_pixels, get_smea_multiplier, AugmentCostParams,
    AugmentToolType, GenerationCostParams, GenerationMode, SmeaMode, UpscaleCostParams,
};

// =============================================================================
// Category A: calc_v4_base_cost(width, height, steps)
// Formula: ceil(2.951823174884865e-6 * W*H + 5.753298233447344e-7 * W*H * steps)
// =============================================================================
mod calc_v4_base_cost_tests {
    use super::*;

    #[test]
    fn a1_832x1216_23_steps() {
        assert_eq!(calc_v4_base_cost(832, 1216, 23), 17);
    }

    #[test]
    fn a2_1024x1024_28_steps() {
        assert_eq!(calc_v4_base_cost(1024, 1024, 28), 20);
    }

    #[test]
    fn a3_2048x1536_50_steps() {
        assert_eq!(calc_v4_base_cost(2048, 1536, 50), 100);
    }

    #[test]
    fn a4_64x64_1_step_smallest_possible() {
        // ceil(2.951823174884865e-6*4096 + 5.753298233447344e-7*4096*1) = ceil(0.01444) = 1
        assert_eq!(calc_v4_base_cost(64, 64, 1), 1);
    }

    #[test]
    fn a5_832x1216_1_step() {
        // ceil(2.951823174884865e-6*1011712 + 5.753298233447344e-7*1011712*1) = ceil(3.5689) = 4
        assert_eq!(calc_v4_base_cost(832, 1216, 1), 4);
    }

    #[test]
    fn a6_832x1216_50_steps() {
        // ceil(2.951823174884865e-6*1011712 + 5.753298233447344e-7*1011712*50) = ceil(32.089) = 33
        assert_eq!(calc_v4_base_cost(832, 1216, 50), 33);
    }

    #[test]
    fn a7_1024x1024_1_step() {
        // ceil(2.951823174884865e-6*1048576 + 5.753298233447344e-7*1048576*1) = ceil(3.6978) = 4
        assert_eq!(calc_v4_base_cost(1024, 1024, 1), 4);
    }
}

// =============================================================================
// Category B: get_smea_multiplier(mode)
// =============================================================================
mod get_smea_multiplier_tests {
    use super::*;

    #[test]
    fn b1_off_returns_1_0() {
        assert_eq!(get_smea_multiplier(SmeaMode::Off), 1.0);
    }

    #[test]
    fn b2_smea_returns_1_2() {
        assert_eq!(get_smea_multiplier(SmeaMode::Smea), 1.2);
    }

    #[test]
    fn b3_smea_dyn_returns_1_4() {
        assert_eq!(get_smea_multiplier(SmeaMode::SmeaDyn), 1.4);
    }
}

// =============================================================================
// Category C: is_opus_free_generation(width, height, steps, char_ref_count, tier, vibe_count)
// =============================================================================
mod is_opus_free_generation_tests {
    use novelai_api::anlas::is_opus_free_generation;

    #[test]
    fn c1_832x1216_23_steps_0_charref_tier3_true() {
        assert!(is_opus_free_generation(832, 1216, 23, 0, 3, 0));
    }

    #[test]
    fn c2_1024x1024_28_steps_0_charref_tier3_true_exact_pixel_boundary() {
        assert!(is_opus_free_generation(1024, 1024, 28, 0, 3, 0));
    }

    #[test]
    fn c3_1088x1024_28_steps_0_charref_tier3_false_pixels_exceed() {
        // 1088*1024 = 1114112 > 1048576
        assert!(!is_opus_free_generation(1088, 1024, 28, 0, 3, 0));
    }

    #[test]
    fn c4_1024x1024_29_steps_0_charref_tier3_false_steps_exceed() {
        assert!(!is_opus_free_generation(1024, 1024, 29, 0, 3, 0));
    }

    #[test]
    fn c5_1024x1024_28_steps_0_charref_tier2_false_tier_too_low() {
        assert!(!is_opus_free_generation(1024, 1024, 28, 0, 2, 0));
    }

    #[test]
    fn c6_1024x1024_28_steps_1_charref_tier3_false_has_charref() {
        assert!(!is_opus_free_generation(1024, 1024, 28, 1, 3, 0));
    }

    #[test]
    fn c7_1024x1024_28_steps_0_charref_tier0_false_free_tier() {
        assert!(!is_opus_free_generation(1024, 1024, 28, 0, 0, 0));
    }

    #[test]
    fn c8_1088x1024_29_steps_1_charref_tier2_false_multiple_conditions_fail() {
        assert!(!is_opus_free_generation(1088, 1024, 29, 1, 2, 0));
    }
}

// =============================================================================
// Category D: Per-image cost with strength/SMEA (via calculate_generation_cost adjusted_cost)
// All using tier=0 to avoid Opus free, n_samples=1
// =============================================================================
mod per_image_cost_tests {
    use super::*;

    #[test]
    fn d1_base_case_adjusted_cost_17() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            tier: 0,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.adjusted_cost, 17);
    }

    #[test]
    fn d2_smea_adjusted_cost_21() {
        // ceil(17*1.2) = 21
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            smea: SmeaMode::Smea,
            tier: 0,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.adjusted_cost, 21);
    }

    #[test]
    fn d3_smea_dyn_adjusted_cost_24() {
        // ceil(17*1.4) = 24
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            smea: SmeaMode::SmeaDyn,
            tier: 0,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.adjusted_cost, 24);
    }

    #[test]
    fn d4_img2img_strength_0_62_adjusted_cost_11() {
        // max(ceil(17*0.62), 2) = max(ceil(10.54), 2) = 11
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            mode: GenerationMode::Img2Img,
            strength: 0.62,
            tier: 0,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.adjusted_cost, 11);
    }

    #[test]
    fn d5_img2img_strength_0_01_adjusted_cost_2_min_cost() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            mode: GenerationMode::Img2Img,
            strength: 0.01,
            tier: 0,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.adjusted_cost, 2);
    }

    #[test]
    fn d6_img2img_strength_1_0_adjusted_cost_17() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            mode: GenerationMode::Img2Img,
            strength: 1.0,
            tier: 0,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.adjusted_cost, 17);
    }

    #[test]
    fn d7_smea_img2img_strength_0_62_adjusted_cost_13() {
        // baseCost=17, smea=1.2, perImageCost=17*1.2=20.4 (not ceiled)
        // adjustedCost = max(ceil(20.4*0.62), 2) = max(ceil(12.648), 2) = 13
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            smea: SmeaMode::Smea,
            mode: GenerationMode::Img2Img,
            strength: 0.62,
            tier: 0,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.adjusted_cost, 13);
    }

    #[test]
    fn d8_2048x1536_50_steps_smea_dyn_adjusted_cost_140_no_error() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 2048,
            height: 1536,
            steps: 50,
            smea: SmeaMode::SmeaDyn,
            tier: 0,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.adjusted_cost, 140);
        assert!(!result.error);
    }

    #[test]
    fn d9_max_case_2048x1536_50_steps_smea_dyn_adjusted_cost_140_no_error() {
        // baseCost=100, smea_dyn -> 100*1.4=140 exactly at MAX_COST, not error
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 2048,
            height: 1536,
            steps: 50,
            smea: SmeaMode::SmeaDyn,
            tier: 0,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.adjusted_cost, 140);
        assert!(!result.error);
    }

    #[test]
    fn d10_img2img_strength_0_12_adjusted_cost_3() {
        // max(ceil(17*0.12), 2) = max(ceil(2.04), 2) = max(3, 2) = 3
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            mode: GenerationMode::Img2Img,
            strength: 0.12,
            tier: 0,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.adjusted_cost, 3);
    }
}

// =============================================================================
// Category E: Billable images and Opus discount (via calculate_generation_cost)
// =============================================================================
mod billable_images_and_opus_discount_tests {
    use super::*;

    #[test]
    fn e1_opus_tier_1_sample_opus_free_true_billable_0_generation_cost_0() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            tier: 3,
            n_samples: 1,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert!(result.is_opus_free);
        assert_eq!(result.billable_images, 0);
        assert_eq!(result.generation_cost, 0);
    }

    #[test]
    fn e2_opus_tier_2_samples_opus_free_true_billable_1_generation_cost_17() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            tier: 3,
            n_samples: 2,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert!(result.is_opus_free);
        assert_eq!(result.billable_images, 1);
        assert_eq!(result.generation_cost, 17);
    }

    #[test]
    fn e3_opus_tier_4_samples_opus_free_true_billable_3_generation_cost_51() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            tier: 3,
            n_samples: 4,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert!(result.is_opus_free);
        assert_eq!(result.billable_images, 3);
        assert_eq!(result.generation_cost, 51);
    }

    #[test]
    fn e4_non_opus_1_sample_opus_free_false_billable_1_generation_cost_17() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            tier: 0,
            n_samples: 1,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert!(!result.is_opus_free);
        assert_eq!(result.billable_images, 1);
        assert_eq!(result.generation_cost, 17);
    }

    #[test]
    fn e5_non_opus_4_samples_opus_free_false_billable_4_generation_cost_68() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            tier: 0,
            n_samples: 4,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert!(!result.is_opus_free);
        assert_eq!(result.billable_images, 4);
        assert_eq!(result.generation_cost, 68);
    }
}

// =============================================================================
// Category F: Vibe costs
// =============================================================================
mod vibe_costs_tests {
    use super::*;

    // --- calc_vibe_batch_cost ---

    #[test]
    fn f1_0_vibes_returns_0() {
        assert_eq!(calc_vibe_batch_cost(0), 0);
    }

    #[test]
    fn f2_1_vibe_returns_0() {
        assert_eq!(calc_vibe_batch_cost(1), 0);
    }

    #[test]
    fn f3_4_vibes_returns_0_at_threshold() {
        assert_eq!(calc_vibe_batch_cost(4), 0);
    }

    #[test]
    fn f4_5_vibes_returns_2() {
        assert_eq!(calc_vibe_batch_cost(5), 2);
    }

    #[test]
    fn f5_6_vibes_returns_4() {
        assert_eq!(calc_vibe_batch_cost(6), 4);
    }

    #[test]
    fn f6_10_vibes_returns_12() {
        assert_eq!(calc_vibe_batch_cost(10), 12);
    }

    // --- Vibe costs in calculate_generation_cost ---

    #[test]
    fn f7_3_vibes_all_encoded_opus_vibe_encode_cost_0_vibe_batch_cost_0() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            vibe_count: 3,
            vibe_unencoded_count: 0,
            tier: 3,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.vibe_encode_cost, 0);
        assert_eq!(result.vibe_batch_cost, 0);
    }

    #[test]
    fn f8_6_vibes_2_unencoded_opus_vibes_disable_opus_free() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            vibe_count: 6,
            vibe_unencoded_count: 2,
            tier: 3,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.vibe_encode_cost, 4);
        assert_eq!(result.vibe_batch_cost, 4);
        assert!(!result.is_opus_free);
        assert_eq!(result.generation_cost, 17);
        assert_eq!(result.total_cost, 25);
    }

    #[test]
    fn f9_5_vibes_charref_disables_vibe() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            vibe_count: 5,
            char_ref_count: 1,
            tier: 0,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.vibe_batch_cost, 0);
        assert_eq!(result.vibe_encode_cost, 0);
    }

    #[test]
    fn f10_5_vibes_inpaint_disables_vibe() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            vibe_count: 5,
            mode: GenerationMode::Inpaint,
            tier: 0,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.vibe_batch_cost, 0);
        assert_eq!(result.vibe_encode_cost, 0);
    }
}

// =============================================================================
// Category G: Character reference cost
// =============================================================================
mod calc_char_ref_cost_tests {
    use super::*;

    #[test]
    fn g1_0_charrefs_1_sample_returns_0() {
        assert_eq!(calc_char_ref_cost(0, 1), 0);
    }

    #[test]
    fn g2_1_charref_1_sample_returns_5() {
        assert_eq!(calc_char_ref_cost(1, 1), 5);
    }

    #[test]
    fn g3_2_charrefs_1_sample_returns_10() {
        assert_eq!(calc_char_ref_cost(2, 1), 10);
    }

    #[test]
    fn g4_1_charref_4_samples_returns_20() {
        assert_eq!(calc_char_ref_cost(1, 4), 20);
    }

    #[test]
    fn g5_6_charrefs_4_samples_returns_120() {
        assert_eq!(calc_char_ref_cost(6, 4), 120);
    }
}

// =============================================================================
// Category H: Inpaint size correction
// =============================================================================
mod calc_inpaint_size_correction_tests {
    use super::*;

    #[test]
    fn h1_1024x1024_not_corrected() {
        // maskPixels=1048576, threshold=0.8*1048576=838861
        let result = calc_inpaint_size_correction(1024, 1024);
        assert!(!result.corrected);
        assert_eq!(result.width, 1024);
        assert_eq!(result.height, 1024);
    }

    #[test]
    fn h2_928x928_not_corrected() {
        // 861184 >= 838861
        let result = calc_inpaint_size_correction(928, 928);
        assert!(!result.corrected);
        assert_eq!(result.width, 928);
        assert_eq!(result.height, 928);
    }

    #[test]
    fn h3_512x512_corrected_to_1024x1024() {
        // scale = sqrt(1048576/262144) = 2.0
        // width = floor(floor(512*2.0)/64)*64 = 1024
        let result = calc_inpaint_size_correction(512, 512);
        assert!(result.corrected);
        assert_eq!(result.width, 1024);
        assert_eq!(result.height, 1024);
    }

    #[test]
    fn h4_256x256_corrected_to_1024x1024() {
        // scale = sqrt(1048576/65536) = 4.0
        let result = calc_inpaint_size_correction(256, 256);
        assert!(result.corrected);
        assert_eq!(result.width, 1024);
        assert_eq!(result.height, 1024);
    }

    #[test]
    fn h5_300x400_corrected_to_832x1152() {
        // scale = sqrt(1048576/120000) ~ 2.9559
        // width = floor(floor(300*2.9559)/64)*64 = floor(886/64)*64 = 13*64 = 832
        // height = floor(floor(400*2.9559)/64)*64 = floor(1182/64)*64 = 18*64 = 1152
        let result = calc_inpaint_size_correction(300, 400);
        assert!(result.corrected);
        assert_eq!(result.width, 832);
        assert_eq!(result.height, 1152);
    }
}

// =============================================================================
// Category I: Full integration tests (calculate_generation_cost)
// =============================================================================
mod calculate_generation_cost_integration_tests {
    use super::*;

    #[test]
    fn i1_opus_free_total_cost_0() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            tier: 3,
            n_samples: 1,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.total_cost, 0);
    }

    #[test]
    fn i2_non_opus_total_cost_17() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            tier: 0,
            n_samples: 1,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.total_cost, 17);
    }

    #[test]
    fn i3_1024x1024_opus_total_cost_0() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 1024,
            height: 1024,
            steps: 28,
            tier: 3,
            n_samples: 1,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.total_cost, 0);
    }

    #[test]
    fn i4_2048x1536_50_steps_total_cost_100() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 2048,
            height: 1536,
            steps: 50,
            tier: 0,
            n_samples: 1,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.total_cost, 100);
    }

    #[test]
    fn i5_opus_2_samples_total_cost_17() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            tier: 3,
            n_samples: 2,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.total_cost, 17);
    }

    #[test]
    fn i6_smea_total_cost_21() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            smea: SmeaMode::Smea,
            tier: 0,
            n_samples: 1,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.total_cost, 21);
    }

    #[test]
    fn i7_smea_dyn_total_cost_24() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            smea: SmeaMode::SmeaDyn,
            tier: 0,
            n_samples: 1,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.total_cost, 24);
    }

    #[test]
    fn i8_img2img_strength_0_62_total_cost_11() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            mode: GenerationMode::Img2Img,
            strength: 0.62,
            tier: 0,
            n_samples: 1,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.total_cost, 11);
    }

    #[test]
    fn i9_1024x1024_28_steps_non_opus_total_cost_20() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 1024,
            height: 1024,
            steps: 28,
            tier: 0,
            n_samples: 1,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.total_cost, 20);
    }

    #[test]
    fn i10_charref_disables_opus_free() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            tier: 3,
            n_samples: 1,
            char_ref_count: 2,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.char_ref_cost, 10);
        assert_eq!(result.generation_cost, 17);
        assert_eq!(result.total_cost, 27);
    }

    #[test]
    fn i11_opus_vibes_disable_opus_free() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            tier: 3,
            n_samples: 1,
            vibe_count: 6,
            vibe_unencoded_count: 2,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert!(!result.is_opus_free);
        assert_eq!(result.generation_cost, 17);
        assert_eq!(result.vibe_encode_cost, 4);
        assert_eq!(result.vibe_batch_cost, 4);
        assert_eq!(result.total_cost, 25);
    }

    #[test]
    fn i12_opus_2_samples_vibes() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            tier: 3,
            n_samples: 2,
            vibe_count: 5,
            vibe_unencoded_count: 1,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert!(!result.is_opus_free);
        assert_eq!(result.generation_cost, 34);
        assert_eq!(result.vibe_encode_cost, 2);
        assert_eq!(result.vibe_batch_cost, 2);
        assert_eq!(result.total_cost, 38);
    }

    #[test]
    fn i13_error_false_for_max_configuration() {
        // baseCost=100, smea_dyn*1.4=140 exactly -> at MAX_COST, not exceeding
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 2048,
            height: 1536,
            steps: 50,
            smea: SmeaMode::SmeaDyn,
            tier: 0,
            n_samples: 1,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert!(!result.error);
        assert_eq!(result.adjusted_cost, 140);
    }
}

// =============================================================================
// Category J: calculate_augment_cost
// =============================================================================
mod calculate_augment_cost_tests {
    use super::*;

    #[test]
    fn j1_lineart_512x512_tier3_expand_to_1024x1024() {
        let result = calculate_augment_cost(&AugmentCostParams {
            tool: AugmentToolType::Lineart,
            width: 512,
            height: 512,
            tier: 3,
        })
        .unwrap();
        assert_eq!(result.adjusted_width, 1024);
        assert_eq!(result.adjusted_height, 1024);
        assert_eq!(result.adjusted_pixels, 1048576);
        assert_eq!(result.base_cost, 20);
        assert_eq!(result.final_cost, 20);
        assert!(result.is_opus_free);
        assert_eq!(result.effective_cost, 0);
    }

    #[test]
    fn j2_bg_removal_512x512_tier3_always_charged() {
        let result = calculate_augment_cost(&AugmentCostParams {
            tool: AugmentToolType::BgRemoval,
            width: 512,
            height: 512,
            tier: 3,
        })
        .unwrap();
        assert_eq!(result.base_cost, 20);
        assert_eq!(result.final_cost, 65);
        assert!(!result.is_opus_free);
        assert_eq!(result.effective_cost, 65);
    }

    #[test]
    fn j3_lineart_1024x1024_tier3_no_expansion() {
        let result = calculate_augment_cost(&AugmentCostParams {
            tool: AugmentToolType::Lineart,
            width: 1024,
            height: 1024,
            tier: 3,
        })
        .unwrap();
        assert_eq!(result.adjusted_width, 1024);
        assert_eq!(result.adjusted_height, 1024);
        assert_eq!(result.base_cost, 20);
        assert_eq!(result.final_cost, 20);
        assert!(result.is_opus_free);
        assert_eq!(result.effective_cost, 0);
    }

    #[test]
    fn j4_colorize_2048x1536_tier0() {
        let result = calculate_augment_cost(&AugmentCostParams {
            tool: AugmentToolType::Colorize,
            width: 2048,
            height: 1536,
            tier: 0,
        })
        .unwrap();
        assert_eq!(result.original_pixels, 3145728);
        assert_eq!(result.base_cost, 60);
        assert_eq!(result.final_cost, 60);
        assert!(!result.is_opus_free);
        assert_eq!(result.effective_cost, 60);
    }

    #[test]
    fn j5_bg_removal_1024x1024_tier0() {
        let result = calculate_augment_cost(&AugmentCostParams {
            tool: AugmentToolType::BgRemoval,
            width: 1024,
            height: 1024,
            tier: 0,
        })
        .unwrap();
        assert_eq!(result.base_cost, 20);
        assert_eq!(result.final_cost, 65);
        assert!(!result.is_opus_free);
        assert_eq!(result.effective_cost, 65);
    }

    #[test]
    fn j6_sketch_1200x900_tier0() {
        let result = calculate_augment_cost(&AugmentCostParams {
            tool: AugmentToolType::Sketch,
            width: 1200,
            height: 900,
            tier: 0,
        })
        .unwrap();
        assert_eq!(result.original_pixels, 1080000);
        assert_eq!(result.base_cost, 21);
        assert_eq!(result.final_cost, 21);
        assert!(!result.is_opus_free);
        assert_eq!(result.effective_cost, 21);
    }

    #[test]
    fn j7_all_6_tool_types_1024x1024_tier0() {
        let tools = [
            AugmentToolType::Colorize,
            AugmentToolType::Declutter,
            AugmentToolType::Emotion,
            AugmentToolType::Sketch,
            AugmentToolType::Lineart,
            AugmentToolType::BgRemoval,
        ];
        for tool in &tools {
            let result = calculate_augment_cost(&AugmentCostParams {
                tool: *tool,
                width: 1024,
                height: 1024,
                tier: 0,
            })
            .unwrap();
            assert_eq!(result.base_cost, 20);
            if *tool == AugmentToolType::BgRemoval {
                assert_eq!(result.final_cost, 65);
            } else {
                assert_eq!(result.final_cost, 20);
            }
        }
    }

    #[test]
    fn j8_emotion_100x100_tier0_expand_with_floor_no_grid_snap() {
        // scale=sqrt(1048576/10000)=10.24 exactly
        // w=floor(100*10.24)=floor(1024)=1024, h=1024
        // adjustedPixels=1024*1024=1048576
        let result = calculate_augment_cost(&AugmentCostParams {
            tool: AugmentToolType::Emotion,
            width: 100,
            height: 100,
            tier: 0,
        })
        .unwrap();
        assert_eq!(result.adjusted_width, 1024);
        assert_eq!(result.adjusted_height, 1024);
        assert_eq!(result.adjusted_pixels, 1024 * 1024);
        assert_eq!(result.base_cost, 20);
    }
}

// =============================================================================
// Category K: calculate_upscale_cost
// =============================================================================
mod calculate_upscale_cost_tests {
    use super::*;

    #[test]
    fn k1_512x512_tier0() {
        let result = calculate_upscale_cost(&UpscaleCostParams {
            width: 512,
            height: 512,
            tier: 0,
        })
        .unwrap();
        assert_eq!(result.pixels, 262144);
        assert_eq!(result.cost, Some(1));
        assert!(!result.is_opus_free);
        assert!(!result.error);
    }

    #[test]
    fn k2_640x640_tier0() {
        let result = calculate_upscale_cost(&UpscaleCostParams {
            width: 640,
            height: 640,
            tier: 0,
        })
        .unwrap();
        assert_eq!(result.pixels, 409600);
        assert_eq!(result.cost, Some(2));
        assert!(!result.is_opus_free);
        assert!(!result.error);
    }

    #[test]
    fn k3_512x1024_tier0() {
        let result = calculate_upscale_cost(&UpscaleCostParams {
            width: 512,
            height: 1024,
            tier: 0,
        })
        .unwrap();
        assert_eq!(result.pixels, 524288);
        assert_eq!(result.cost, Some(3));
        assert!(!result.error);
    }

    #[test]
    fn k4_1024x768_tier0() {
        let result = calculate_upscale_cost(&UpscaleCostParams {
            width: 1024,
            height: 768,
            tier: 0,
        })
        .unwrap();
        assert_eq!(result.pixels, 786432);
        assert_eq!(result.cost, Some(5));
        assert!(!result.error);
    }

    #[test]
    fn k5_1024x1024_tier0() {
        let result = calculate_upscale_cost(&UpscaleCostParams {
            width: 1024,
            height: 1024,
            tier: 0,
        })
        .unwrap();
        assert_eq!(result.pixels, 1048576);
        assert_eq!(result.cost, Some(7));
        assert!(!result.error);
    }

    #[test]
    fn k6_1025x1024_tier0_error_pixels_exceed() {
        let result = calculate_upscale_cost(&UpscaleCostParams {
            width: 1025,
            height: 1024,
            tier: 0,
        })
        .unwrap();
        assert_eq!(result.pixels, 1049600);
        assert_eq!(result.cost, None);
        assert!(result.error);
        assert_eq!(result.error_code, Some(-3));
    }

    #[test]
    fn k7_512x512_tier3_opus_free() {
        let result = calculate_upscale_cost(&UpscaleCostParams {
            width: 512,
            height: 512,
            tier: 3,
        })
        .unwrap();
        assert_eq!(result.pixels, 262144);
        assert!(result.is_opus_free);
        assert_eq!(result.cost, Some(0));
    }

    #[test]
    fn k8_640x640_tier3_opus_free() {
        let result = calculate_upscale_cost(&UpscaleCostParams {
            width: 640,
            height: 640,
            tier: 3,
        })
        .unwrap();
        assert_eq!(result.pixels, 409600);
        assert!(result.is_opus_free);
        assert_eq!(result.cost, Some(0));
    }

    #[test]
    fn k9_512x1024_tier3_not_opus_free() {
        let result = calculate_upscale_cost(&UpscaleCostParams {
            width: 512,
            height: 1024,
            tier: 3,
        })
        .unwrap();
        assert_eq!(result.pixels, 524288);
        assert!(!result.is_opus_free);
        assert_eq!(result.cost, Some(3));
    }

    #[test]
    fn k10_exact_boundary_512x512_cost_1() {
        let result = calculate_upscale_cost(&UpscaleCostParams {
            width: 512,
            height: 512,
            tier: 0,
        })
        .unwrap();
        assert_eq!(result.pixels, 262144);
        assert_eq!(result.cost, Some(1));
    }
}

// =============================================================================
// Category L: Size helpers
// =============================================================================
mod size_helpers_tests {
    use super::*;

    // --- expand_to_min_pixels ---

    #[test]
    fn l1_1024x1024_min_1048576_no_change() {
        let result = expand_to_min_pixels(1024, 1024, 1048576);
        assert_eq!(result.width, 1024);
        assert_eq!(result.height, 1024);
    }

    #[test]
    fn l2_512x512_min_1048576_scale_2_to_1024x1024() {
        let result = expand_to_min_pixels(512, 512, 1048576);
        assert_eq!(result.width, 1024);
        assert_eq!(result.height, 1024);
    }

    #[test]
    fn l3_100x100_min_1048576_floor_no_grid_snap() {
        // scale=sqrt(1048576/10000)=10.24 exactly
        // width=floor(100*10.24)=floor(1024)=1024
        let result = expand_to_min_pixels(100, 100, 1048576);
        assert_eq!(result.width, 1024);
        assert_eq!(result.height, 1024);
    }

    // --- clamp_to_max_pixels ---

    #[test]
    fn l4_1024x1024_max_3145728_no_change() {
        let result = clamp_to_max_pixels(1024, 1024, 3145728);
        assert_eq!(result.width, 1024);
        assert_eq!(result.height, 1024);
    }

    #[test]
    fn l5_2048x2048_max_3145728_scale_down() {
        // 4194304 > 3145728, scale=sqrt(3145728/4194304)~0.86602
        // width=floor(2048*0.86602)=floor(1773.96)=1773
        let result = clamp_to_max_pixels(2048, 2048, 3145728);
        assert_eq!(result.width, 1773);
        assert_eq!(result.height, 1773);
    }

    #[test]
    fn l6_3000x2000_max_3145728_scale_down() {
        // 6000000 > 3145728, scale=sqrt(3145728/6000000)~0.72408
        // width=floor(3000*0.72408)=floor(2172.23)=2172
        // height=floor(2000*0.72408)=floor(1448.15)=1448
        let result = clamp_to_max_pixels(3000, 2000, 3145728);
        assert_eq!(result.width, 2172);
        assert_eq!(result.height, 1448);
    }
}

// =============================================================================
// Category M: Edge cases
// =============================================================================
mod edge_cases_tests {
    use super::*;

    #[test]
    fn m1_img2img_strength_0_adjusted_cost_2_min_cost() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            mode: GenerationMode::Img2Img,
            strength: 0.0,
            tier: 0,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.adjusted_cost, 2);
    }

    #[test]
    fn m2_n_samples_0_total_cost_0() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            tier: 0,
            n_samples: 0,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.total_cost, 0);
    }

    #[test]
    fn m3_default_params_total_cost_17() {
        // tier defaults to 0 via GenerationCostParams::default()
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert_eq!(result.total_cost, 17);
    }

    #[test]
    fn m4_result_object_has_all_expected_fields() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            ..GenerationCostParams::default()
        })
        .unwrap();
        // Verify all fields exist by accessing them (compile-time check in Rust)
        let _ = result.base_cost;
        let _ = result.adjusted_cost;
        let _ = result.is_opus_free;
        let _ = result.billable_images;
        let _ = result.generation_cost;
        let _ = result.vibe_encode_cost;
        let _ = result.vibe_batch_cost;
        let _ = result.char_ref_cost;
        let _ = result.total_cost;
        let _ = result.error;
    }

    #[test]
    fn m5_opus_vibes_disable_opus_free() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            tier: 3,
            n_samples: 1,
            vibe_count: 5,
            vibe_unencoded_count: 1,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert!(!result.is_opus_free);
        assert_eq!(result.generation_cost, 17);
        assert_eq!(result.vibe_encode_cost, 2);
        assert_eq!(result.vibe_batch_cost, 2);
        assert_eq!(result.total_cost, 21);
    }

    #[test]
    fn m6_charref_disables_opus_free_and_vibe_batch_cost_0() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            tier: 3,
            n_samples: 1,
            char_ref_count: 1,
            vibe_count: 5,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert!(!result.is_opus_free);
        assert_eq!(result.vibe_batch_cost, 0);
        assert_eq!(result.vibe_encode_cost, 0);
        assert_eq!(result.char_ref_cost, 5);
    }
}

// =============================================================================
// Category N: Zero division guard (calc_inpaint_size_correction)
// =============================================================================
mod zero_division_guard_tests {
    use super::*;

    #[test]
    fn n1_mask_width_0_mask_height_0_not_corrected() {
        let result = calc_inpaint_size_correction(0, 0);
        assert!(!result.corrected);
        assert_eq!(result.width, 0);
        assert_eq!(result.height, 0);
    }

    #[test]
    fn n2_mask_width_0_mask_height_100_not_corrected() {
        let result = calc_inpaint_size_correction(0, 100);
        assert!(!result.corrected);
        assert_eq!(result.width, 0);
        assert_eq!(result.height, 100);
    }

    #[test]
    fn n3_mask_width_100_mask_height_0_not_corrected() {
        let result = calc_inpaint_size_correction(100, 0);
        assert!(!result.corrected);
        assert_eq!(result.width, 100);
        assert_eq!(result.height, 0);
    }

    #[test]
    fn n4_negative_mask_width_not_corrected() {
        let result = calc_inpaint_size_correction(-10, 100);
        assert!(!result.corrected);
        assert_eq!(result.width, -10);
        assert_eq!(result.height, 100);
    }
}

// =============================================================================
// Category O: Input validation tests
// =============================================================================
mod input_validation_tests {
    use super::*;

    // --- calculate_generation_cost validation ---

    #[test]
    fn o1_negative_width_returns_err() {
        // Rust u32 cannot be negative, so we test with width=0 instead
        // (equivalent validation: must be positive)
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 0,
            height: 1216,
            steps: 23,
            ..GenerationCostParams::default()
        });
        assert!(result.is_err());
    }

    #[test]
    fn o2_zero_height_returns_err() {
        // NaN is not possible with u32, test zero height instead
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 0,
            steps: 23,
            ..GenerationCostParams::default()
        });
        assert!(result.is_err());
    }

    #[test]
    fn o3_zero_steps_returns_err() {
        // Infinity is not possible with u32, test zero steps instead
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 0,
            ..GenerationCostParams::default()
        });
        assert!(result.is_err());
    }

    #[test]
    fn o4_zero_width_returns_err() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 0,
            height: 1216,
            steps: 23,
            ..GenerationCostParams::default()
        });
        assert!(result.is_err());
    }

    #[test]
    fn o5_valid_width_returns_ok() {
        // In Rust, u32 is always an integer, so non-integer test is not applicable.
        // Instead, verify that a valid width succeeds.
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            ..GenerationCostParams::default()
        });
        assert!(result.is_ok());
    }

    #[test]
    fn o6_strength_out_of_range_returns_err() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            mode: GenerationMode::Img2Img,
            strength: 1.5,
            ..GenerationCostParams::default()
        });
        assert!(result.is_err());
    }

    #[test]
    fn o7_negative_strength_returns_err() {
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 832,
            height: 1216,
            steps: 23,
            mode: GenerationMode::Img2Img,
            strength: -0.1,
            ..GenerationCostParams::default()
        });
        assert!(result.is_err());
    }

    #[test]
    fn o8_zero_width_generation_returns_err() {
        // Rust u32 cannot be negative; test another zero-value validation
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 0,
            height: 1216,
            steps: 23,
            ..GenerationCostParams::default()
        });
        assert!(result.is_err());
    }

    #[test]
    fn o9_all_zero_dimensions_returns_err() {
        // NaN n_samples is not possible with u32; test all-zero dimensions
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 0,
            height: 0,
            steps: 0,
            ..GenerationCostParams::default()
        });
        assert!(result.is_err());
    }

    // --- calculate_augment_cost validation ---

    #[test]
    fn o10_augment_zero_width_returns_err() {
        // Negative is not possible with u32, test zero width
        let result = calculate_augment_cost(&AugmentCostParams {
            tool: AugmentToolType::Lineart,
            width: 0,
            height: 512,
            tier: 0,
        });
        assert!(result.is_err());
    }

    #[test]
    fn o11_augment_zero_height_returns_err() {
        // NaN is not possible with u32, test zero height
        let result = calculate_augment_cost(&AugmentCostParams {
            tool: AugmentToolType::Lineart,
            width: 512,
            height: 0,
            tier: 0,
        });
        assert!(result.is_err());
    }

    // --- calculate_upscale_cost validation ---

    #[test]
    fn o12_upscale_zero_width_returns_err() {
        // Negative is not possible with u32, test zero width
        let result = calculate_upscale_cost(&UpscaleCostParams {
            width: 0,
            height: 512,
            tier: 0,
        });
        assert!(result.is_err());
    }

    #[test]
    fn o13_upscale_zero_height_returns_err() {
        // Infinity is not possible with u32, test zero height
        let result = calculate_upscale_cost(&UpscaleCostParams {
            width: 512,
            height: 0,
            tier: 0,
        });
        assert!(result.is_err());
    }
}

// =============================================================================
// Category P: Error totalCost behavior
// =============================================================================
mod error_total_cost_tests {
    use super::*;

    #[test]
    fn p1_max_config_does_not_error() {
        // 2048x1536, 50 steps, smea_dyn, vibes, img2img strength=1.0
        // adjustedCost=140 is at MAX_COST_PER_IMAGE=140, so no error
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 2048,
            height: 1536,
            steps: 50,
            smea: SmeaMode::SmeaDyn,
            tier: 0,
            n_samples: 1,
            vibe_count: 6,
            vibe_unencoded_count: 2,
            mode: GenerationMode::Img2Img,
            strength: 1.0,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert!(!result.error);
    }

    #[test]
    fn p2_adjusted_cost_exceeds_max_error_true_total_cost_0() {
        // calcV4BaseCost(2048, 2048, 50) = ceil(2.951823174884865e-6*4194304 + 5.753298233447344e-7*4194304*50)
        //   = ceil(12.381 + 120.6) = ceil(132.98) = 133
        // smea_dyn: 133*1.4 = 186.2 -> ceil(186.2) = 187 > 140 -> error=true
        let result = calculate_generation_cost(&GenerationCostParams {
            width: 2048,
            height: 2048,
            steps: 50,
            smea: SmeaMode::SmeaDyn,
            tier: 0,
            n_samples: 1,
            ..GenerationCostParams::default()
        })
        .unwrap();
        assert!(result.error);
        assert_eq!(result.error_code, Some(-3));
        assert_eq!(result.total_cost, 0);
    }
}
