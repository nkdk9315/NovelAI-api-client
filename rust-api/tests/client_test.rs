//! NovelAI Client Tests
//! Tests for response parsing, payload building, client construction,
//! and HTTP-mocked integration tests.

use std::io::Write;

use base64::Engine;
use novelai_api::client::payload;
use novelai_api::client::response;
use novelai_api::client::{DefaultLogger, NovelAIClient};
use novelai_api::constants;
use novelai_api::error::NovelAIError;
use novelai_api::schemas::*;

// =============================================================================
// Test Helpers
// =============================================================================

/// Create a minimal valid PNG image.
fn create_test_png(width: u32, height: u32) -> Vec<u8> {
    let img = image::RgbaImage::new(width, height);
    let dynamic = image::DynamicImage::ImageRgba8(img);
    let mut buf = std::io::Cursor::new(Vec::new());
    dynamic
        .write_to(&mut buf, image::ImageFormat::Png)
        .unwrap();
    buf.into_inner()
}

/// Create a ZIP archive containing a single PNG image.
fn create_test_zip_with_png(png_data: &[u8]) -> Vec<u8> {
    let buf = std::io::Cursor::new(Vec::new());
    let mut writer = zip::ZipWriter::new(buf);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);
    writer.start_file("image.png", options).unwrap();
    writer.write_all(png_data).unwrap();
    let cursor = writer.finish().unwrap();
    cursor.into_inner()
}

/// Create a ZIP with a non-image file only.
fn create_test_zip_no_image() -> Vec<u8> {
    let buf = std::io::Cursor::new(Vec::new());
    let mut writer = zip::ZipWriter::new(buf);
    let options = zip::write::SimpleFileOptions::default();
    writer.start_file("readme.txt", options).unwrap();
    writer.write_all(b"hello world").unwrap();
    let cursor = writer.finish().unwrap();
    cursor.into_inner()
}

/// Create a ZIP with too many entries.
fn create_test_zip_too_many_entries() -> Vec<u8> {
    let buf = std::io::Cursor::new(Vec::new());
    let mut writer = zip::ZipWriter::new(buf);
    let options = zip::write::SimpleFileOptions::default();
    for i in 0..=constants::MAX_ZIP_ENTRIES {
        writer
            .start_file(format!("file_{}.txt", i), options)
            .unwrap();
        writer.write_all(b"data").unwrap();
    }
    let cursor = writer.finish().unwrap();
    cursor.into_inner()
}

/// Create a msgpack response with binary data under the given key.
fn create_test_msgpack(key: &str, data: &[u8]) -> Vec<u8> {
    let val = rmpv::Value::Map(vec![(
        rmpv::Value::String(key.into()),
        rmpv::Value::Binary(data.to_vec()),
    )]);
    let mut buf = Vec::new();
    rmpv::encode::write_value(&mut buf, &val).unwrap();
    buf
}

// =============================================================================
// Response Parsing Tests
// =============================================================================

mod response_parsing {
    use super::*;

    #[test]
    fn parse_zip_response_with_valid_png() {
        let png = create_test_png(64, 64);
        let zip = create_test_zip_with_png(&png);
        let result = response::parse_zip_response(&zip).unwrap();
        assert_eq!(result, png);
    }

    #[test]
    fn parse_zip_response_no_image_file() {
        let zip = create_test_zip_no_image();
        let result = response::parse_zip_response(&zip);
        assert!(result.is_err());
        match result.unwrap_err() {
            NovelAIError::Parse(msg) => assert!(msg.contains("No image found")),
            e => panic!("Expected Parse error, got: {:?}", e),
        }
    }

    #[test]
    fn parse_zip_response_too_many_entries() {
        let zip = create_test_zip_too_many_entries();
        let result = response::parse_zip_response(&zip);
        assert!(result.is_err());
        match result.unwrap_err() {
            NovelAIError::Parse(msg) => assert!(msg.contains("Too many ZIP entries")),
            e => panic!("Expected Parse error, got: {:?}", e),
        }
    }

    #[test]
    fn parse_zip_response_invalid_data() {
        let result = response::parse_zip_response(b"not a zip file");
        assert!(result.is_err());
        match result.unwrap_err() {
            NovelAIError::Parse(msg) => assert!(msg.contains("Failed to open ZIP")),
            e => panic!("Expected Parse error, got: {:?}", e),
        }
    }

    #[test]
    fn parse_zip_response_with_webp_extension() {
        let buf = std::io::Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(buf);
        let options = zip::write::SimpleFileOptions::default();
        writer.start_file("image.webp", options).unwrap();
        let test_data = b"fake-webp-data";
        writer.write_all(test_data).unwrap();
        let cursor = writer.finish().unwrap();
        let zip = cursor.into_inner();

        let result = response::parse_zip_response(&zip).unwrap();
        assert_eq!(result, test_data);
    }

    // -------------------------------------------------------------------------
    // Stream response parsing
    // -------------------------------------------------------------------------

    #[test]
    fn parse_stream_response_png_signature() {
        let png = create_test_png(32, 32);
        let logger = DefaultLogger;
        let result = response::parse_stream_response(&png, &logger).unwrap();
        assert_eq!(result, png);
    }

    #[test]
    fn parse_stream_response_zip_signature() {
        let png = create_test_png(32, 32);
        let zip = create_test_zip_with_png(&png);
        let logger = DefaultLogger;
        let result = response::parse_stream_response(&zip, &logger).unwrap();
        assert_eq!(result, png);
    }

