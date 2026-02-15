//! NovelAI Schema Validation Tests
//! Ported from ts-api/tests/schemas.test.ts
//!
//! Validates all schema types and their validation logic.
//! Tests for non-integer values (width: 512.5, steps: 23.5, seed: 123.4, defry: 2.5)
//! and negative u32 values (defry: -1) are SKIPPED because Rust's type system handles them.
//! Token count tests use async validate_async() with the T5 tokenizer.

#[cfg(test)]
mod tests {
    use novelai_api::constants::*;
    use novelai_api::schemas::*;

    // =========================================================================
    // Test Helpers
    // =========================================================================

    fn make_generate_params(prompt: &str) -> GenerateParams {
        GenerateParams {
            prompt: prompt.to_string(),
            ..Default::default()
        }
    }

    fn make_valid_vibe_result() -> VibeEncodeResult {
        VibeEncodeResult {
            encoding: "SGVsbG8gV29ybGQ=".to_string(),
            model: Model::NaiDiffusion45Full,
            information_extracted: 0.7,
            strength: 0.7,
            source_image_hash: "a".repeat(64),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            saved_path: None,
            anlas_remaining: None,
            anlas_consumed: None,
        }
    }

    /// Helper to create a default VibeConfig wrapping a VibeItem.
    fn make_vibe_config(item: VibeItem) -> VibeConfig {
        VibeConfig {
            item,
            strength: DEFAULT_VIBE_STRENGTH,
            info_extracted: DEFAULT_VIBE_INFO_EXTRACTED,
        }
    }

    // =========================================================================
    // CharacterConfig Tests
    // =========================================================================
    mod character_config {
        use super::*;

        #[test]
        fn should_validate_valid_character_config() {
            let config = CharacterConfig {
                prompt: "1girl, beautiful".to_string(),
                center_x: 0.5,
                center_y: 0.5,
                negative_prompt: "lowres".to_string(),
            };
            assert!(config.validate().is_ok());
        }

        #[test]
        fn should_verify_defaults_for_center_and_negative_prompt() {
            let config = CharacterConfig {
                prompt: "1girl".to_string(),
                ..Default::default()
            };
            assert!((config.center_x - 0.5).abs() < f64::EPSILON);
            assert!((config.center_y - 0.5).abs() < f64::EPSILON);
            assert_eq!(config.negative_prompt, "");
            assert!(config.validate().is_ok());
        }

        #[test]
        fn should_reject_empty_prompt() {
            let config = CharacterConfig {
                prompt: "".to_string(),
                ..Default::default()
            };
            assert!(config.validate().is_err());
        }

        #[test]
        fn should_reject_center_x_outside_0_1_range() {
            let config_below = CharacterConfig {
                prompt: "1girl".to_string(),
                center_x: -0.1,
                ..Default::default()
            };
            assert!(config_below.validate().is_err());

            let config_above = CharacterConfig {
                prompt: "1girl".to_string(),
                center_x: 1.1,
                ..Default::default()
            };
            assert!(config_above.validate().is_err());
        }

        #[test]
        fn should_reject_center_y_outside_0_1_range() {
            let config_below = CharacterConfig {
                prompt: "1girl".to_string(),
                center_y: -0.1,
                ..Default::default()
            };
            assert!(config_below.validate().is_err());

            let config_above = CharacterConfig {
                prompt: "1girl".to_string(),
                center_y: 1.1,
                ..Default::default()
            };
            assert!(config_above.validate().is_err());
        }
    }

    // =========================================================================
    // CharacterReferenceConfig Tests
    // =========================================================================
    mod character_reference_config {
        use super::*;

        #[test]
        fn should_validate_with_file_path_image_input() {
            let config = CharacterReferenceConfig {
                image: ImageInput::FilePath("path/to/image.png".into()),
                strength: 0.6,
                fidelity: 1.0,
                mode: CharRefMode::CharacterAndStyle,
            };
            assert!(config.validate().is_ok());
        }

        #[test]
        fn should_validate_with_bytes_image_input() {
            let config = CharacterReferenceConfig {
                image: ImageInput::Bytes(vec![1, 2, 3]),
                strength: 0.6,
                fidelity: 1.0,
                mode: CharRefMode::CharacterAndStyle,
            };
            assert!(config.validate().is_ok());
        }

        #[test]
        fn should_reject_empty_string_image_input() {
            let config = CharacterReferenceConfig {
                image: ImageInput::FilePath("".into()),
                strength: 0.6,
                fidelity: 1.0,
                mode: CharRefMode::CharacterAndStyle,
            };
            assert!(config.validate().is_err());
        }

        #[test]
        fn should_verify_expected_default_values() {
            // CharacterReferenceConfig has no Default impl, so construct with expected defaults
            let config = CharacterReferenceConfig {
                image: ImageInput::FilePath("test.png".into()),
                strength: 0.6,
                fidelity: 1.0,
                mode: CharRefMode::CharacterAndStyle,
            };
            assert!((config.strength - 0.6).abs() < f64::EPSILON);
            assert!((config.fidelity - 1.0).abs() < f64::EPSILON);
            assert_eq!(config.mode, CharRefMode::CharacterAndStyle);
            assert!(config.validate().is_ok());
        }

        #[test]
        fn should_reject_fidelity_outside_0_1_range() {
            let config_below = CharacterReferenceConfig {
                image: ImageInput::FilePath("test.png".into()),
                strength: 0.6,
                fidelity: -0.1,
                mode: CharRefMode::CharacterAndStyle,
            };
            assert!(config_below.validate().is_err());

            let config_above = CharacterReferenceConfig {
                image: ImageInput::FilePath("test.png".into()),
                strength: 0.6,
                fidelity: 1.1,
                mode: CharRefMode::CharacterAndStyle,
            };
            assert!(config_above.validate().is_err());
        }

        #[test]
        fn should_reject_strength_outside_0_1_range() {
            let config_below = CharacterReferenceConfig {
                image: ImageInput::FilePath("test.png".into()),
                strength: -0.1,
                fidelity: 1.0,
                mode: CharRefMode::CharacterAndStyle,
            };
            assert!(config_below.validate().is_err());

            let config_above = CharacterReferenceConfig {
                image: ImageInput::FilePath("test.png".into()),
                strength: 1.1,
                fidelity: 1.0,
                mode: CharRefMode::CharacterAndStyle,
            };
            assert!(config_above.validate().is_err());
        }

        #[test]
        fn should_accept_all_valid_modes() {
            for mode in [
                CharRefMode::Character,
                CharRefMode::CharacterAndStyle,
                CharRefMode::Style,
            ] {
                let config = CharacterReferenceConfig {
                    image: ImageInput::FilePath("test.png".into()),
                    strength: 0.6,
                    fidelity: 1.0,
                    mode,
                };
                assert!(config.validate().is_ok());
            }
        }
    }

    // =========================================================================
    // VibeEncodeResult Tests
    // =========================================================================
    mod vibe_encode_result {
        use super::*;

