use novelai_api::constants;
use novelai_api::error::NovelAIError;
use novelai_api::schemas::{ImageInput, VibeEncodeResult, VibeItem};
use novelai_api::utils;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use std::io::Write;
use tempfile::NamedTempFile;

/// Helper: create a small test PNG image in memory
fn create_test_png(width: u32, height: u32) -> Vec<u8> {
    let img = image::RgbaImage::from_pixel(width, height, image::Rgba([255, 0, 0, 255]));
    let dynamic = image::DynamicImage::ImageRgba8(img);
    let mut buf = std::io::Cursor::new(Vec::new());
    dynamic
        .write_to(&mut buf, image::ImageFormat::Png)
        .unwrap();
    buf.into_inner()
}

// =============================================================================
// A. ImageFileSize error
// =============================================================================

#[test]
fn a1_image_file_size_error_includes_source_in_message() {
    let err = NovelAIError::ImageFileSize {
        file_size_mb: 15.0,
        max_size_mb: 10,
        file_source: Some("/path/to/image.png".to_string()),
    };
    let msg = err.to_string();
    assert!(msg.contains("/path/to/image.png"), "message: {}", msg);
}

#[test]
fn a2_image_file_size_error_message_without_source() {
    let err = NovelAIError::ImageFileSize {
        file_size_mb: 15.0,
        max_size_mb: 10,
        file_source: None,
    };
    let msg = err.to_string();
    assert!(!msg.ends_with(':'), "message should not end with colon: {}", msg);
    assert!(msg.contains("15.00 MB"), "message: {}", msg);
    assert!(msg.contains("10 MB"), "message: {}", msg);
}

#[test]
fn a3_image_file_size_error_stores_values() {
    let err = NovelAIError::ImageFileSize {
        file_size_mb: 15.5,
        max_size_mb: 10,
        file_source: None,
    };
    match err {
        NovelAIError::ImageFileSize { file_size_mb, max_size_mb, .. } => {
            assert!((file_size_mb - 15.5).abs() < f64::EPSILON);
            assert_eq!(max_size_mb, 10);
        }
        _ => panic!("Expected ImageFileSize variant"),
    }
}

// =============================================================================
// B. validate_image_data_size
// =============================================================================

#[test]
fn b1_does_not_throw_for_buffer_within_size_limit() {
    let buf = vec![0u8; 1024]; // 1KB
    assert!(utils::image::validate_image_data_size(&buf, None).is_ok());
}

#[test]
fn b2_throws_image_file_size_error_for_oversized_buffer() {
    let size = (constants::MAX_REF_IMAGE_SIZE_MB as usize + 1) * 1024 * 1024;
    let buf = vec![0u8; size];
    let result = utils::image::validate_image_data_size(&buf, None);
    assert!(matches!(result, Err(NovelAIError::ImageFileSize { .. })));
}

#[test]
fn b3_error_message_includes_source() {
    let size = (constants::MAX_REF_IMAGE_SIZE_MB as usize + 1) * 1024 * 1024;
    let buf = vec![0u8; size];
    let result = utils::image::validate_image_data_size(&buf, Some("test.png"));
    match result {
        Err(e) => assert!(e.to_string().contains("test.png")),
        Ok(_) => panic!("Expected error"),
    }
}

#[test]
fn b4_exactly_max_size_passes() {
    let size = constants::MAX_REF_IMAGE_SIZE_MB as usize * 1024 * 1024;
    let buf = vec![0u8; size];
    assert!(utils::image::validate_image_data_size(&buf, None).is_ok());
}

#[test]
fn b5_empty_buffer_passes() {
    let buf: Vec<u8> = vec![];
    assert!(utils::image::validate_image_data_size(&buf, None).is_ok());
}

// =============================================================================
// C. get_image_buffer — Bytes input
// =============================================================================

#[test]
fn c1_returns_bytes_input() {
    let data = vec![1u8, 2, 3, 4];
    let input = ImageInput::Bytes(data.clone());
    let result = utils::image::get_image_buffer(&input).unwrap();
    assert_eq!(result, data);
}

#[test]
fn c2_returns_empty_bytes() {
    let input = ImageInput::Bytes(vec![]);
    let result = utils::image::get_image_buffer(&input).unwrap();
    assert!(result.is_empty());
}

// =============================================================================
// D. get_image_buffer — FilePath input (sanitizeFilePath)
// =============================================================================