    #[test]
    fn parse_stream_response_msgpack_data_key() {
        let png = create_test_png(32, 32);
        let msgpack = create_test_msgpack("data", &png);
        let logger = DefaultLogger;
        let result = response::parse_stream_response(&msgpack, &logger).unwrap();
        assert_eq!(result, png);
    }

    #[test]
    fn parse_stream_response_msgpack_image_key() {
        let png = create_test_png(32, 32);
        let msgpack = create_test_msgpack("image", &png);
        let logger = DefaultLogger;
        let result = response::parse_stream_response(&msgpack, &logger).unwrap();
        assert_eq!(result, png);
    }

    #[test]
    fn parse_stream_response_embedded_png_after_garbage() {
        let png = create_test_png(32, 32);
        let mut data = vec![0x00, 0x01, 0x02, 0x03]; // garbage prefix
        data.extend_from_slice(&png);
        let logger = DefaultLogger;
        let result = response::parse_stream_response(&data, &logger).unwrap();
        // Should find and extract the PNG starting from its signature
        assert!(!result.is_empty());
        assert_eq!(
            &result[..8],
            &[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]
        );
    }

    #[test]
    fn parse_stream_response_streaming_msgpack_preview_then_png() {
        // Simulate streaming format: [msgpack_preview][full_PNG]
        // The msgpack contains a small "data" field (preview), and the
        // trailing bytes are the full-resolution PNG. The parser should
        // return the trailing PNG, not the msgpack preview data.
        let small_png = create_test_png(8, 8); // preview (small)
        let full_png = create_test_png(64, 64); // full resolution

        // Build msgpack preview message with "data" key
        let msgpack_preview = create_test_msgpack("data", &small_png);

        // Concatenate: [msgpack_preview][full_PNG]
        let mut stream_data = msgpack_preview;
        stream_data.extend_from_slice(&full_png);

        let logger = DefaultLogger;
        let result = response::parse_stream_response(&stream_data, &logger).unwrap();
        assert_eq!(result, full_png);
    }

    #[test]
    fn parse_stream_response_multiple_msgpack_previews_then_png() {
        // Simulate: [msgpack_preview_1][msgpack_preview_2][full_PNG]
        let preview1 = create_test_png(4, 4);
        let preview2 = create_test_png(8, 8);
        let full_png = create_test_png(64, 64);

        let mut stream_data = create_test_msgpack("data", &preview1);
        stream_data.extend_from_slice(&create_test_msgpack("data", &preview2));
        stream_data.extend_from_slice(&full_png);

        let logger = DefaultLogger;
        let result = response::parse_stream_response(&stream_data, &logger).unwrap();
        assert_eq!(result, full_png);
    }

    #[test]
    fn parse_stream_response_unparseable() {
        let data = vec![0x00, 0x01, 0x02, 0x03, 0x04];
        let logger = DefaultLogger;
        let result = response::parse_stream_response(&data, &logger);
        assert!(result.is_err());
        match result.unwrap_err() {
            NovelAIError::Parse(msg) => assert!(msg.contains("Cannot parse stream response")),
            e => panic!("Expected Parse error, got: {:?}", e),
        }
    }

    #[test]
    fn parse_stream_response_empty() {
        let logger = DefaultLogger;
        let result = response::parse_stream_response(&[], &logger);
        assert!(result.is_err());
    }
}

// =============================================================================
// Payload Building Tests
// =============================================================================

mod payload_building {
    use super::*;

    #[test]
    fn build_base_payload_structure() {
        let params = GenerateParams {
            prompt: "1girl, flower".to_string(),
            width: 832,
            height: 1216,
            steps: 23,
            scale: 5.0,
            ..Default::default()
        };
        let payload = payload::build_base_payload(&params, 12345, "bad quality");

        assert_eq!(payload["input"], "1girl, flower");
        assert_eq!(payload["model"], "nai-diffusion-4-5-full");
        assert_eq!(payload["action"], "generate");
        assert_eq!(payload["parameters"]["width"], 832);
        assert_eq!(payload["parameters"]["height"], 1216);
        assert_eq!(payload["parameters"]["steps"], 23);
        assert_eq!(payload["parameters"]["scale"], 5.0);
        assert_eq!(payload["parameters"]["seed"], 12345);
        assert_eq!(payload["parameters"]["negative_prompt"], "bad quality");
        assert_eq!(payload["parameters"]["params_version"], 3);
        assert_eq!(payload["use_new_shared_trial"], true);
    }

    #[test]
    fn build_base_payload_with_different_model() {
        let params = GenerateParams {
            prompt: "test".to_string(),
            model: constants::Model::NaiDiffusion4Full,
            ..Default::default()
        };
        let payload = payload::build_base_payload(&params, 0, "");
        assert_eq!(payload["model"], "nai-diffusion-4-full");
    }

    #[test]
    fn build_base_payload_with_different_sampler() {
        let params = GenerateParams {
            prompt: "test".to_string(),
            sampler: constants::Sampler::KDpmpp2mSde,
            noise_schedule: constants::NoiseSchedule::Exponential,
            ..Default::default()
        };
        let payload = payload::build_base_payload(&params, 0, "");
        assert_eq!(payload["parameters"]["sampler"], "k_dpmpp_2m_sde");
        assert_eq!(payload["parameters"]["noise_schedule"], "exponential");
    }

