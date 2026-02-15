use crate::constants;
use crate::error::{NovelAIError, Result};
use crate::schemas::CharacterReferenceConfig;
use crate::utils::image::{load_image_safe, encode_to_png};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use image::{DynamicImage, Rgba, RgbaImage};

// =============================================================================
// Result type
// =============================================================================

/// Result of processing character references for API payload construction.
pub struct ProcessedCharacterReferences {
    pub images: Vec<String>,
    pub descriptions: Vec<serde_json::Value>,
    pub info_extracted: Vec<f64>,
    pub strength_values: Vec<f64>,
    pub secondary_strength_values: Vec<f64>,
}

// =============================================================================
// Public Functions
// =============================================================================

/// Prepare a character reference image by resizing and padding to the appropriate size.
///
/// Selects target size based on aspect ratio:
/// - Portrait (< 0.8): 1024x1536
/// - Landscape (> 1.25): 1536x1024
/// - Square (0.8-1.25): 1472x1472
///
/// The image is resized to fit within the target dimensions while maintaining
/// aspect ratio, then centered on a black canvas.
pub fn prepare_character_reference_image(image_buffer: &[u8]) -> Result<Vec<u8>> {
    let img = load_image_safe(image_buffer)?;

    let orig_width = img.width();
    let orig_height = img.height();

    if orig_width == 0 || orig_height == 0 {
        return Err(NovelAIError::Image(
            "Could not get image dimensions".to_string(),
        ));
    }

    let aspect_ratio = orig_width as f64 / orig_height as f64;

    let (target_width, target_height) =
        if aspect_ratio < constants::CHARREF_PORTRAIT_THRESHOLD {
            constants::CHARREF_PORTRAIT_SIZE
        } else if aspect_ratio > constants::CHARREF_LANDSCAPE_THRESHOLD {
            constants::CHARREF_LANDSCAPE_SIZE
        } else {
            constants::CHARREF_SQUARE_SIZE
        };

    // Resize to fit within target while maintaining aspect ratio
    let resized = img.resize(
        target_width,
        target_height,
        image::imageops::FilterType::Lanczos3,
    );

    // Create opaque black canvas
    let mut canvas =
        RgbaImage::from_pixel(target_width, target_height, Rgba([0, 0, 0, 255]));

    // Center the resized image on the canvas
    let offset_x = ((target_width - resized.width()) / 2) as i64;
    let offset_y = ((target_height - resized.height()) / 2) as i64;

    image::imageops::overlay(&mut canvas, &resized.to_rgba8(), offset_x, offset_y);

    // Encode to PNG
    let dynamic = DynamicImage::ImageRgba8(canvas);
    encode_to_png(&dynamic)
}

/// Process an array of character reference configs into payload-ready data.
pub fn process_character_references(
    refs: &[CharacterReferenceConfig],
) -> Result<ProcessedCharacterReferences> {
    let mut images = Vec::new();
    let mut descriptions = Vec::new();
    let mut info_extracted = Vec::new();
    let mut strength_values = Vec::new();
    let mut secondary_strength_values = Vec::new();

    for ref_config in refs {
        let image_buffer = super::image::get_image_buffer(&ref_config.image)?;
        let processed_buffer = prepare_character_reference_image(&image_buffer)?;
        let b64_image = BASE64.encode(&processed_buffer);

        images.push(b64_image);

        let ref_type = ref_config.mode.as_str();
        descriptions.push(serde_json::json!({
            "caption": {
                "base_caption": ref_type,
                "char_captions": []
            },
            "legacy_uc": false
        }));

        info_extracted.push(1.0);
        strength_values.push(ref_config.strength);
        secondary_strength_values.push(1.0 - ref_config.fidelity);
    }

    Ok(ProcessedCharacterReferences {
        images,
        descriptions,
        info_extracted,
        strength_values,
        secondary_strength_values,
    })
}