#[test]
fn d1_reads_file_for_normal_path() {
    let png_data = create_test_png(2, 2);
    let mut temp = NamedTempFile::new().unwrap();
    temp.write_all(&png_data).unwrap();

    let input = ImageInput::FilePath(temp.path().to_str().unwrap().to_string());
    let result = utils::image::get_image_buffer(&input).unwrap();
    assert_eq!(result, png_data);
}

#[test]
fn d2_throws_for_path_traversal() {
    let input = ImageInput::FilePath("../../etc/passwd".to_string());
    let result = utils::image::get_image_buffer(&input);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("path traversal detected"));
}

#[test]
fn d3_throws_for_nested_path_traversal() {
    let input = ImageInput::FilePath("images/../../../secret".to_string());
    let result = utils::image::get_image_buffer(&input);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("path traversal detected"));
}

#[test]
fn d4_throws_for_nonexistent_file() {
    let input = ImageInput::FilePath("/nonexistent/image.png".to_string());
    let result = utils::image::get_image_buffer(&input);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found or not readable"));
}

// =============================================================================
// E. get_image_buffer — Base64 / DataUrl input
// =============================================================================

#[test]
fn e1_decodes_valid_data_url() {
    let original = b"hello world";
    let b64 = BASE64.encode(original);
    let input = ImageInput::DataUrl(format!("data:image/png;base64,{}", b64));
    let result = utils::image::get_image_buffer(&input).unwrap();
    assert_eq!(result, original);
}

#[test]
fn e2_strips_data_url_prefix_correctly() {
    let original = b"test-data";
    let b64 = BASE64.encode(original);
    let input = ImageInput::DataUrl(format!("data:image/png;base64,{}", b64));
    let result = utils::image::get_image_buffer(&input).unwrap();
    assert_eq!(result, original);
}

#[test]
fn e3_strips_svg_data_url_prefix() {
    let original = b"svg-content";
    let b64 = BASE64.encode(original);
    let input = ImageInput::DataUrl(format!("data:image/svg+xml;base64,{}", b64));
    let result = utils::image::get_image_buffer(&input).unwrap();
    assert_eq!(result, original);
}

#[test]
fn e4_throws_for_invalid_base64_characters() {
    let input = ImageInput::Base64("abc!@#$def".to_string());
    let result = utils::image::get_image_buffer(&input);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid Base64"));
}

#[test]
fn e5_throws_for_empty_base64_string() {
    let input = ImageInput::Base64("".to_string());
    let result = utils::image::get_image_buffer(&input);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid Base64"));
}

#[test]
fn e6_decodes_plain_base64_string() {
    let original = b"hello";
    let b64 = BASE64.encode(original);
    let input = ImageInput::Base64(b64);
    let result = utils::image::get_image_buffer(&input).unwrap();
    assert_eq!(result, original);
}

// =============================================================================
// F. get_image_base64
// =============================================================================

#[test]
fn f1_converts_bytes_to_base64() {
    let data = b"hello";
    let input = ImageInput::Bytes(data.to_vec());
    let result = utils::image::get_image_base64(&input).unwrap();
    assert_eq!(result, BASE64.encode(data));
}

#[test]
fn f2_round_trips_correctly() {
    let data = b"Hello";
    let input = ImageInput::Bytes(data.to_vec());
    let b64 = utils::image::get_image_base64(&input).unwrap();
    let decoded = BASE64.decode(b64.as_bytes()).unwrap();
    assert_eq!(decoded, data);
}

// =============================================================================
// G. get_image_dimensions
// =============================================================================

#[test]
fn g1_returns_correct_dimensions() {
    let png_data = create_test_png(200, 300);
    let input = ImageInput::Bytes(png_data.clone());
    let (width, height, buffer) = utils::image::get_image_dimensions(&input).unwrap();
    assert_eq!(width, 200);
    assert_eq!(height, 300);
    assert_eq!(buffer, png_data);
}

#[test]
fn g2_throws_for_oversized_buffer() {
    // Create a very large image (would exceed MAX_REF_IMAGE_SIZE_MB when decoded)
    // Instead, we'll test with raw bytes that are too large
    let size = (constants::MAX_REF_IMAGE_SIZE_MB as usize + 1) * 1024 * 1024;
    let buf = vec![0u8; size];
    let input = ImageInput::Bytes(buf);
    let result = utils::image::get_image_dimensions(&input);
    // Will fail at validate_image_data_size or image parsing
    assert!(result.is_err());
}