    #[test]
    fn build_base_payload_img2img_action() {
        let params = GenerateParams {
            prompt: "test".to_string(),
            action: GenerateAction::Img2Img {
                source_image: ImageInput::Bytes(vec![0]),
                strength: 0.7,
                noise: 0.0,
            },
            ..Default::default()
        };
        let payload = payload::build_base_payload(&params, 0, "");
        assert_eq!(payload["action"], "img2img");
    }

    #[test]
    fn apply_vibe_params_adds_fields() {
        let params = GenerateParams::default();
        let mut payload = payload::build_base_payload(&params, 0, "");

        let encodings = vec!["enc1".to_string(), "enc2".to_string()];
        let strengths = Some(vec![0.7, 0.8]);
        let info_list = vec![0.5, 0.6];

        payload::apply_vibe_params(&mut payload, &encodings, &strengths, &info_list);

        let refs = &payload["parameters"]["reference_image_multiple"];
        assert_eq!(refs.as_array().unwrap().len(), 2);
        assert_eq!(refs[0], "enc1");
        assert_eq!(refs[1], "enc2");

        let strengths_val = &payload["parameters"]["reference_strength_multiple"];
        assert_eq!(strengths_val[0], 0.7);
        assert_eq!(strengths_val[1], 0.8);

        let info_val = &payload["parameters"]["reference_information_extracted_multiple"];
        assert_eq!(info_val[0], 0.5);
        assert_eq!(info_val[1], 0.6);
    }

    #[test]
    fn apply_vibe_params_noop_when_empty() {
        let params = GenerateParams::default();
        let mut payload = payload::build_base_payload(&params, 0, "");
        payload::apply_vibe_params(&mut payload, &[], &None, &[]);
        assert!(payload["parameters"]["reference_image_multiple"].is_null());
    }

    #[test]
    fn build_v4_prompt_structure_basic() {
        let params = GenerateParams::default();
        let mut payload = payload::build_base_payload(&params, 0, "");

        payload::build_v4_prompt_structure(&mut payload, "1girl", "bad quality", &[], &[]);

        let v4p = &payload["parameters"]["v4_prompt"];
        assert_eq!(v4p["caption"]["base_caption"], "1girl");
        assert_eq!(v4p["use_coords"], true);
        assert_eq!(v4p["use_order"], true);
        assert_eq!(
            v4p["caption"]["char_captions"]
                .as_array()
                .unwrap()
                .len(),
            0
        );

        let v4n = &payload["parameters"]["v4_negative_prompt"];
        assert_eq!(v4n["caption"]["base_caption"], "bad quality");
        assert_eq!(v4n["legacy_uc"], false);
    }

    #[test]
    fn build_v4_prompt_with_characters() {
        let params = GenerateParams::default();
        let mut payload = payload::build_base_payload(&params, 0, "");

        let char_captions = vec![
            CaptionDict {
                char_caption: "1girl, blue hair".to_string(),
                centers: vec![CaptionCenter { x: 0.3, y: 0.5 }],
            },
            CaptionDict {
                char_caption: "1boy, red hair".to_string(),
                centers: vec![CaptionCenter { x: 0.7, y: 0.5 }],
            },
        ];
        let char_neg = vec![
            CaptionDict {
                char_caption: "".to_string(),
                centers: vec![CaptionCenter { x: 0.3, y: 0.5 }],
            },
            CaptionDict {
                char_caption: "".to_string(),
                centers: vec![CaptionCenter { x: 0.7, y: 0.5 }],
            },
        ];

        payload::build_v4_prompt_structure(
            &mut payload,
            "2people",
            "bad quality",
            &char_captions,
            &char_neg,
        );

        let chars = &payload["parameters"]["v4_prompt"]["caption"]["char_captions"];
        assert_eq!(chars.as_array().unwrap().len(), 2);
        assert_eq!(chars[0]["char_caption"], "1girl, blue hair");
        assert_eq!(chars[0]["centers"][0]["x"], 0.3);
        assert_eq!(chars[1]["char_caption"], "1boy, red hair");
        assert_eq!(chars[1]["centers"][0]["x"], 0.7);
    }

    #[test]
    fn apply_character_prompts_basic() {
        let params = GenerateParams::default();
        let mut payload = payload::build_base_payload(&params, 0, "");

        let configs = vec![CharacterConfig {
            prompt: "1girl, blue hair".to_string(),
            center_x: 0.3,
            center_y: 0.5,
            negative_prompt: "bad".to_string(),
        }];

        payload::apply_character_prompts(&mut payload, &configs);

        assert_eq!(payload["parameters"]["use_coords"], true);
        let prompts = &payload["parameters"]["characterPrompts"];
        assert_eq!(prompts.as_array().unwrap().len(), 1);
        assert_eq!(prompts[0]["prompt"], "1girl, blue hair");
        assert_eq!(prompts[0]["uc"], "bad");
        assert_eq!(prompts[0]["center"]["x"], 0.3);
        assert_eq!(prompts[0]["center"]["y"], 0.5);
        assert_eq!(prompts[0]["enabled"], true);
    }

    #[test]
    fn apply_character_prompts_noop_when_empty() {
        let params = GenerateParams::default();
        let mut payload = payload::build_base_payload(&params, 0, "");
        payload::apply_character_prompts(&mut payload, &[]);
        assert!(payload["parameters"]["characterPrompts"].is_null());
    }

