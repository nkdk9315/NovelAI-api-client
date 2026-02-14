/// NovelAI Client Constants Tests
/// 定数のテスト
use novelai_api::constants;

// =============================================================================
// URL Constants Tests
// =============================================================================
mod url_constants {
    use super::*;

    #[test]
    fn should_have_valid_api_urls() {
        // Remove any env overrides for deterministic testing
        std::env::remove_var("NOVELAI_API_URL");
        std::env::remove_var("NOVELAI_STREAM_URL");
        std::env::remove_var("NOVELAI_ENCODE_URL");
        std::env::remove_var("NOVELAI_SUBSCRIPTION_URL");

        assert_eq!(
            constants::api_url(),
            "https://image.novelai.net/ai/generate-image"
        );
        assert_eq!(
            constants::stream_url(),
            "https://image.novelai.net/ai/generate-image-stream"
        );
        assert_eq!(
            constants::encode_url(),
            "https://image.novelai.net/ai/encode-vibe"
        );
        assert_eq!(
            constants::subscription_url(),
            "https://api.novelai.net/user/subscription"
        );
    }

    #[test]
    fn should_have_valid_augment_and_upscale_urls() {
        std::env::remove_var("NOVELAI_AUGMENT_URL");
        std::env::remove_var("NOVELAI_UPSCALE_URL");

        assert_eq!(
            constants::augment_url(),
            "https://image.novelai.net/ai/augment-image"
        );
        assert_eq!(
            constants::upscale_url(),
            "https://api.novelai.net/ai/upscale"
        );
    }
}

// =============================================================================
// Default Values Tests
// =============================================================================
mod default_values {
    use super::*;

    #[test]
    fn should_have_valid_default_model() {
        assert_eq!(constants::DEFAULT_MODEL, "nai-diffusion-4-5-full");
        assert!(
            constants::VALID_MODELS.contains(&constants::DEFAULT_MODEL),
            "DEFAULT_MODEL should be in VALID_MODELS"
        );
    }

    #[test]
    fn should_have_valid_default_dimensions() {
        assert_eq!(constants::DEFAULT_WIDTH, 832);
        assert_eq!(constants::DEFAULT_HEIGHT, 1216);
        assert_eq!(
            constants::DEFAULT_WIDTH % 64,
            0,
            "DEFAULT_WIDTH must be a multiple of 64"
        );
        assert_eq!(
            constants::DEFAULT_HEIGHT % 64,
            0,
            "DEFAULT_HEIGHT must be a multiple of 64"
        );
        assert!(
            (constants::DEFAULT_WIDTH as u64) * (constants::DEFAULT_HEIGHT as u64)
                <= constants::MAX_PIXELS,
            "DEFAULT_WIDTH * DEFAULT_HEIGHT must not exceed MAX_PIXELS"
        );
    }

    #[test]
    fn should_have_valid_default_generation_params() {
        assert_eq!(constants::DEFAULT_STEPS, 23);
        assert!((constants::DEFAULT_SCALE - 5.0).abs() < f64::EPSILON);
        assert_eq!(constants::DEFAULT_SAMPLER, "k_euler_ancestral");
        assert_eq!(constants::DEFAULT_NOISE_SCHEDULE, "karras");
    }

    #[test]
    fn should_have_valid_defry_defaults() {
        assert_eq!(constants::DEFAULT_DEFRY, 3);
        const { assert!(constants::DEFAULT_DEFRY <= constants::MAX_DEFRY) };
    }

    #[test]
    fn should_have_valid_upscale_defaults() {
        assert_eq!(constants::DEFAULT_UPSCALE_SCALE, 4);
        assert!(
            constants::VALID_UPSCALE_SCALES.contains(&constants::DEFAULT_UPSCALE_SCALE),
            "DEFAULT_UPSCALE_SCALE should be in VALID_UPSCALE_SCALES"
        );
    }
}

// =============================================================================
// Validation Constants Tests
// =============================================================================
mod validation_constants {
    use super::*;