#[test]
fn g3_throws_for_invalid_image_data() {
    let input = ImageInput::Bytes(vec![1, 2, 3, 4]);
    let result = utils::image::get_image_dimensions(&input);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Could not determine image dimensions"));
}

// =============================================================================
// H. looks_like_file_path
// =============================================================================

#[test]
fn h1_data_url_is_not_a_path() {
    assert!(!utils::image::looks_like_file_path("data:image/png;base64,abc123"));
}

#[test]
fn h2_long_base64_string_is_not_a_path() {
    let long_b64 = BASE64.encode("x".repeat(100).as_bytes());
    assert!(long_b64.len() > 64);
    assert!(!utils::image::looks_like_file_path(&long_b64));
}

#[test]
fn h3_absolute_unix_path_with_extension() {
    assert!(utils::image::looks_like_file_path("/image.png"));
}

#[test]
fn h4_absolute_unix_path_with_segments() {
    assert!(utils::image::looks_like_file_path("/dir/file"));
}

#[test]
fn h5_windows_path() {
    assert!(utils::image::looks_like_file_path("C:\\images\\test.png"));
}

#[test]
fn h6_relative_path_with_extension() {
    assert!(utils::image::looks_like_file_path("images/test.png"));
}

#[test]
fn h7_filename_with_extension() {
    assert!(utils::image::looks_like_file_path("test.png"));
}

#[test]
fn h8_naiv4vibe_extension() {
    assert!(utils::image::looks_like_file_path("vibe.naiv4vibe"));
}

// =============================================================================
// I. load_vibe_file
// =============================================================================

#[test]
fn i1_parses_json_from_valid_file() {
    let vibe_data = serde_json::json!({ "encodings": { "v4full": {} } });
    let mut temp = NamedTempFile::new().unwrap();
    write!(temp, "{}", vibe_data).unwrap();

    let result = utils::vibe::load_vibe_file(temp.path().to_str().unwrap()).unwrap();
    assert_eq!(result, vibe_data);
}

#[test]
fn i2_throws_for_path_traversal() {
    let result = utils::vibe::load_vibe_file("../../etc/secret.naiv4vibe");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("path traversal detected"));
}

#[test]
fn i3_throws_for_nonexistent_file() {
    let result = utils::vibe::load_vibe_file("/nonexistent/vibe.naiv4vibe");
    assert!(result.is_err());
}

// =============================================================================
// J. extract_encoding
// =============================================================================

fn make_vibe_data(
    model_key: &str,
    encoding: &str,
    info_extracted: Option<f64>,
    import_info: Option<f64>,
) -> serde_json::Value {
    let mut data = serde_json::json!({
        "encodings": {
            model_key: {
                "someKey": {
                    "encoding": encoding,
                    "params": {}
                }
            }
        }
    });
    if let Some(ie) = info_extracted {
        data["encodings"][model_key]["someKey"]["params"]["information_extracted"] = serde_json::json!(ie);
    }
    if let Some(ii) = import_info {
        data["importInfo"] = serde_json::json!({ "information_extracted": ii });
    }
    data
}

#[test]
fn j1_extracts_encoding_and_info() {
    let data = make_vibe_data("v4-5full", "abc123", Some(0.8), None);
    let (encoding, info) = utils::vibe::extract_encoding(&data, "nai-diffusion-4-5-full").unwrap();
    assert_eq!(encoding, "abc123");
    assert!((info - 0.8).abs() < f64::EPSILON);
}

#[test]
fn j2_import_info_takes_priority() {
    let data = make_vibe_data("v4-5full", "abc123", Some(0.5), Some(0.9));
    let (_, info) = utils::vibe::extract_encoding(&data, "nai-diffusion-4-5-full").unwrap();
    assert!((info - 0.9).abs() < f64::EPSILON);
}

#[test]
fn j3_throws_for_nonexistent_model_key() {
    let data = serde_json::json!({ "encodings": {} });
    let result = utils::vibe::extract_encoding(&data, "nai-diffusion-4-5-full");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No encoding found"));
}

#[test]
fn j4_defaults_to_v4_5_full_model() {
    let data = make_vibe_data("v4-5full", "default-enc", None, None);
    let (encoding, _) = utils::vibe::extract_encoding(&data, "nai-diffusion-4-5-full").unwrap();
    assert_eq!(encoding, "default-enc");
}

// =============================================================================
// K. process_vibes
// =============================================================================