    #[test]
    fn apply_character_prompts_multiple() {
        let params = GenerateParams::default();
        let mut payload = payload::build_base_payload(&params, 0, "");

        let configs = vec![
            CharacterConfig {
                prompt: "char1".to_string(),
                center_x: 0.2,
                center_y: 0.3,
                negative_prompt: "bad1".to_string(),
            },
            CharacterConfig {
                prompt: "char2".to_string(),
                center_x: 0.8,
                center_y: 0.7,
                negative_prompt: "bad2".to_string(),
            },
        ];

        payload::apply_character_prompts(&mut payload, &configs);

        let prompts = &payload["parameters"]["characterPrompts"];
        assert_eq!(prompts.as_array().unwrap().len(), 2);
        assert_eq!(prompts[0]["prompt"], "char1");
        assert_eq!(prompts[1]["prompt"], "char2");
    }

    #[test]
    fn build_base_payload_includes_safety_params() {
        let params = GenerateParams::default();
        let payload = payload::build_base_payload(&params, 999, "neg");

        let p = &payload["parameters"];
        assert_eq!(p["qualityToggle"], true);
        assert_eq!(p["legacy"], false);
        assert_eq!(p["legacy_v3_extend"], false);
        assert_eq!(p["deliberate_euler_ancestral_bug"], false);
        assert_eq!(p["prefer_brownian"], true);
        assert_eq!(p["n_samples"], 1);
        assert_eq!(p["ucPreset"], 0);
    }
}

// =============================================================================
// Client Construction Tests
// =============================================================================

mod client_construction {
    use super::*;

    #[test]
    fn new_with_api_key() {
        let client = NovelAIClient::new(Some("test-key-123"), None);
        assert!(client.is_ok());
    }

    #[test]
    fn new_with_empty_api_key_fails() {
        std::env::remove_var("NOVELAI_API_KEY");
        let result = NovelAIClient::new(Some(""), None);
        assert!(result.is_err());
        let err = result.err().unwrap();
        match err {
            NovelAIError::Validation(msg) => assert!(msg.contains("API key is required")),
            e => panic!("Expected Validation error, got: {:?}", e),
        }
    }

    #[test]
    fn new_without_api_key_no_env_fails() {
        std::env::remove_var("NOVELAI_API_KEY");
        let result = NovelAIClient::new(None, None);
        assert!(result.is_err());
        let err = result.err().unwrap();
        match err {
            NovelAIError::Validation(msg) => assert!(msg.contains("API key is required")),
            e => panic!("Expected Validation error, got: {:?}", e),
        }
    }

    #[test]
    fn new_with_custom_logger() {
        struct TestLogger;
        impl novelai_api::client::Logger for TestLogger {
            fn warn(&self, _message: &str) {}
            fn error(&self, _message: &str) {}
        }

        let client = NovelAIClient::new(Some("test-key"), Some(Box::new(TestLogger)));
        assert!(client.is_ok());
    }
}

// =============================================================================
// Integration Tests (HTTP Mocked)
// =============================================================================

mod integration {
    use super::*;
    use serial_test::serial;

    /// Set all NovelAI URL env vars to point at a mockito server.
    fn set_mock_urls(base: &str) {
        // Reset cached URLs first so env var changes take effect
        constants::reset_url_cache();
        std::env::set_var(
            "NOVELAI_SUBSCRIPTION_URL",
            format!("{}/user/subscription", base),
        );
        std::env::set_var(
            "NOVELAI_API_URL",
            format!("{}/ai/generate-image", base),
        );
        std::env::set_var(
            "NOVELAI_STREAM_URL",
            format!("{}/ai/generate-image-stream", base),
        );
        std::env::set_var(
            "NOVELAI_ENCODE_URL",
            format!("{}/ai/encode-vibe", base),
        );
        std::env::set_var(
            "NOVELAI_AUGMENT_URL",
            format!("{}/ai/augment-image", base),
        );
        std::env::set_var(
            "NOVELAI_UPSCALE_URL",
            format!("{}/ai/upscale", base),
        );
        // Reset again after setting to ensure fresh resolution
        constants::reset_url_cache();
    }

    fn clear_mock_urls() {
        for var in &[
            "NOVELAI_SUBSCRIPTION_URL",
            "NOVELAI_API_URL",
            "NOVELAI_STREAM_URL",
            "NOVELAI_ENCODE_URL",
            "NOVELAI_AUGMENT_URL",
            "NOVELAI_UPSCALE_URL",
        ] {
            std::env::remove_var(var);
        }
        constants::reset_url_cache();
    }