    #[test]
    fn should_have_valid_samplers_array() {
        assert!(constants::VALID_SAMPLERS.contains(&"k_euler"));
        assert!(constants::VALID_SAMPLERS.contains(&"k_euler_ancestral"));
        assert!(constants::VALID_SAMPLERS.contains(&"k_dpmpp_2s_ancestral"));
        assert!(constants::VALID_SAMPLERS.contains(&"k_dpmpp_2m_sde"));
        assert!(constants::VALID_SAMPLERS.contains(&"k_dpmpp_2m"));
        assert!(constants::VALID_SAMPLERS.contains(&"k_dpmpp_sde"));
        assert_eq!(constants::VALID_SAMPLERS.len(), 6);
    }

    #[test]
    fn should_have_valid_models_array() {
        assert!(constants::VALID_MODELS.contains(&"nai-diffusion-4-curated-preview"));
        assert!(constants::VALID_MODELS.contains(&"nai-diffusion-4-full"));
        assert!(constants::VALID_MODELS.contains(&"nai-diffusion-4-5-curated"));
        assert!(constants::VALID_MODELS.contains(&"nai-diffusion-4-5-full"));
        assert_eq!(constants::VALID_MODELS.len(), 4);
    }

    #[test]
    fn should_have_valid_noise_schedules_array() {
        assert!(constants::VALID_NOISE_SCHEDULES.contains(&"karras"));
        assert!(constants::VALID_NOISE_SCHEDULES.contains(&"exponential"));
        assert!(constants::VALID_NOISE_SCHEDULES.contains(&"polyexponential"));
        assert_eq!(constants::VALID_NOISE_SCHEDULES.len(), 3);
    }
}

// =============================================================================
// Augment Tool Constants Tests
// =============================================================================
mod augment_tool_constants {
    use super::*;

    #[test]
    fn should_have_all_augment_req_types() {
        assert!(constants::AUGMENT_REQ_TYPES.contains(&"colorize"));
        assert!(constants::AUGMENT_REQ_TYPES.contains(&"declutter"));
        assert!(constants::AUGMENT_REQ_TYPES.contains(&"emotion"));
        assert!(constants::AUGMENT_REQ_TYPES.contains(&"sketch"));
        assert!(constants::AUGMENT_REQ_TYPES.contains(&"lineart"));
        assert!(constants::AUGMENT_REQ_TYPES.contains(&"bg-removal"));
        assert_eq!(constants::AUGMENT_REQ_TYPES.len(), 6);
    }

    #[test]
    fn should_have_all_emotion_keywords() {
        let expected_keywords = [
            "neutral", "happy", "sad", "angry", "scared", "surprised",
            "tired", "excited", "nervous", "thinking", "confused", "shy",
            "disgusted", "smug", "bored", "laughing", "irritated", "aroused",
            "embarrassed", "love", "worried", "determined", "hurt", "playful",
        ];

        for keyword in &expected_keywords {
            assert!(
                constants::EMOTION_KEYWORDS.contains(keyword),
                "EMOTION_KEYWORDS should contain '{}'",
                keyword
            );
        }
        assert_eq!(constants::EMOTION_KEYWORDS.len(), 24);
    }

    #[test]
    fn should_have_valid_defry_range() {
        assert_eq!(constants::MIN_DEFRY, 0);
        assert_eq!(constants::MAX_DEFRY, 5);
        const { assert!(constants::MIN_DEFRY < constants::MAX_DEFRY) };
    }

    #[test]
    fn should_have_valid_upscale_scales() {
        assert!(constants::VALID_UPSCALE_SCALES.contains(&2));
        assert!(constants::VALID_UPSCALE_SCALES.contains(&4));
        assert_eq!(constants::VALID_UPSCALE_SCALES.len(), 2);
    }
}

// =============================================================================
// Limit Constants Tests
// =============================================================================
mod limit_constants {
    use super::*;

    #[test]
    fn should_have_valid_pixel_limits() {
        assert_eq!(constants::MAX_PIXELS, 3_145_728); // 2048 * 1536 (server-side limit)
        assert_eq!(constants::MIN_DIMENSION, 64);
    }

    #[test]
    fn should_have_valid_step_limits() {
        assert_eq!(constants::MIN_STEPS, 1);
        assert_eq!(constants::MAX_STEPS, 50);
    }