#[test]
fn k1_processes_vibe_encode_result() {
    let vibes = vec![VibeItem::Encoded(VibeEncodeResult {
        encoding: "enc1".to_string(),
        model: constants::Model::NaiDiffusion45Full,
        information_extracted: 0.7,
        strength: 0.5,
        source_image_hash: "a".repeat(64),
        created_at: "2024-01-01".to_string(),
        saved_path: None,
        anlas_remaining: None,
        anlas_consumed: None,
    })];
    let result = utils::vibe::process_vibes(&vibes, "nai-diffusion-4-5-full").unwrap();
    assert_eq!(result.encodings, vec!["enc1"]);
    assert!((result.info_extracted_list[0] - 0.7).abs() < f64::EPSILON);
}

#[test]
fn k2_processes_naiv4vibe_file_path() {
    let vibe_data = serde_json::json!({
        "encodings": {
            "v4-5full": {
                "key1": {
                    "encoding": "file-enc",
                    "params": { "information_extracted": 0.6 }
                }
            }
        }
    });
    let mut temp = NamedTempFile::new().unwrap();
    write!(temp, "{}", vibe_data).unwrap();

    let path = temp.path().to_str().unwrap().to_string();
    let vibes = vec![VibeItem::FilePath(path)];
    let result = utils::vibe::process_vibes(&vibes, "nai-diffusion-4-5-full").unwrap();
    assert_eq!(result.encodings, vec!["file-enc"]);
    assert!((result.info_extracted_list[0] - 0.6).abs() < f64::EPSILON);
}

#[test]
fn k3_processes_raw_encoding_with_default_info() {
    let vibes = vec![VibeItem::RawEncoding("someBase64EncodedString".to_string())];
    let result = utils::vibe::process_vibes(&vibes, "nai-diffusion-4-5-full").unwrap();
    assert_eq!(result.encodings, vec!["someBase64EncodedString"]);
    assert!((result.info_extracted_list[0] - 1.0).abs() < f64::EPSILON);
}

#[test]
fn k4_empty_array_returns_empty_results() {
    let vibes: Vec<VibeItem> = vec![];
    let result = utils::vibe::process_vibes(&vibes, "nai-diffusion-4-5-full").unwrap();
    assert!(result.encodings.is_empty());
    assert!(result.info_extracted_list.is_empty());
}

// =============================================================================
// L. create_rectangular_mask — validation
// =============================================================================

#[test]
fn l1_throws_for_zero_width() {
    let region = utils::mask::MaskRegion { x: 0.0, y: 0.0, w: 1.0, h: 1.0 };
    let result = utils::mask::create_rectangular_mask(0, 100, &region);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid dimensions"));
}

#[test]
fn l2_throws_for_negative_height() {
    // u32 can't be negative, so we test with 0 instead
    let region = utils::mask::MaskRegion { x: 0.0, y: 0.0, w: 1.0, h: 1.0 };
    let result = utils::mask::create_rectangular_mask(100, 0, &region);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid dimensions"));
}

#[test]
fn l3_throws_for_region_x_gt_1() {
    let region = utils::mask::MaskRegion { x: 1.5, y: 0.0, w: 0.5, h: 0.5 };
    let result = utils::mask::create_rectangular_mask(100, 100, &region);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid region.x"));
}

#[test]
fn l4_throws_for_region_y_lt_0() {
    let region = utils::mask::MaskRegion { x: 0.0, y: -0.1, w: 0.5, h: 0.5 };
    let result = utils::mask::create_rectangular_mask(100, 100, &region);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid region.y"));
}

#[test]
fn l5_returns_valid_png_for_valid_input() {
    let region = utils::mask::MaskRegion { x: 0.1, y: 0.1, w: 0.5, h: 0.5 };
    let result = utils::mask::create_rectangular_mask(800, 600, &region).unwrap();
    // Verify it's a valid PNG by loading it
    let img = image::load_from_memory(&result).unwrap();
    assert_eq!(img.width(), 800 / 8);
    assert_eq!(img.height(), 600 / 8);
}

// =============================================================================
// M. create_circular_mask — validation
// =============================================================================

#[test]
fn m1_throws_for_zero_width() {
    let center = utils::mask::MaskCenter { x: 0.5, y: 0.5 };
    let result = utils::mask::create_circular_mask(0, 100, &center, 0.3);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid dimensions"));
}