    fn mock_balance_json() -> &'static str {
        r#"{"trainingStepsLeft":{"fixedTrainingStepsLeft":1000,"purchasedTrainingSteps":500},"tier":3}"#
    }

    // -------------------------------------------------------------------------
    // Balance
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[serial]
    async fn get_anlas_balance_success() {
        let mut server = mockito::Server::new_async().await;
        set_mock_urls(&server.url());

        let mock = server
            .mock("GET", "/user/subscription")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_balance_json())
            .create_async()
            .await;

        let client = NovelAIClient::new(Some("test-key"), None).unwrap();
        let balance = client.get_anlas_balance().await.unwrap();

        assert_eq!(balance.fixed, 1000);
        assert_eq!(balance.purchased, 500);
        assert_eq!(balance.total, 1500);
        assert_eq!(balance.tier, 3);

        mock.assert_async().await;
        clear_mock_urls();
    }

    #[tokio::test]
    #[serial]
    async fn get_anlas_balance_zero_values() {
        let mut server = mockito::Server::new_async().await;
        set_mock_urls(&server.url());

        let _mock = server
            .mock("GET", "/user/subscription")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"trainingStepsLeft":{"fixedTrainingStepsLeft":0,"purchasedTrainingSteps":0},"tier":0}"#)
            .create_async()
            .await;

        let client = NovelAIClient::new(Some("test-key"), None).unwrap();
        let balance = client.get_anlas_balance().await.unwrap();

        assert_eq!(balance.fixed, 0);
        assert_eq!(balance.purchased, 0);
        assert_eq!(balance.total, 0);
        assert_eq!(balance.tier, 0);

        clear_mock_urls();
    }

    #[tokio::test]
    #[serial]
    async fn get_anlas_balance_unauthorized() {
        let mut server = mockito::Server::new_async().await;
        set_mock_urls(&server.url());

        let _mock = server
            .mock("GET", "/user/subscription")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let client = NovelAIClient::new(Some("bad-key"), None).unwrap();
        let result = client.get_anlas_balance().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            NovelAIError::Api { status_code, .. } => assert_eq!(status_code, 401),
            e => panic!("Expected Api error, got: {:?}", e),
        }

        clear_mock_urls();
    }

    // -------------------------------------------------------------------------
    // Generate
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[serial]
    async fn generate_basic_success() {
        let mut server = mockito::Server::new_async().await;
        set_mock_urls(&server.url());

        let png = create_test_png(64, 64);
        let zip = create_test_zip_with_png(&png);

        // Balance endpoint (called before and after generation)
        let _balance_mock = server
            .mock("GET", "/user/subscription")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_balance_json())
            .create_async()
            .await;

        // Generation endpoint
        let gen_mock = server
            .mock("POST", "/ai/generate-image")
            .with_status(200)
            .with_body(zip)
            .create_async()
            .await;

        let client = NovelAIClient::new(Some("test-key"), None).unwrap();
        let params = GenerateParams {
            prompt: "1girl, flower".to_string(),
            seed: Some(42),
            ..Default::default()
        };
        let result = client.generate(&params).await.unwrap();

        assert_eq!(result.image_data, png);
        assert_eq!(result.seed, 42);
        assert!(result.anlas_remaining.is_some());

        gen_mock.assert_async().await;
        clear_mock_urls();
    }

    #[tokio::test]
    #[serial]
    async fn generate_api_error_401() {
        let mut server = mockito::Server::new_async().await;
        set_mock_urls(&server.url());

        let _balance_mock = server
            .mock("GET", "/user/subscription")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let _gen_mock = server
            .mock("POST", "/ai/generate-image")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let client = NovelAIClient::new(Some("bad-key"), None).unwrap();
        let params = GenerateParams {
            prompt: "test".to_string(),
            seed: Some(1),
            ..Default::default()
        };
        let result = client.generate(&params).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            NovelAIError::Api { status_code, .. } => assert_eq!(status_code, 401),
            e => panic!("Expected Api error, got: {:?}", e),
        }

        clear_mock_urls();
    }

    #[tokio::test]
    #[serial]
    async fn generate_server_error_500() {
        let mut server = mockito::Server::new_async().await;
        set_mock_urls(&server.url());

        let _balance_mock = server
            .mock("GET", "/user/subscription")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_balance_json())
            .create_async()
            .await;

        let _gen_mock = server
            .mock("POST", "/ai/generate-image")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = NovelAIClient::new(Some("test-key"), None).unwrap();
        let params = GenerateParams {
            prompt: "test".to_string(),
            seed: Some(1),
            ..Default::default()
        };
        let result = client.generate(&params).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            NovelAIError::Api { status_code, .. } => assert_eq!(status_code, 500),
            e => panic!("Expected Api error, got: {:?}", e),
        }

        clear_mock_urls();
    }

    #[tokio::test]
    #[serial]
    async fn generate_with_save_to_temp_dir() {
        let mut server = mockito::Server::new_async().await;
        set_mock_urls(&server.url());

        let png = create_test_png(64, 64);
        let zip = create_test_zip_with_png(&png);

        let _balance_mock = server
            .mock("GET", "/user/subscription")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_balance_json())
            .create_async()
            .await;

        let _gen_mock = server
            .mock("POST", "/ai/generate-image")
            .with_status(200)
            .with_body(zip)
            .create_async()
            .await;

        let tmp = tempfile::tempdir().unwrap();
        let save_dir = tmp.path().join("output");

        let client = NovelAIClient::new(Some("test-key"), None).unwrap();
        let params = GenerateParams {
            prompt: "1girl".to_string(),
            seed: Some(42),
            save: SaveTarget::Directory {
                dir: save_dir.to_string_lossy().to_string(),
                filename: None,
            },
            ..Default::default()
        };
        let result = client.generate(&params).await.unwrap();

        assert!(result.saved_path.is_some());
        let saved = result.saved_path.unwrap();
        assert!(std::path::Path::new(&saved).exists());
        assert!(saved.ends_with(".png"));

        let saved_data = std::fs::read(&saved).unwrap();
        assert_eq!(saved_data, png);

        clear_mock_urls();
    }

    #[tokio::test]
    #[serial]
    async fn generate_with_save_to_explicit_path() {
        let mut server = mockito::Server::new_async().await;
        set_mock_urls(&server.url());

        let png = create_test_png(64, 64);
        let zip = create_test_zip_with_png(&png);

        let _balance_mock = server
            .mock("GET", "/user/subscription")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_balance_json())
            .create_async()
            .await;

        let _gen_mock = server
            .mock("POST", "/ai/generate-image")
            .with_status(200)
            .with_body(zip)
            .create_async()
            .await;

        let tmp = tempfile::tempdir().unwrap();
        let save_path = tmp.path().join("my_image.png");

        let client = NovelAIClient::new(Some("test-key"), None).unwrap();
        let params = GenerateParams {
            prompt: "1girl".to_string(),
            seed: Some(42),
            save: SaveTarget::ExactPath(save_path.to_string_lossy().to_string()),
            ..Default::default()
        };
        let result = client.generate(&params).await.unwrap();

        assert_eq!(
            result.saved_path.as_deref(),
            Some(save_path.to_string_lossy().as_ref())
        );
        assert!(save_path.exists());

        clear_mock_urls();
    }

    // -------------------------------------------------------------------------
    // Augment
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[serial]
    async fn augment_image_success() {
        let mut server = mockito::Server::new_async().await;
        set_mock_urls(&server.url());

        let input_png = create_test_png(128, 128);
        let output_png = create_test_png(128, 128);
        let zip = create_test_zip_with_png(&output_png);

        let _balance_mock = server
            .mock("GET", "/user/subscription")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_balance_json())
            .create_async()
            .await;

        let aug_mock = server
            .mock("POST", "/ai/augment-image")
            .with_status(200)
            .with_body(zip)
            .create_async()
            .await;

        let client = NovelAIClient::new(Some("test-key"), None).unwrap();
        let params = AugmentParams {
            req_type: constants::AugmentReqType::Declutter,
            image: ImageInput::Bytes(input_png),
            prompt: None,
            defry: None,
            save: SaveTarget::None,
        };
        let result = client.augment_image(&params).await.unwrap();

        assert_eq!(result.image_data, output_png);
        assert_eq!(result.req_type, constants::AugmentReqType::Declutter);

        aug_mock.assert_async().await;
        clear_mock_urls();
    }

    #[tokio::test]
    #[serial]
    async fn augment_image_colorize_with_prompt() {
        let mut server = mockito::Server::new_async().await;
        set_mock_urls(&server.url());

        let input_png = create_test_png(64, 64);
        let output_png = create_test_png(64, 64);
        let zip = create_test_zip_with_png(&output_png);

        let _balance_mock = server
            .mock("GET", "/user/subscription")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_balance_json())
            .create_async()
            .await;

        let _aug_mock = server
            .mock("POST", "/ai/augment-image")
            .with_status(200)
            .with_body(zip)
            .create_async()
            .await;

        let client = NovelAIClient::new(Some("test-key"), None).unwrap();
        let params = AugmentParams {
            req_type: constants::AugmentReqType::Colorize,
            image: ImageInput::Bytes(input_png),
            prompt: Some("warm colors".to_string()),
            defry: Some(4),
            save: SaveTarget::None,
        };
        let result = client.augment_image(&params).await.unwrap();

        assert!(!result.image_data.is_empty());
        assert_eq!(result.req_type, constants::AugmentReqType::Colorize);

        clear_mock_urls();
    }

    #[tokio::test]
    #[serial]
    async fn augment_image_with_save() {
        let mut server = mockito::Server::new_async().await;
        set_mock_urls(&server.url());

        let input_png = create_test_png(64, 64);
        let output_png = create_test_png(64, 64);
        let zip = create_test_zip_with_png(&output_png);

        let _balance_mock = server
            .mock("GET", "/user/subscription")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_balance_json())
            .create_async()
            .await;

        let _aug_mock = server
            .mock("POST", "/ai/augment-image")
            .with_status(200)
            .with_body(zip)
            .create_async()
            .await;

        let tmp = tempfile::tempdir().unwrap();
        let save_dir = tmp.path().join("augmented");

        let client = NovelAIClient::new(Some("test-key"), None).unwrap();
        let params = AugmentParams {
            req_type: constants::AugmentReqType::Sketch,
            image: ImageInput::Bytes(input_png),
            prompt: None,
            defry: None,
            save: SaveTarget::Directory {
                dir: save_dir.to_string_lossy().to_string(),
                filename: None,
            },
        };
        let result = client.augment_image(&params).await.unwrap();

        assert!(result.saved_path.is_some());
        let saved = result.saved_path.unwrap();
        assert!(std::path::Path::new(&saved).exists());
        assert!(saved.ends_with(".png"));

        clear_mock_urls();
    }

    // -------------------------------------------------------------------------
    // Upscale
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[serial]
    async fn upscale_image_success_zip_response() {
        let mut server = mockito::Server::new_async().await;
        set_mock_urls(&server.url());

        let input_png = create_test_png(64, 64);
        let output_png = create_test_png(256, 256);
        let zip = create_test_zip_with_png(&output_png);

        let _balance_mock = server
            .mock("GET", "/user/subscription")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_balance_json())
            .create_async()
            .await;

        let upscale_mock = server
            .mock("POST", "/ai/upscale")
            .with_status(200)
            .with_body(zip)
            .create_async()
            .await;

        let client = NovelAIClient::new(Some("test-key"), None).unwrap();
        let params = UpscaleParams {
            image: ImageInput::Bytes(input_png),
            scale: 4,
            save: SaveTarget::None,
        };
        let result = client.upscale_image(&params).await.unwrap();

        assert_eq!(result.image_data, output_png);
        assert_eq!(result.scale, 4);
        assert_eq!(result.output_width, 256); // 64 * 4
        assert_eq!(result.output_height, 256);

        upscale_mock.assert_async().await;
        clear_mock_urls();
    }

    #[tokio::test]
    #[serial]
    async fn upscale_image_raw_response() {
        let mut server = mockito::Server::new_async().await;
        set_mock_urls(&server.url());

        let input_png = create_test_png(64, 64);
        let output_png = create_test_png(128, 128);

        let _balance_mock = server
            .mock("GET", "/user/subscription")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_balance_json())
            .create_async()
            .await;

        // Return raw PNG (not ZIP)
        let _upscale_mock = server
            .mock("POST", "/ai/upscale")
            .with_status(200)
            .with_body(output_png.clone())
            .create_async()
            .await;

        let client = NovelAIClient::new(Some("test-key"), None).unwrap();
        let params = UpscaleParams {
            image: ImageInput::Bytes(input_png),
            scale: 2,
            save: SaveTarget::None,
        };
        let result = client.upscale_image(&params).await.unwrap();

        // Raw PNG returned directly
        assert_eq!(result.image_data, output_png);
        assert_eq!(result.scale, 2);
        assert_eq!(result.output_width, 128); // 64 * 2
        assert_eq!(result.output_height, 128);

        clear_mock_urls();
    }

    #[tokio::test]
    #[serial]
    async fn upscale_image_with_save() {
        let mut server = mockito::Server::new_async().await;
        set_mock_urls(&server.url());

        let input_png = create_test_png(64, 64);
        let output_png = create_test_png(256, 256);
        let zip = create_test_zip_with_png(&output_png);

        let _balance_mock = server
            .mock("GET", "/user/subscription")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_balance_json())
            .create_async()
            .await;

        let _upscale_mock = server
            .mock("POST", "/ai/upscale")
            .with_status(200)
            .with_body(zip)
            .create_async()
            .await;

        let tmp = tempfile::tempdir().unwrap();
        let save_dir = tmp.path().join("upscaled");

        let client = NovelAIClient::new(Some("test-key"), None).unwrap();
        let params = UpscaleParams {
            image: ImageInput::Bytes(input_png),
            scale: 4,
            save: SaveTarget::Directory {
                dir: save_dir.to_string_lossy().to_string(),
                filename: None,
            },
        };
        let result = client.upscale_image(&params).await.unwrap();

        assert!(result.saved_path.is_some());
        let saved = result.saved_path.unwrap();
        assert!(std::path::Path::new(&saved).exists());
        assert!(saved.contains("upscale_4x"));

        clear_mock_urls();
    }

    // -------------------------------------------------------------------------
    // Encode Vibe
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[serial]
    async fn encode_vibe_success() {
        let mut server = mockito::Server::new_async().await;
        set_mock_urls(&server.url());

        let input_png = create_test_png(64, 64);
        let encoding_data = b"mock-vibe-encoding-data-bytes";

        let _balance_mock = server
            .mock("GET", "/user/subscription")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_balance_json())
            .create_async()
            .await;

        let encode_mock = server
            .mock("POST", "/ai/encode-vibe")
            .with_status(200)
            .with_body(encoding_data.as_slice())
            .create_async()
            .await;

        let client = NovelAIClient::new(Some("test-key"), None).unwrap();
        let params = EncodeVibeParams {
            image: ImageInput::Bytes(input_png),
            model: constants::Model::NaiDiffusion45Full,
            information_extracted: 0.7,
            strength: 0.7,
            save: SaveTarget::None,
        };
        let result = client.encode_vibe(&params).await.unwrap();

        // Encoding should be base64 of the response
        let expected = base64::engine::general_purpose::STANDARD.encode(encoding_data);
        assert_eq!(result.encoding, expected);
        assert_eq!(result.model, constants::Model::NaiDiffusion45Full);
        assert_eq!(result.information_extracted, 0.7);
        assert_eq!(result.strength, 0.7);
        assert!(!result.source_image_hash.is_empty());
        assert!(!result.created_at.is_empty());

        encode_mock.assert_async().await;
        clear_mock_urls();
    }

    #[tokio::test]
    #[serial]
    async fn encode_vibe_save_to_temp_dir() {
        let mut server = mockito::Server::new_async().await;
        set_mock_urls(&server.url());

        let input_png = create_test_png(64, 64);
        let encoding_data = b"mock-encoding";

        let _balance_mock = server
            .mock("GET", "/user/subscription")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_balance_json())
            .create_async()
            .await;

        let _encode_mock = server
            .mock("POST", "/ai/encode-vibe")
            .with_status(200)
            .with_body(encoding_data.as_slice())
            .create_async()
            .await;

        let tmp = tempfile::tempdir().unwrap();
        let save_dir = tmp.path().join("vibes");

        let client = NovelAIClient::new(Some("test-key"), None).unwrap();
        let params = EncodeVibeParams {
            image: ImageInput::Bytes(input_png),
            save: SaveTarget::Directory {
                dir: save_dir.to_string_lossy().to_string(),
                filename: None,
            },
            ..Default::default()
        };
        let result = client.encode_vibe(&params).await.unwrap();

        assert!(result.saved_path.is_some());
        let saved = result.saved_path.unwrap();
        assert!(saved.ends_with(".naiv4vibe"));
        assert!(std::path::Path::new(&saved).exists());

        // Verify it's valid JSON with the correct structure
        let content = std::fs::read_to_string(&saved).unwrap();
        let vibe_json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(vibe_json["identifier"], "novelai-vibe-transfer");
        assert_eq!(vibe_json["version"], 1);
        assert_eq!(vibe_json["type"], "encoding");
        assert!(!vibe_json["id"].as_str().unwrap().is_empty());
        assert!(!vibe_json["name"].as_str().unwrap().is_empty());
        assert!(!vibe_json["createdAt"].as_str().unwrap().is_empty());

        // Check encodings structure
        let model_key = constants::Model::NaiDiffusion45Full.model_key();
        assert!(vibe_json["encodings"][model_key]["unknown"]["encoding"]
            .as_str()
            .is_some());

        clear_mock_urls();
    }

    #[tokio::test]
    #[serial]
    async fn encode_vibe_save_with_custom_filename() {
        let mut server = mockito::Server::new_async().await;
        set_mock_urls(&server.url());

        let input_png = create_test_png(64, 64);
        let encoding_data = b"mock-encoding-2";

        let _balance_mock = server
            .mock("GET", "/user/subscription")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_balance_json())
            .create_async()
            .await;

        let _encode_mock = server
            .mock("POST", "/ai/encode-vibe")
            .with_status(200)
            .with_body(encoding_data.as_slice())
            .create_async()
            .await;

        let tmp = tempfile::tempdir().unwrap();
        let save_dir = tmp.path().join("vibes");

        let client = NovelAIClient::new(Some("test-key"), None).unwrap();
        let params = EncodeVibeParams {
            image: ImageInput::Bytes(input_png),
            save: SaveTarget::Directory {
                dir: save_dir.to_string_lossy().to_string(),
                filename: Some("my_vibe".to_string()),
            },
            ..Default::default()
        };
        let result = client.encode_vibe(&params).await.unwrap();

        assert!(result.saved_path.is_some());
        let saved = result.saved_path.unwrap();
        assert!(saved.ends_with("my_vibe.naiv4vibe"));

        clear_mock_urls();
    }

    #[tokio::test]
    #[serial]
    async fn encode_vibe_save_with_explicit_path() {
        let mut server = mockito::Server::new_async().await;
        set_mock_urls(&server.url());

        let input_png = create_test_png(64, 64);
        let encoding_data = b"mock-encoding-3";

        let _balance_mock = server
            .mock("GET", "/user/subscription")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_balance_json())
            .create_async()
            .await;

        let _encode_mock = server
            .mock("POST", "/ai/encode-vibe")
            .with_status(200)
            .with_body(encoding_data.as_slice())
            .create_async()
            .await;

        let tmp = tempfile::tempdir().unwrap();
        let save_path = tmp.path().join("custom.naiv4vibe");

        let client = NovelAIClient::new(Some("test-key"), None).unwrap();
        let params = EncodeVibeParams {
            image: ImageInput::Bytes(input_png),
            save: SaveTarget::ExactPath(save_path.to_string_lossy().to_string()),
            ..Default::default()
        };
        let result = client.encode_vibe(&params).await.unwrap();

        assert_eq!(
            result.saved_path.as_deref(),
            Some(save_path.to_string_lossy().as_ref())
        );
        assert!(save_path.exists());

        clear_mock_urls();
    }

    // -------------------------------------------------------------------------
    // Anlas Tracking
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[serial]
    async fn generate_tracks_anlas_consumed() {
        let mut server = mockito::Server::new_async().await;
        set_mock_urls(&server.url());

        let png = create_test_png(64, 64);
        let zip = create_test_zip_with_png(&png);

        // First balance call returns 1000, second returns 983 (17 consumed)
        let _balance_before = server
            .mock("GET", "/user/subscription")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"trainingStepsLeft":{"fixedTrainingStepsLeft":500,"purchasedTrainingSteps":500},"tier":3}"#)
            .create_async()
            .await;

        let _gen_mock = server
            .mock("POST", "/ai/generate-image")
            .with_status(200)
            .with_body(zip)
            .create_async()
            .await;

        let client = NovelAIClient::new(Some("test-key"), None).unwrap();
        let params = GenerateParams {
            prompt: "test".to_string(),
            seed: Some(1),
            ..Default::default()
        };
        let result = client.generate(&params).await.unwrap();

        // Both calls return the same balance, so consumed = 0
        assert!(result.anlas_remaining.is_some());
        assert_eq!(result.anlas_consumed, Some(0));

        clear_mock_urls();
    }

    // -------------------------------------------------------------------------
    // Balance fetch failure graceful degradation
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[serial]
    async fn generate_succeeds_even_if_balance_fails() {
        let mut server = mockito::Server::new_async().await;
        set_mock_urls(&server.url());

        let png = create_test_png(64, 64);
        let zip = create_test_zip_with_png(&png);

        // Balance endpoint returns error
        let _balance_mock = server
            .mock("GET", "/user/subscription")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let _gen_mock = server
            .mock("POST", "/ai/generate-image")
            .with_status(200)
            .with_body(zip)
            .create_async()
            .await;

        let client = NovelAIClient::new(Some("test-key"), None).unwrap();
        let params = GenerateParams {
            prompt: "test".to_string(),
            seed: Some(1),
            ..Default::default()
        };
        let result = client.generate(&params).await.unwrap();

        // Generation succeeds even without balance info
        assert_eq!(result.image_data, png);
        assert!(result.anlas_remaining.is_none());
        assert!(result.anlas_consumed.is_none());

        clear_mock_urls();
    }
}