        #[test]
        fn should_accept_valid_vibe_encode_result() {
            let result = make_valid_vibe_result();
            assert!(result.validate().is_ok());
        }

        #[test]
        fn should_accept_uppercase_hex_in_source_image_hash() {
            let mut result = make_valid_vibe_result();
            result.source_image_hash =
                format!("ABCDEFabcdef0123456789{}", "a".repeat(42));
            assert!(result.validate().is_ok());
        }

        #[test]
        fn should_reject_non_base64_encoding() {
            let mut result = make_valid_vibe_result();
            result.encoding = "invalid base64 with spaces!".to_string();
            let err = result.validate().unwrap_err();
            assert!(err.to_string().contains("base64"));
        }

        #[test]
        fn should_reject_empty_encoding() {
            let mut result = make_valid_vibe_result();
            result.encoding = "".to_string();
            assert!(result.validate().is_err());
        }
    }

    // =========================================================================
    // GenerateParams Tests
    // =========================================================================
    mod generate_params {

        // -----------------------------------------------------------------
        // Basic Validation
        // -----------------------------------------------------------------
        mod basic {
            use super::super::*;

            #[test]
            fn should_validate_minimal_params_prompt_only() {
                let params = make_generate_params("1girl");
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_apply_all_defaults_correctly() {
                let params = make_generate_params("1girl");
                assert!(params.action.is_generate());
                assert_eq!(params.model, Model::default());
                assert_eq!(params.width, DEFAULT_WIDTH);
                assert_eq!(params.height, DEFAULT_HEIGHT);
                assert_eq!(params.steps, DEFAULT_STEPS);
                assert!((params.scale - DEFAULT_SCALE).abs() < f64::EPSILON);
                assert_eq!(params.sampler, Sampler::default());
                assert_eq!(params.noise_schedule, NoiseSchedule::default());
            }
        }

        // -----------------------------------------------------------------
        // Dimension Validation
        // -----------------------------------------------------------------
        mod dimensions {
            use super::super::*;