#[test]
fn m2_throws_for_center_gt_1() {
    let center = utils::mask::MaskCenter { x: 1.5, y: 0.5 };
    let result = utils::mask::create_circular_mask(100, 100, &center, 0.3);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid center"));
}

#[test]
fn m3_throws_for_negative_radius() {
    let center = utils::mask::MaskCenter { x: 0.5, y: 0.5 };
    let result = utils::mask::create_circular_mask(100, 100, &center, -0.1);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid radius"));
}

#[test]
fn m4_throws_for_radius_gt_1() {
    let center = utils::mask::MaskCenter { x: 0.5, y: 0.5 };
    let result = utils::mask::create_circular_mask(100, 100, &center, 1.5);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid radius"));
}

#[test]
fn m5_returns_valid_png_for_valid_input() {
    let center = utils::mask::MaskCenter { x: 0.5, y: 0.5 };
    let result = utils::mask::create_circular_mask(800, 600, &center, 0.3).unwrap();
    let img = image::load_from_memory(&result).unwrap();
    assert_eq!(img.width(), 800 / 8);
    assert_eq!(img.height(), 600 / 8);
}

// =============================================================================
// N. prepare_character_reference_image — aspect ratio thresholds
// =============================================================================

#[test]
fn n1_portrait_aspect_ratio_uses_portrait_size() {
    // 400/600 = 0.667 < 0.8 → Portrait (1024x1536)
    let png_data = create_test_png(400, 600);
    let result = utils::charref::prepare_character_reference_image(&png_data).unwrap();
    let img = image::load_from_memory(&result).unwrap();
    assert_eq!(img.width(), constants::CHARREF_PORTRAIT_SIZE.0);
    assert_eq!(img.height(), constants::CHARREF_PORTRAIT_SIZE.1);
}

#[test]
fn n2_landscape_aspect_ratio_uses_landscape_size() {
    // 800/400 = 2.0 > 1.25 → Landscape (1536x1024)
    let png_data = create_test_png(800, 400);
    let result = utils::charref::prepare_character_reference_image(&png_data).unwrap();
    let img = image::load_from_memory(&result).unwrap();
    assert_eq!(img.width(), constants::CHARREF_LANDSCAPE_SIZE.0);
    assert_eq!(img.height(), constants::CHARREF_LANDSCAPE_SIZE.1);
}

#[test]
fn n3_square_aspect_ratio_uses_square_size() {
    // 500/500 = 1.0, between 0.8 and 1.25 → Square (1472x1472)
    let png_data = create_test_png(500, 500);
    let result = utils::charref::prepare_character_reference_image(&png_data).unwrap();
    let img = image::load_from_memory(&result).unwrap();
    assert_eq!(img.width(), constants::CHARREF_SQUARE_SIZE.0);
    assert_eq!(img.height(), constants::CHARREF_SQUARE_SIZE.1);
}

#[test]
fn n4_invalid_image_data_throws() {
    let result = utils::charref::prepare_character_reference_image(&[1, 2, 3, 4]);
    assert!(result.is_err());
}

// =============================================================================
// O. calculate_cache_secret_key
// =============================================================================

#[test]
fn o1_returns_sha256_hex_string() {
    let data = b"test image data";
    let hash = utils::mask::calculate_cache_secret_key(data);
    assert_eq!(hash.len(), 64); // SHA256 hex is 64 chars
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn o2_same_input_produces_same_hash() {
    let data = b"consistent data";
    let hash1 = utils::mask::calculate_cache_secret_key(data);
    let hash2 = utils::mask::calculate_cache_secret_key(data);
    assert_eq!(hash1, hash2);
}

#[test]
fn o3_different_input_produces_different_hash() {
    let hash1 = utils::mask::calculate_cache_secret_key(b"data1");
    let hash2 = utils::mask::calculate_cache_secret_key(b"data2");
    assert_ne!(hash1, hash2);
}

// =============================================================================
// P. resize_mask_image
// =============================================================================

#[test]
fn p1_resizes_to_one_eighth() {
    let png_data = create_test_png(800, 600);
    let result = utils::mask::resize_mask_image(&png_data, 800, 600).unwrap();
    let img = image::load_from_memory(&result).unwrap();
    assert_eq!(img.width(), 100); // 800/8
    assert_eq!(img.height(), 75);  // 600/8
}

#[test]
fn p2_invalid_image_data_throws() {
    let result = utils::mask::resize_mask_image(&[1, 2, 3], 800, 600);
    assert!(result.is_err());
}