    #[test]
    fn should_have_valid_scale_limits() {
        assert!((constants::MIN_SCALE - 0.0).abs() < f64::EPSILON);
        assert!((constants::MAX_SCALE - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn should_have_valid_seed_limit() {
        assert_eq!(constants::MAX_SEED, 4_294_967_295); // 2^32 - 1
    }

    #[test]
    fn should_have_valid_token_limits() {
        assert_eq!(constants::MAX_TOKENS, 512);
    }

    #[test]
    fn should_have_valid_character_and_vibe_limits() {
        assert_eq!(constants::MAX_CHARACTERS, 6);
        assert_eq!(constants::MAX_VIBES, 10);
    }
}

// =============================================================================
// Model Key Map Tests
// =============================================================================
mod model_key_map {
    use super::*;

    #[test]
    fn should_have_mappings_for_all_valid_models() {
        for model in constants::VALID_MODELS {
            assert!(
                constants::model_key_from_str(model).is_some(),
                "model_key_from_str should return Some for valid model '{}'",
                model
            );
        }
    }

    #[test]
    fn should_have_correct_model_key_mappings() {
        assert_eq!(
            constants::model_key_from_str("nai-diffusion-4-curated-preview"),
            Some("v4curated")
        );
        assert_eq!(
            constants::model_key_from_str("nai-diffusion-4-full"),
            Some("v4full")
        );
        assert_eq!(
            constants::model_key_from_str("nai-diffusion-4-5-curated"),
            Some("v4-5curated")
        );
        assert_eq!(
            constants::model_key_from_str("nai-diffusion-4-5-full"),
            Some("v4-5full")
        );
    }
}

// =============================================================================
// Anlas Cost Constants Tests
// =============================================================================
mod anlas_cost_constants {
    use super::*;

    #[test]
    fn should_have_valid_opus_free_conditions() {
        assert_eq!(constants::OPUS_FREE_PIXELS, 1_048_576);
        assert_eq!(constants::OPUS_FREE_MAX_STEPS, 28);
        assert_eq!(constants::OPUS_MIN_TIER, 3);
    }

    #[test]
    fn should_have_valid_per_image_cost_limits() {
        assert_eq!(constants::MAX_COST_PER_IMAGE, 140);
        assert_eq!(constants::MIN_COST_PER_IMAGE, 2);
        const { assert!(constants::MIN_COST_PER_IMAGE < constants::MAX_COST_PER_IMAGE) };
    }

    #[test]
    fn should_have_valid_vibe_cost_constants() {
        assert_eq!(constants::VIBE_BATCH_PRICE, 2);
        assert_eq!(constants::VIBE_FREE_THRESHOLD, 4);
        assert_eq!(constants::VIBE_ENCODE_PRICE, 2);
    }

    #[test]
    fn should_have_valid_character_reference_cost() {
        assert_eq!(constants::CHAR_REF_PRICE, 5);
    }

    #[test]
    fn should_have_valid_v4_cost_coefficients() {
        let linear_diff = (constants::V4_COST_COEFF_LINEAR - 2.951823174884865e-6).abs();
        assert!(
            linear_diff < 1e-15,
            "V4_COST_COEFF_LINEAR mismatch: diff = {:e}",
            linear_diff
        );

        let step_diff = (constants::V4_COST_COEFF_STEP - 5.753298233447344e-7).abs();
        assert!(
            step_diff < 1e-15,
            "V4_COST_COEFF_STEP mismatch: diff = {:e}",
            step_diff
        );
    }

    #[test]
    fn should_have_valid_augment_constants() {
        assert_eq!(constants::AUGMENT_FIXED_STEPS, 28);
        assert_eq!(constants::AUGMENT_MIN_PIXELS, 1_048_576);
        assert_eq!(constants::BG_REMOVAL_MULTIPLIER, 3);
        assert_eq!(constants::BG_REMOVAL_ADDEND, 5);
    }

    #[test]
    fn should_have_valid_upscale_cost_table() {
        assert_eq!(constants::UPSCALE_COST_TABLE.len(), 5);
        assert_eq!(constants::UPSCALE_COST_TABLE[0], (262_144, 1));
        assert_eq!(constants::UPSCALE_COST_TABLE[4], (1_048_576, 7));
        assert_eq!(constants::UPSCALE_OPUS_FREE_PIXELS, 409_600);
    }

    #[test]
    fn should_have_valid_grid_size_and_inpaint_threshold() {
        assert_eq!(constants::GRID_SIZE, 64);
        assert!((constants::INPAINT_THRESHOLD_RATIO - 0.8).abs() < f64::EPSILON);
    }
}