            #[test]
            fn should_accept_width_height_as_multiples_of_64() {
                let mut params = make_generate_params("1girl");
                params.width = 512;
                params.height = 768;
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_reject_width_not_a_multiple_of_64() {
                let mut params = make_generate_params("1girl");
                params.width = 500;
                let err = params.validate().unwrap_err();
                assert!(err.to_string().contains("multiple of 64"));
            }

            #[test]
            fn should_reject_height_not_a_multiple_of_64() {
                let mut params = make_generate_params("1girl");
                params.height = 700;
                assert!(params.validate().is_err());
            }

            #[test]
            fn should_reject_width_below_min_dimension() {
                let mut params = make_generate_params("1girl");
                params.width = 32;
                assert!(params.validate().is_err());
            }

            // SKIP: non-integer width (512.5) - Rust type system handles this

            #[test]
            fn should_reject_width_exceeding_max_generation_dimension() {
                let mut params = make_generate_params("1girl");
                params.width = 2112;
                assert!(params.validate().is_err());
            }

            #[test]
            fn should_reject_height_exceeding_max_generation_dimension() {
                let mut params = make_generate_params("1girl");
                params.height = 2112;
                assert!(params.validate().is_err());
            }

            #[test]
            fn should_accept_max_dimension_2048() {
                let mut params = make_generate_params("1girl");
                params.width = 2048;
                params.height = 1536;
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_reject_total_pixels_exceeding_max_pixels() {
                // 2048 x 1536 = 3,145,728 which equals MAX_PIXELS, should pass
                let mut params_pass = make_generate_params("1girl");
                params_pass.width = 2048;
                params_pass.height = 1536;
                assert!(params_pass.validate().is_ok());

                // 2048 x 1600 = 3,276,800 which exceeds MAX_PIXELS, should fail
                let mut params = make_generate_params("1girl");
                params.width = 2048;
                params.height = 1600;
                let err = params.validate().unwrap_err();
                assert!(err.to_string().contains("exceeds limit"));
            }
        }

        // -----------------------------------------------------------------
        // Steps Validation
        // -----------------------------------------------------------------
        mod steps {
            use super::super::*;

            #[test]
            fn should_accept_valid_steps() {
                let mut params1 = make_generate_params("1girl");
                params1.steps = 1;
                assert!(params1.validate().is_ok());

                let mut params2 = make_generate_params("1girl");
                params2.steps = 50;
                assert!(params2.validate().is_ok());
            }

            #[test]
            fn should_reject_steps_below_min() {
                let mut params = make_generate_params("1girl");
                params.steps = 0;
                assert!(params.validate().is_err());
            }

            #[test]
            fn should_reject_steps_above_max() {
                let mut params = make_generate_params("1girl");
                params.steps = 51;
                assert!(params.validate().is_err());
            }

            // SKIP: non-integer steps (23.5) - Rust type system handles this
        }

        // -----------------------------------------------------------------
        // Scale Validation
        // -----------------------------------------------------------------
        mod scale {
            use super::super::*;

            #[test]
            fn should_accept_valid_scale() {
                let mut params1 = make_generate_params("1girl");
                params1.scale = 0.0;
                assert!(params1.validate().is_ok());

                let mut params2 = make_generate_params("1girl");
                params2.scale = 10.0;
                assert!(params2.validate().is_ok());

                let mut params3 = make_generate_params("1girl");
                params3.scale = 5.5;
                assert!(params3.validate().is_ok());
            }

            #[test]
            fn should_reject_scale_above_max() {
                let mut params = make_generate_params("1girl");
                params.scale = 10.1;
                assert!(params.validate().is_err());
            }
        }

        // -----------------------------------------------------------------
        // Seed Validation
        // -----------------------------------------------------------------
        mod seed {
            use super::super::*;

            #[test]
            fn should_accept_valid_seed() {
                let mut params1 = make_generate_params("1girl");
                params1.seed = Some(0);
                assert!(params1.validate().is_ok());

                let mut params2 = make_generate_params("1girl");
                params2.seed = Some(MAX_SEED as u64);
                assert!(params2.validate().is_ok());
            }

            #[test]
            fn should_reject_seed_above_max() {
                let mut params = make_generate_params("1girl");
                params.seed = Some(MAX_SEED as u64 + 1);
                assert!(params.validate().is_err());
            }

            // SKIP: negative seed (-1) - Rust type system handles this (u64)
            // SKIP: non-integer seed (123.4) - Rust type system handles this (u64)
        }

        // -----------------------------------------------------------------
        // Enum Validation
        // -----------------------------------------------------------------
        mod enums {
            use super::super::*;

            #[test]
            fn should_accept_all_valid_models() {
                for model in [
                    Model::NaiDiffusion4CuratedPreview,
                    Model::NaiDiffusion4Full,
                    Model::NaiDiffusion45Curated,
                    Model::NaiDiffusion45Full,
                ] {
                    let mut params = make_generate_params("1girl");
                    params.model = model;
                    assert!(params.validate().is_ok());
                }
            }

            // Invalid model is handled by Rust type system (enum)

            #[test]
            fn should_accept_all_valid_samplers() {
                for sampler in [
                    Sampler::KEuler,
                    Sampler::KEulerAncestral,
                    Sampler::KDpmpp2sAncestral,
                    Sampler::KDpmpp2mSde,
                    Sampler::KDpmpp2m,
                    Sampler::KDpmppSde,
                ] {
                    let mut params = make_generate_params("1girl");
                    params.sampler = sampler;
                    assert!(params.validate().is_ok());
                }
            }

            // Invalid sampler is handled by Rust type system (enum)

            #[test]
            fn should_accept_all_valid_noise_schedules() {
                for schedule in [
                    NoiseSchedule::Karras,
                    NoiseSchedule::Exponential,
                    NoiseSchedule::Polyexponential,
                ] {
                    let mut params = make_generate_params("1girl");
                    params.noise_schedule = schedule;
                    assert!(params.validate().is_ok());
                }
            }

            // Invalid noise_schedule is handled by Rust type system (enum)
            // Invalid action is handled by Rust type system (enum)

            #[test]
            fn should_accept_generate_action() {
                let mut params = make_generate_params("1girl");
                params.action = GenerateAction::Generate;
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_accept_img2img_action_with_data() {
                let mut params = make_generate_params("1girl");
                params.action = GenerateAction::Img2Img {
                    source_image: ImageInput::FilePath("path/to/image.png".into()),
                    strength: 0.5,
                    noise: 0.0,
                };
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_accept_infill_action_with_data() {
                let mut params = make_generate_params("1girl");
                params.action = GenerateAction::Infill {
                    source_image: ImageInput::FilePath("path/to/image.png".into()),
                    mask: ImageInput::FilePath("path/to/mask.png".into()),
                    mask_strength: 0.5,
                    color_correct: false,
                    hybrid_strength: None,
                    hybrid_noise: None,
                };
                assert!(params.validate().is_ok());
            }
        }

        // -----------------------------------------------------------------
        // cfg_rescale Validation
        // -----------------------------------------------------------------
        mod cfg_rescale {
            use super::super::*;

            #[test]
            fn should_accept_valid_cfg_rescale() {
                let mut params1 = make_generate_params("1girl");
                params1.cfg_rescale = 0.0;
                assert!(params1.validate().is_ok());

                let mut params2 = make_generate_params("1girl");
                params2.cfg_rescale = 1.0;
                assert!(params2.validate().is_ok());

                let mut params3 = make_generate_params("1girl");
                params3.cfg_rescale = 0.5;
                assert!(params3.validate().is_ok());
            }

            #[test]
            fn should_reject_cfg_rescale_out_of_range() {
                let mut params1 = make_generate_params("1girl");
                params1.cfg_rescale = -0.1;
                assert!(params1.validate().is_err());

                let mut params2 = make_generate_params("1girl");
                params2.cfg_rescale = 1.1;
                assert!(params2.validate().is_err());
            }
        }

        // -----------------------------------------------------------------
        // img2img Validation
        // -----------------------------------------------------------------
        mod img2img {
            use super::super::*;

            #[test]
            fn should_accept_img2img_with_valid_params() {
                let mut params = make_generate_params("1girl");
                params.action = GenerateAction::Img2Img {
                    source_image: ImageInput::FilePath("path/to/image.png".into()),
                    strength: 0.5,
                    noise: 0.0,
                };
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_reject_img2img_with_empty_source_image() {
                let mut params = make_generate_params("1girl");
                params.action = GenerateAction::Img2Img {
                    source_image: ImageInput::FilePath("".into()),
                    strength: 0.5,
                    noise: 0.0,
                };
                let err = params.validate().unwrap_err();
                assert!(err.to_string().contains("must not be empty"));
            }

            #[test]
            fn should_validate_img2img_strength_range() {
                let mut params1 = make_generate_params("1girl");
                params1.action = GenerateAction::Img2Img {
                    source_image: ImageInput::FilePath("image.png".into()),
                    strength: -0.1,
                    noise: 0.0,
                };
                assert!(params1.validate().is_err());

                let mut params2 = make_generate_params("1girl");
                params2.action = GenerateAction::Img2Img {
                    source_image: ImageInput::FilePath("image.png".into()),
                    strength: 1.1,
                    noise: 0.0,
                };
                assert!(params2.validate().is_err());
            }

            #[test]
            fn should_validate_img2img_noise_range() {
                let mut params = make_generate_params("1girl");
                params.action = GenerateAction::Img2Img {
                    source_image: ImageInput::FilePath("image.png".into()),
                    strength: 0.5,
                    noise: 1.1,
                };
                assert!(params.validate().is_err());
            }

            // source_image/mask required checks are now enforced by type system
            // (data-carrying enum makes illegal states unrepresentable)
        }

        // -----------------------------------------------------------------
        // infill Validation
        // -----------------------------------------------------------------
        mod infill {
            use super::super::*;

            #[test]
            fn should_accept_infill_with_valid_params() {
                let mut params = make_generate_params("1girl");
                params.action = GenerateAction::Infill {
                    source_image: ImageInput::FilePath("image.png".into()),
                    mask: ImageInput::FilePath("mask.png".into()),
                    mask_strength: 0.5,
                    color_correct: false,
                    hybrid_strength: None,
                    hybrid_noise: None,
                };
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_reject_infill_with_empty_source_image() {
                let mut params = make_generate_params("1girl");
                params.action = GenerateAction::Infill {
                    source_image: ImageInput::FilePath("".into()),
                    mask: ImageInput::FilePath("mask.png".into()),
                    mask_strength: 0.5,
                    color_correct: false,
                    hybrid_strength: None,
                    hybrid_noise: None,
                };
                assert!(params.validate().is_err());
            }

            #[test]
            fn should_reject_infill_with_empty_mask() {
                let mut params = make_generate_params("1girl");
                params.action = GenerateAction::Infill {
                    source_image: ImageInput::FilePath("image.png".into()),
                    mask: ImageInput::FilePath("".into()),
                    mask_strength: 0.5,
                    color_correct: false,
                    hybrid_strength: None,
                    hybrid_noise: None,
                };
                assert!(params.validate().is_err());
            }

            #[test]
            fn should_reject_infill_mask_strength_out_of_range() {
                let mut params = make_generate_params("1girl");
                params.action = GenerateAction::Infill {
                    source_image: ImageInput::FilePath("image.png".into()),
                    mask: ImageInput::FilePath("mask.png".into()),
                    mask_strength: 0.0, // below 0.01
                    color_correct: false,
                    hybrid_strength: None,
                    hybrid_noise: None,
                };
                assert!(params.validate().is_err());
            }

            #[test]
            fn should_reject_infill_hybrid_strength_out_of_range() {
                let mut params = make_generate_params("1girl");
                params.action = GenerateAction::Infill {
                    source_image: ImageInput::FilePath("image.png".into()),
                    mask: ImageInput::FilePath("mask.png".into()),
                    mask_strength: 0.5,
                    color_correct: false,
                    hybrid_strength: Some(1.0), // above 0.99
                    hybrid_noise: None,
                };
                assert!(params.validate().is_err());
            }

            #[test]
            fn should_reject_infill_hybrid_noise_out_of_range() {
                let mut params = make_generate_params("1girl");
                params.action = GenerateAction::Infill {
                    source_image: ImageInput::FilePath("image.png".into()),
                    mask: ImageInput::FilePath("mask.png".into()),
                    mask_strength: 0.5,
                    color_correct: false,
                    hybrid_strength: None,
                    hybrid_noise: Some(1.0), // above 0.99
                };
                assert!(params.validate().is_err());
            }
        }

        // -----------------------------------------------------------------
        // vibes and character_reference mutual exclusion
        // -----------------------------------------------------------------
        mod vibes_charref {
            use super::super::*;

            #[test]
            fn should_reject_vibes_and_character_reference_used_together() {
                let mut params = make_generate_params("1girl");
                params.vibes = Some(vec![make_vibe_config(
                    VibeItem::FilePath("vibe1.naiv4vibe".into()),
                )]);
                params.character_reference = Some(CharacterReferenceConfig {
                    image: ImageInput::FilePath("test.png".into()),
                    strength: 0.6,
                    fidelity: 1.0,
                    mode: CharRefMode::CharacterAndStyle,
                });
                let err = params.validate().unwrap_err();
                assert!(err.to_string().contains("cannot be used together"));
            }

            #[test]
            fn should_accept_vibes_without_character_reference() {
                let mut params = make_generate_params("1girl");
                params.vibes = Some(vec![make_vibe_config(
                    VibeItem::FilePath("vibe1.naiv4vibe".into()),
                )]);
                assert!(params.validate().is_ok());
            }
        }

        // -----------------------------------------------------------------
        // VibeConfig validation (strength and info_extracted ranges)
        // -----------------------------------------------------------------
        mod vibe_config_validation {
            use super::super::*;

            #[test]
            fn should_accept_valid_vibe_config_ranges() {
                let mut params = make_generate_params("1girl");
                params.vibes = Some(vec![
                    VibeConfig {
                        item: VibeItem::FilePath("vibe1.naiv4vibe".into()),
                        strength: 0.5,
                        info_extracted: 0.7,
                    },
                    VibeConfig {
                        item: VibeItem::FilePath("vibe2.naiv4vibe".into()),
                        strength: 1.0,
                        info_extracted: 0.0,
                    },
                ]);
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_reject_vibe_strength_out_of_range() {
                let mut params = make_generate_params("1girl");
                params.vibes = Some(vec![VibeConfig {
                    item: VibeItem::FilePath("vibe1.naiv4vibe".into()),
                    strength: 1.5,
                    info_extracted: 0.7,
                }]);
                let err = params.validate().unwrap_err();
                assert!(err.to_string().contains("strength"));
            }

            #[test]
            fn should_reject_vibe_info_extracted_out_of_range() {
                let mut params = make_generate_params("1girl");
                params.vibes = Some(vec![VibeConfig {
                    item: VibeItem::FilePath("vibe1.naiv4vibe".into()),
                    strength: 0.5,
                    info_extracted: -0.1,
                }]);
                let err = params.validate().unwrap_err();
                assert!(err.to_string().contains("info_extracted"));
            }

            // Length mismatch tests are removed: no longer possible with VibeConfig
            // (strength and info_extracted are bundled per-vibe)
        }

        // -----------------------------------------------------------------
        // save (SaveTarget) validation
        // -----------------------------------------------------------------
        mod save_options {
            use super::super::*;

            // Mutual exclusion test removed: SaveTarget enum makes it impossible
            // to have both save_path and save_dir at the same time.

            #[test]
            fn should_accept_exact_path() {
                let mut params = make_generate_params("1girl");
                params.save = SaveTarget::ExactPath("/path/to/file.png".to_string());
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_accept_directory() {
                let mut params = make_generate_params("1girl");
                params.save = SaveTarget::Directory {
                    dir: "/path/to/dir/".to_string(),
                    filename: None,
                };
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_accept_none_save_target() {
                let params = make_generate_params("1girl");
                // Default is SaveTarget::None
                assert!(params.validate().is_ok());
            }
        }

        // -----------------------------------------------------------------
        // Path Traversal Defense
        // -----------------------------------------------------------------
        mod path_traversal {
            use super::super::*;

            #[test]
            fn should_reject_save_path_with_path_traversal() {
                let mut params = make_generate_params("1girl");
                params.save = SaveTarget::ExactPath("../etc/passwd".to_string());
                let err = params.validate().unwrap_err();
                assert!(err.to_string().contains("path traversal"));
            }

            #[test]
            fn should_reject_save_dir_with_path_traversal() {
                let mut params = make_generate_params("1girl");
                params.save = SaveTarget::Directory {
                    dir: "/output/../../etc/".to_string(),
                    filename: None,
                };
                let err = params.validate().unwrap_err();
                assert!(err.to_string().contains("path traversal"));
            }

            #[test]
            fn should_reject_backslash_path_traversal() {
                let mut params = make_generate_params("1girl");
                params.save = SaveTarget::ExactPath(
                    "..\\windows\\system32\\file.png".to_string(),
                );
                let err = params.validate().unwrap_err();
                assert!(err.to_string().contains("path traversal"));
            }

            #[test]
            fn should_accept_valid_paths_without_traversal() {
                let mut params = make_generate_params("1girl");
                params.save = SaveTarget::ExactPath(
                    "/home/user/images/output.png".to_string(),
                );
                assert!(params.validate().is_ok());
            }
        }

        // -----------------------------------------------------------------
        // Vibe Items
        // -----------------------------------------------------------------
        mod vibe_items {
            use super::super::*;

            #[test]
            fn should_accept_string_vibes_file_paths() {
                let mut params = make_generate_params("1girl");
                params.vibes = Some(vec![
                    make_vibe_config(VibeItem::FilePath("vibe1.naiv4vibe".into())),
                    make_vibe_config(VibeItem::FilePath("path/to/vibe2.naiv4vibe".into())),
                ]);
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_accept_vibe_encode_result_objects_as_vibes() {
                let vibe_result = make_valid_vibe_result();
                let mut params = make_generate_params("1girl");
                params.vibes = Some(vec![make_vibe_config(VibeItem::Encoded(vibe_result))]);
                assert!(params.validate().is_ok());
            }

            // SKIP: number as vibe item - Rust type system handles this (enum)
            // SKIP: boolean as vibe item - Rust type system handles this (enum)

            #[test]
            fn should_reject_empty_string_as_vibe_item() {
                let mut params = make_generate_params("1girl");
                params.vibes = Some(vec![make_vibe_config(VibeItem::FilePath("".into()))]);
                assert!(params.validate().is_err());
            }
        }

        // -----------------------------------------------------------------
        // Characters Validation
        // -----------------------------------------------------------------
        mod characters {
            use super::super::*;

            #[test]
            fn should_accept_valid_characters_array() {
                let mut params = make_generate_params("2girls");
                params.characters = Some(vec![
                    CharacterConfig {
                        prompt: "girl A".to_string(),
                        center_x: 0.3,
                        center_y: 0.5,
                        negative_prompt: String::new(),
                    },
                    CharacterConfig {
                        prompt: "girl B".to_string(),
                        center_x: 0.7,
                        center_y: 0.5,
                        negative_prompt: String::new(),
                    },
                ]);
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_reject_characters_exceeding_max_count() {
                let too_many_chars: Vec<CharacterConfig> =
                    (0..MAX_CHARACTERS + 1)
                        .map(|_| CharacterConfig {
                            prompt: "test".to_string(),
                            ..Default::default()
                        })
                        .collect();
                let mut params = make_generate_params("many girls");
                params.characters = Some(too_many_chars);
                assert!(params.validate().is_err());
            }
        }

        // -----------------------------------------------------------------
        // Token Count Validation
        // -----------------------------------------------------------------
        mod token_count {
            use super::super::*;

            #[tokio::test]
            async fn should_accept_short_prompts_under_512_tokens() {
                let params = make_generate_params(
                    "a beautiful landscape with mountains and rivers",
                );
                assert!(params.validate_async().await.is_ok());
            }

            #[tokio::test]
            async fn should_reject_prompts_exceeding_512_tokens() {
                let long_prompt = vec!["masterpiece beautiful detailed anime girl"; 600]
                    .join(", ");
                let params = make_generate_params(&long_prompt);
                assert!(params.validate_async().await.is_err());
            }

            #[tokio::test]
            async fn should_accept_combined_positive_prompts_under_512_tokens() {
                let mut params = make_generate_params("masterpiece, best quality, 1girl");
                params.characters = Some(vec![
                    CharacterConfig {
                        prompt: "red hair, blue eyes".to_string(),
                        ..Default::default()
                    },
                    CharacterConfig {
                        prompt: "white dress".to_string(),
                        ..Default::default()
                    },
                ]);
                assert!(params.validate_async().await.is_ok());
            }

            #[tokio::test]
            async fn should_reject_combined_positive_prompts_exceeding_512_tokens() {
                let base_prompt =
                    vec!["masterpiece beautiful"; 250].join(", ");
                let char_prompt1 =
                    vec!["detailed anime girl"; 200].join(", ");
                let char_prompt2 =
                    vec!["stunning artwork"; 200].join(", ");

                let mut params = make_generate_params(&base_prompt);
                params.characters = Some(vec![
                    CharacterConfig {
                        prompt: char_prompt1,
                        ..Default::default()
                    },
                    CharacterConfig {
                        prompt: char_prompt2,
                        ..Default::default()
                    },
                ]);
                let err = params.validate_async().await.err().unwrap();
                assert!(err.to_string().contains("token count"));
            }

            #[tokio::test]
            async fn should_accept_combined_negative_prompts_under_512_tokens() {
                let mut params = make_generate_params("1girl");
                params.negative_prompt =
                    Some("lowres, bad anatomy".to_string());
                params.characters = Some(vec![
                    CharacterConfig {
                        prompt: "girl A".to_string(),
                        negative_prompt: "extra limbs".to_string(),
                        ..Default::default()
                    },
                    CharacterConfig {
                        prompt: "girl B".to_string(),
                        negative_prompt: "bad hands".to_string(),
                        ..Default::default()
                    },
                ]);
                assert!(params.validate_async().await.is_ok());
            }

            #[tokio::test]
            async fn should_reject_combined_negative_prompts_exceeding_512_tokens() {
                let base_negative =
                    vec!["lowres bad anatomy"; 250].join(", ");
                let char_negative1 =
                    vec!["extra limbs deformed"; 200].join(", ");
                let char_negative2 =
                    vec!["ugly blurry"; 200].join(", ");

                let mut params = make_generate_params("1girl");
                params.negative_prompt = Some(base_negative);
                params.characters = Some(vec![
                    CharacterConfig {
                        prompt: "girl A".to_string(),
                        negative_prompt: char_negative1,
                        ..Default::default()
                    },
                    CharacterConfig {
                        prompt: "girl B".to_string(),
                        negative_prompt: char_negative2,
                        ..Default::default()
                    },
                ]);
                let err = params.validate_async().await.err().unwrap();
                assert!(err.to_string().contains("token count"));
            }

            #[tokio::test]
            async fn should_validate_positive_and_negative_prompts_independently() {
                let long_positive =
                    vec!["masterpiece beautiful"; 600].join(", ");

                let mut params = make_generate_params(&long_positive);
                params.negative_prompt = Some("lowres".to_string());
                params.characters = Some(vec![CharacterConfig {
                    prompt: "short prompt".to_string(),
                    negative_prompt: "bad".to_string(),
                    ..Default::default()
                }]);
                // Should fail for positive prompt only
                assert!(params.validate_async().await.is_err());
            }

            #[tokio::test]
            async fn should_count_only_character_prompts_when_base_prompt_is_empty() {
                let char_prompt1 =
                    vec!["detailed anime girl"; 300].join(", ");
                let char_prompt2 =
                    vec!["stunning artwork"; 300].join(", ");

                let mut params = make_generate_params("");
                params.characters = Some(vec![
                    CharacterConfig {
                        prompt: char_prompt1,
                        ..Default::default()
                    },
                    CharacterConfig {
                        prompt: char_prompt2,
                        ..Default::default()
                    },
                ]);
                assert!(params.validate_async().await.is_err());
            }
        }
    }

    // =========================================================================
    // EncodeVibeParams Tests
    // =========================================================================
    mod encode_vibe_params {
        use super::*;

        #[test]
        fn should_validate_minimal_params_image_only() {
            let params = EncodeVibeParams {
                image: ImageInput::FilePath("test.png".into()),
                ..Default::default()
            };
            assert!(params.validate().is_ok());
        }

        #[test]
        fn should_apply_defaults_correctly() {
            let params = EncodeVibeParams {
                image: ImageInput::FilePath("test.png".into()),
                ..Default::default()
            };
            assert_eq!(params.model, Model::default());
            assert!((params.information_extracted - 0.7).abs() < f64::EPSILON);
            assert!((params.strength - 0.7).abs() < f64::EPSILON);
        }

        #[test]
        fn should_accept_bytes_as_image() {
            let params = EncodeVibeParams {
                image: ImageInput::Bytes(vec![1, 2, 3]),
                ..Default::default()
            };
            assert!(params.validate().is_ok());
        }

        #[test]
        fn should_reject_empty_string_image() {
            let params = EncodeVibeParams {
                image: ImageInput::FilePath("".into()),
                ..Default::default()
            };
            assert!(params.validate().is_err());
        }

        mod information_extracted {
            use super::*;

            #[test]
            fn should_accept_valid_range() {
                for val in [0.0, 1.0, 0.5] {
                    let params = EncodeVibeParams {
                        image: ImageInput::FilePath("test.png".into()),
                        information_extracted: val,
                        ..Default::default()
                    };
                    assert!(params.validate().is_ok());
                }
            }

            #[test]
            fn should_reject_out_of_range() {
                for val in [-0.1, 1.1] {
                    let params = EncodeVibeParams {
                        image: ImageInput::FilePath("test.png".into()),
                        information_extracted: val,
                        ..Default::default()
                    };
                    assert!(params.validate().is_err());
                }
            }
        }

        mod strength {
            use super::*;

            #[test]
            fn should_accept_valid_range() {
                for val in [0.0, 1.0] {
                    let params = EncodeVibeParams {
                        image: ImageInput::FilePath("test.png".into()),
                        strength: val,
                        ..Default::default()
                    };
                    assert!(params.validate().is_ok());
                }
            }

            #[test]
            fn should_reject_out_of_range() {
                for val in [-0.1, 1.1] {
                    let params = EncodeVibeParams {
                        image: ImageInput::FilePath("test.png".into()),
                        strength: val,
                        ..Default::default()
                    };
                    assert!(params.validate().is_err());
                }
            }
        }

        mod save_target_validation {
            use super::*;

            // Mutual exclusion test removed: SaveTarget enum makes it impossible.

            #[test]
            fn should_reject_path_traversal_in_save_path() {
                let params = EncodeVibeParams {
                    image: ImageInput::FilePath("test.png".into()),
                    save: SaveTarget::ExactPath("../etc/passwd".to_string()),
                    ..Default::default()
                };
                let err = params.validate().unwrap_err();
                assert!(err.to_string().contains("path traversal"));
            }

            #[test]
            fn should_accept_exact_path() {
                let params = EncodeVibeParams {
                    image: ImageInput::FilePath("test.png".into()),
                    save: SaveTarget::ExactPath("/path/to/file.naiv4vibe".to_string()),
                    ..Default::default()
                };
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_accept_directory_with_filename() {
                let params = EncodeVibeParams {
                    image: ImageInput::FilePath("test.png".into()),
                    save: SaveTarget::Directory {
                        dir: "./vibes/".to_string(),
                        filename: Some("my_custom_vibe".to_string()),
                    },
                    ..Default::default()
                };
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_accept_directory_without_filename() {
                let params = EncodeVibeParams {
                    image: ImageInput::FilePath("test.png".into()),
                    save: SaveTarget::Directory {
                        dir: "./vibes/".to_string(),
                        filename: None,
                    },
                    ..Default::default()
                };
                assert!(params.validate().is_ok());
            }
        }

        mod model_validation {
            use super::*;

            #[test]
            fn should_accept_all_valid_models() {
                for model in [
                    Model::NaiDiffusion4CuratedPreview,
                    Model::NaiDiffusion4Full,
                    Model::NaiDiffusion45Curated,
                    Model::NaiDiffusion45Full,
                ] {
                    let params = EncodeVibeParams {
                        image: ImageInput::FilePath("test.png".into()),
                        model,
                        ..Default::default()
                    };
                    assert!(params.validate().is_ok());
                }
            }

            // Invalid model is handled by Rust type system (enum)
        }
    }

    // =========================================================================
    // Helper Functions Tests
    // =========================================================================
    mod helper_functions {
        use super::*;

        #[test]
        fn character_to_caption_dict_converts_correctly() {
            let config = CharacterConfig {
                prompt: "1girl, red hair".to_string(),
                center_x: 0.3,
                center_y: 0.7,
                negative_prompt: "".to_string(),
            };
            let result = character_to_caption_dict(&config);
            assert_eq!(result.char_caption, "1girl, red hair");
            assert_eq!(result.centers.len(), 1);
            assert!((result.centers[0].x - 0.3).abs() < f64::EPSILON);
            assert!((result.centers[0].y - 0.7).abs() < f64::EPSILON);
        }

        #[test]
        fn character_to_negative_caption_dict_converts_correctly() {
            let config = CharacterConfig {
                prompt: "1girl".to_string(),
                center_x: 0.5,
                center_y: 0.5,
                negative_prompt: "lowres, bad anatomy".to_string(),
            };
            let result = character_to_negative_caption_dict(&config);
            assert_eq!(result.char_caption, "lowres, bad anatomy");
            assert_eq!(result.centers.len(), 1);
            assert!((result.centers[0].x - 0.5).abs() < f64::EPSILON);
            assert!((result.centers[0].y - 0.5).abs() < f64::EPSILON);
        }
    }

    // =========================================================================
    // AugmentParams Tests
    // =========================================================================
    mod augment_params {
        use super::*;

        mod basic {
            use super::*;

            #[test]
            fn should_validate_minimal_params_for_declutter() {
                let params = AugmentParams {
                    req_type: AugmentReqType::Declutter,
                    image: ImageInput::FilePath("test.png".into()),
                    prompt: None,
                    defry: None,
                    save: SaveTarget::None,
                };
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_accept_bytes_as_image() {
                let params = AugmentParams {
                    req_type: AugmentReqType::Sketch,
                    image: ImageInput::Bytes(vec![1, 2, 3]),
                    prompt: None,
                    defry: None,
                    save: SaveTarget::None,
                };
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_reject_empty_string_image() {
                let params = AugmentParams {
                    req_type: AugmentReqType::Sketch,
                    image: ImageInput::FilePath("".into()),
                    prompt: None,
                    defry: None,
                    save: SaveTarget::None,
                };
                assert!(params.validate().is_err());
            }
        }

        mod req_type {
            use super::*;

            #[test]
            fn should_accept_all_valid_simple_req_types() {
                let simple_types = [
                    AugmentReqType::Declutter,
                    AugmentReqType::Sketch,
                    AugmentReqType::Lineart,
                    AugmentReqType::BgRemoval,
                ];
                for req_type in simple_types {
                    let params = AugmentParams {
                        req_type,
                        image: ImageInput::FilePath("test.png".into()),
                        prompt: None,
                        defry: None,
                        save: SaveTarget::None,
                    };
                    assert!(params.validate().is_ok());
                }
            }

            #[test]
            fn should_accept_colorize_with_defry() {
                let params = AugmentParams {
                    req_type: AugmentReqType::Colorize,
                    image: ImageInput::FilePath("test.png".into()),
                    prompt: None,
                    defry: Some(3),
                    save: SaveTarget::None,
                };
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_accept_emotion_with_defry_and_prompt() {
                let params = AugmentParams {
                    req_type: AugmentReqType::Emotion,
                    image: ImageInput::FilePath("test.png".into()),
                    prompt: Some("happy".to_string()),
                    defry: Some(2),
                    save: SaveTarget::None,
                };
                assert!(params.validate().is_ok());
            }

            // Invalid req_type is handled by Rust type system (enum)
        }

        mod colorize {
            use super::*;

            #[test]
            fn should_require_defry_for_colorize() {
                let params = AugmentParams {
                    req_type: AugmentReqType::Colorize,
                    image: ImageInput::FilePath("test.png".into()),
                    prompt: None,
                    defry: None,
                    save: SaveTarget::None,
                };
                let err = params.validate().unwrap_err();
                assert!(err.to_string().contains("defry (0-5) is required for colorize"));
            }

            #[test]
            fn should_accept_colorize_with_defry_and_optional_prompt() {
                // with prompt
                let params1 = AugmentParams {
                    req_type: AugmentReqType::Colorize,
                    image: ImageInput::FilePath("test.png".into()),
                    prompt: Some("vibrant colors".to_string()),
                    defry: Some(3),
                    save: SaveTarget::None,
                };
                assert!(params1.validate().is_ok());

                // without prompt
                let params2 = AugmentParams {
                    req_type: AugmentReqType::Colorize,
                    image: ImageInput::FilePath("test.png".into()),
                    prompt: None,
                    defry: Some(0),
                    save: SaveTarget::None,
                };
                assert!(params2.validate().is_ok());
            }
        }

        mod emotion {
            use super::*;

            #[test]
            fn should_require_defry_for_emotion() {
                let params = AugmentParams {
                    req_type: AugmentReqType::Emotion,
                    image: ImageInput::FilePath("test.png".into()),
                    prompt: Some("happy".to_string()),
                    defry: None,
                    save: SaveTarget::None,
                };
                let err = params.validate().unwrap_err();
                assert!(err.to_string().contains("defry (0-5) is required for emotion"));
            }

            #[test]
            fn should_require_prompt_for_emotion() {
                let params = AugmentParams {
                    req_type: AugmentReqType::Emotion,
                    image: ImageInput::FilePath("test.png".into()),
                    prompt: None,
                    defry: Some(2),
                    save: SaveTarget::None,
                };
                let err = params.validate().unwrap_err();
                assert!(err.to_string().contains("prompt is required for emotion"));
            }

            #[test]
            fn should_accept_all_valid_emotion_keywords() {
                for keyword in EMOTION_KEYWORDS {
                    let params = AugmentParams {
                        req_type: AugmentReqType::Emotion,
                        image: ImageInput::FilePath("test.png".into()),
                        prompt: Some(keyword.to_string()),
                        defry: Some(2),
                        save: SaveTarget::None,
                    };
                    assert!(
                        params.validate().is_ok(),
                        "Expected emotion keyword '{}' to be valid",
                        keyword
                    );
                }
            }

            #[test]
            fn should_reject_invalid_emotion_keyword() {
                let params = AugmentParams {
                    req_type: AugmentReqType::Emotion,
                    image: ImageInput::FilePath("test.png".into()),
                    prompt: Some("invalid_emotion".to_string()),
                    defry: Some(2),
                    save: SaveTarget::None,
                };
                let err = params.validate().unwrap_err();
                assert!(err.to_string().contains("Invalid emotion keyword"));
            }

            #[test]
            fn should_reject_emotion_keyword_with_trailing_semicolons() {
                let params = AugmentParams {
                    req_type: AugmentReqType::Emotion,
                    image: ImageInput::FilePath("test.png".into()),
                    prompt: Some("happy;;".to_string()),
                    defry: Some(2),
                    save: SaveTarget::None,
                };
                let err = params.validate().unwrap_err();
                assert!(err.to_string().contains("Invalid emotion keyword"));
            }
        }

        mod simple_types_reject_extra_params {
            use super::*;

            #[test]
            fn should_reject_prompt_for_simple_types() {
                let simple_types = [
                    AugmentReqType::Declutter,
                    AugmentReqType::Sketch,
                    AugmentReqType::Lineart,
                    AugmentReqType::BgRemoval,
                ];
                for req_type in simple_types {
                    let params = AugmentParams {
                        req_type,
                        image: ImageInput::FilePath("test.png".into()),
                        prompt: Some("should not be here".to_string()),
                        defry: None,
                        save: SaveTarget::None,
                    };
                    let err = params.validate().unwrap_err();
                    assert!(
                        err.to_string().contains(&format!(
                            "prompt cannot be used with {}",
                            req_type.as_str()
                        )),
                        "Expected error for prompt with {}, got: {}",
                        req_type.as_str(),
                        err
                    );
                }
            }

            #[test]
            fn should_reject_defry_for_simple_types() {
                let simple_types = [
                    AugmentReqType::Declutter,
                    AugmentReqType::Sketch,
                    AugmentReqType::Lineart,
                    AugmentReqType::BgRemoval,
                ];
                for req_type in simple_types {
                    let params = AugmentParams {
                        req_type,
                        image: ImageInput::FilePath("test.png".into()),
                        prompt: None,
                        defry: Some(3),
                        save: SaveTarget::None,
                    };
                    let err = params.validate().unwrap_err();
                    assert!(
                        err.to_string().contains(&format!(
                            "defry cannot be used with {}",
                            req_type.as_str()
                        )),
                        "Expected error for defry with {}, got: {}",
                        req_type.as_str(),
                        err
                    );
                }
            }
        }

        mod defry_validation {
            use super::*;

            #[test]
            fn should_accept_valid_defry_range_0_to_5_for_colorize() {
                for i in 0..=5 {
                    let params = AugmentParams {
                        req_type: AugmentReqType::Colorize,
                        image: ImageInput::FilePath("test.png".into()),
                        prompt: None,
                        defry: Some(i),
                        save: SaveTarget::None,
                    };
                    assert!(params.validate().is_ok());
                }
            }

            // SKIP: defry below 0 (-1) - Rust type system handles this (u32)

            #[test]
            fn should_reject_defry_above_5() {
                let params = AugmentParams {
                    req_type: AugmentReqType::Colorize,
                    image: ImageInput::FilePath("test.png".into()),
                    prompt: None,
                    defry: Some(6),
                    save: SaveTarget::None,
                };
                assert!(params.validate().is_err());
            }

            // SKIP: non-integer defry (2.5) - Rust type system handles this (u32)
        }

        mod save_target_validation {
            use super::*;

            // Mutual exclusion test removed: SaveTarget enum makes it impossible.

            #[test]
            fn should_accept_exact_path() {
                let params = AugmentParams {
                    req_type: AugmentReqType::Sketch,
                    image: ImageInput::FilePath("test.png".into()),
                    prompt: None,
                    defry: None,
                    save: SaveTarget::ExactPath("/path/to/file.png".to_string()),
                };
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_accept_directory() {
                let params = AugmentParams {
                    req_type: AugmentReqType::Sketch,
                    image: ImageInput::FilePath("test.png".into()),
                    prompt: None,
                    defry: None,
                    save: SaveTarget::Directory {
                        dir: "/path/to/dir/".to_string(),
                        filename: None,
                    },
                };
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_reject_path_traversal_in_save_path() {
                let params = AugmentParams {
                    req_type: AugmentReqType::Sketch,
                    image: ImageInput::FilePath("test.png".into()),
                    prompt: None,
                    defry: None,
                    save: SaveTarget::ExactPath("../etc/passwd".to_string()),
                };
                let err = params.validate().unwrap_err();
                assert!(err.to_string().contains("path traversal"));
            }
        }
    }

    // =========================================================================
    // UpscaleParams Tests
    // =========================================================================
    mod upscale_params {
        use super::*;

        mod basic {
            use super::*;

            #[test]
            fn should_validate_minimal_params() {
                let params = UpscaleParams {
                    image: ImageInput::FilePath("test.png".into()),
                    ..Default::default()
                };
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_apply_default_scale_value_4() {
                let params = UpscaleParams {
                    image: ImageInput::FilePath("test.png".into()),
                    ..Default::default()
                };
                assert_eq!(params.scale, DEFAULT_UPSCALE_SCALE);
            }

            #[test]
            fn should_accept_bytes_as_image() {
                let params = UpscaleParams {
                    image: ImageInput::Bytes(vec![1, 2, 3]),
                    ..Default::default()
                };
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_reject_empty_string_image() {
                let params = UpscaleParams {
                    image: ImageInput::FilePath("".into()),
                    ..Default::default()
                };
                assert!(params.validate().is_err());
            }
        }

        mod scale_validation {
            use super::*;

            #[test]
            fn should_accept_scale_2() {
                let params = UpscaleParams {
                    image: ImageInput::FilePath("test.png".into()),
                    scale: 2,
                    ..Default::default()
                };
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_accept_scale_4() {
                let params = UpscaleParams {
                    image: ImageInput::FilePath("test.png".into()),
                    scale: 4,
                    ..Default::default()
                };
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_reject_invalid_scale_values() {
                for scale in [1, 3, 5, 8, 0] {
                    let params = UpscaleParams {
                        image: ImageInput::FilePath("test.png".into()),
                        scale,
                        ..Default::default()
                    };
                    assert!(
                        params.validate().is_err(),
                        "Expected scale {} to be invalid",
                        scale
                    );
                }
            }

            // SKIP: non-integer scale (2.5) - Rust type system handles this (u32)
        }

        mod save_target_validation {
            use super::*;

            // Mutual exclusion test removed: SaveTarget enum makes it impossible.

            #[test]
            fn should_accept_exact_path() {
                let params = UpscaleParams {
                    image: ImageInput::FilePath("test.png".into()),
                    save: SaveTarget::ExactPath("/path/to/file.png".to_string()),
                    ..Default::default()
                };
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_accept_directory() {
                let params = UpscaleParams {
                    image: ImageInput::FilePath("test.png".into()),
                    save: SaveTarget::Directory {
                        dir: "/path/to/dir/".to_string(),
                        filename: None,
                    },
                    ..Default::default()
                };
                assert!(params.validate().is_ok());
            }

            #[test]
            fn should_reject_path_traversal_in_save_path() {
                let params = UpscaleParams {
                    image: ImageInput::FilePath("test.png".into()),
                    save: SaveTarget::ExactPath("../etc/passwd".to_string()),
                    ..Default::default()
                };
                let err = params.validate().unwrap_err();
                assert!(err.to_string().contains("path traversal"));
            }
        }
    }

    // =========================================================================
    // UpscaleResult Tests
    // =========================================================================
    mod upscale_result {
        use super::*;

        #[test]
        fn should_accept_valid_result_with_integer_scale_and_dimensions() {
            let result = UpscaleResult {
                image_data: vec![1, 2, 3],
                scale: 4,
                output_width: 2048,
                output_height: 1536,
                anlas_remaining: None,
                anlas_consumed: None,
                saved_path: None,
            };
            assert!(result.validate().is_ok());
        }

        // SKIP: non-integer scale (2.5) - Rust type system handles this (u32)

        #[test]
        fn should_reject_invalid_scale_value() {
            let result = UpscaleResult {
                image_data: vec![1, 2, 3],
                scale: 3,
                output_width: 2048,
                output_height: 1536,
                anlas_remaining: None,
                anlas_consumed: None,
                saved_path: None,
            };
            assert!(result.validate().is_err());
        }

        #[test]
        fn should_reject_zero_output_width() {
            let result = UpscaleResult {
                image_data: vec![1, 2, 3],
                scale: 4,
                output_width: 0,
                output_height: 1536,
                anlas_remaining: None,
                anlas_consumed: None,
                saved_path: None,
            };
            assert!(result.validate().is_err());
        }

        #[test]
        fn should_reject_zero_output_height() {
            let result = UpscaleResult {
                image_data: vec![1, 2, 3],
                scale: 4,
                output_width: 2048,
                output_height: 0,
                anlas_remaining: None,
                anlas_consumed: None,
                saved_path: None,
            };
            assert!(result.validate().is_err());
        }

        #[test]
        fn should_accept_scale_2_result() {
            let result = UpscaleResult {
                image_data: vec![1, 2, 3],
                scale: 2,
                output_width: 1024,
                output_height: 768,
                anlas_remaining: None,
                anlas_consumed: None,
                saved_path: None,
            };
            assert!(result.validate().is_ok());
        }
    }

    // =========================================================================
    // GenerateResult Tests
    // =========================================================================
    mod generate_result {
        use super::*;

        #[test]
        fn should_accept_valid_result() {
            let result = GenerateResult {
                image_data: vec![1, 2, 3],
                seed: 12345,
                anlas_remaining: None,
                anlas_consumed: None,
                saved_path: None,
            };
            assert!(result.validate().is_ok());
        }

        #[test]
        fn should_accept_result_with_bytes_image_data() {
            let result = GenerateResult {
                image_data: vec![1, 2, 3],
                seed: 12345,
                anlas_remaining: None,
                anlas_consumed: None,
                saved_path: None,
            };
            assert!(result.validate().is_ok());
        }

        // SKIP: non-integer seed (123.45) - Rust type system handles this (u64)

        #[test]
        fn should_reject_empty_image_data() {
            let result = GenerateResult {
                image_data: vec![],
                seed: 12345,
                anlas_remaining: None,
                anlas_consumed: None,
                saved_path: None,
            };
            assert!(result.validate().is_err());
        }

        #[test]
        fn should_accept_max_valid_seed() {
            let result = GenerateResult {
                image_data: vec![1, 2, 3],
                seed: MAX_SEED as u64,
                anlas_remaining: None,
                anlas_consumed: None,
                saved_path: None,
            };
            assert!(result.validate().is_ok());
        }

        #[test]
        fn should_reject_seed_above_max() {
            let result = GenerateResult {
                image_data: vec![1, 2, 3],
                seed: MAX_SEED as u64 + 1,
                anlas_remaining: None,
                anlas_consumed: None,
                saved_path: None,
            };
            assert!(result.validate().is_err());
        }
    }
}
