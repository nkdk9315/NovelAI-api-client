use crate::error::{NovelAIError, Result};

use image::{DynamicImage, GrayImage, ImageFormat, Luma};
use sha2::{Digest, Sha256};
use std::io::Cursor;

// =============================================================================
// Mask Region / Center types
// =============================================================================

/// Rectangular mask region with relative coordinates (0.0 - 1.0).
pub struct MaskRegion {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Circle center with relative coordinates (0.0 - 1.0).
pub struct MaskCenter {
    pub x: f64,
    pub y: f64,
}

// =============================================================================
// Public Functions
// =============================================================================

/// Calculate SHA256 hash of image data (for cache_secret_key).
pub fn calculate_cache_secret_key(image_data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(image_data);
    format!("{:x}", hasher.finalize())
}

/// Resize mask image to 1/8 of target dimensions (API specification).
pub fn resize_mask_image(
    mask: &[u8],
    target_width: u32,
    target_height: u32,
) -> Result<Vec<u8>> {
    let mask_width = target_width / 8;
    let mask_height = target_height / 8;

    let img = image::load_from_memory(mask).map_err(|e| {
        NovelAIError::Image(format!("Failed to load mask image: {}", e))
    })?;

    let resized = img.resize_exact(mask_width, mask_height, image::imageops::FilterType::Lanczos3);
    let gray = resized.to_luma8();

    let dynamic = DynamicImage::ImageLuma8(gray);
    let mut buf = Cursor::new(Vec::new());
    dynamic.write_to(&mut buf, ImageFormat::Png).map_err(|e| {
        NovelAIError::Image(format!("Failed to encode mask as PNG: {}", e))
    })?;

    Ok(buf.into_inner())
}

/// Create a rectangular mask image programmatically.
///
/// - `width`, `height`: Original image dimensions
/// - `region`: Mask region with relative coordinates (0.0-1.0)
/// - Returns: PNG-encoded mask (1/8 size, white=change area, black=keep area)
pub fn create_rectangular_mask(
    width: u32,
    height: u32,
    region: &MaskRegion,
) -> Result<Vec<u8>> {
    if width == 0 || height == 0 {
        return Err(NovelAIError::Validation(format!(
            "Invalid dimensions: width ({}) and height ({}) must be positive",
            width, height
        )));
    }

    validate_region_value("x", region.x)?;
    validate_region_value("y", region.y)?;
    validate_region_value("w", region.w)?;
    validate_region_value("h", region.h)?;

    let mask_width = width / 8;
    let mask_height = height / 8;

    // Convert relative coordinates to absolute
    let rect_x = (region.x * mask_width as f64) as u32;
    let rect_y = (region.y * mask_height as f64) as u32;
    let rect_w = (region.w * mask_width as f64) as u32;
    let rect_h = (region.h * mask_height as f64) as u32;

    // Create black canvas
    let mut img = GrayImage::new(mask_width, mask_height);

    // Fill region with white (255)
    for y in rect_y..std::cmp::min(rect_y + rect_h, mask_height) {
        for x in rect_x..std::cmp::min(rect_x + rect_w, mask_width) {
            img.put_pixel(x, y, Luma([255]));
        }
    }

    encode_gray_to_png(&img)
}

/// Create a circular mask image programmatically.
///
/// - `width`, `height`: Original image dimensions
/// - `center`: Center of circle with relative coordinates (0.0-1.0)
/// - `radius`: Radius relative to width (0.0-1.0)
/// - Returns: PNG-encoded mask (1/8 size)
pub fn create_circular_mask(
    width: u32,
    height: u32,
    center: &MaskCenter,
    radius: f64,
) -> Result<Vec<u8>> {
    if width == 0 || height == 0 {
        return Err(NovelAIError::Validation(format!(
            "Invalid dimensions: width ({}) and height ({}) must be positive",
            width, height
        )));
    }

    if center.x < 0.0 || center.x > 1.0 || center.y < 0.0 || center.y > 1.0 {
        return Err(NovelAIError::Validation(format!(
            "Invalid center: ({}, {}) (values must be between 0.0 and 1.0)",
            center.x, center.y
        )));
    }

    if !(0.0..=1.0).contains(&radius) {
        return Err(NovelAIError::Validation(format!(
            "Invalid radius: {} (must be between 0.0 and 1.0)",
            radius
        )));
    }

    let mask_width = width / 8;
    let mask_height = height / 8;

    let center_x = center.x * mask_width as f64;
    let center_y = center.y * mask_height as f64;
    let radius_px_sq = (radius * mask_width as f64).powi(2);

    let mut img = GrayImage::new(mask_width, mask_height);

    for y in 0..mask_height {
        for x in 0..mask_width {
            let dx = x as f64 - center_x;
            let dy = y as f64 - center_y;
            if dx * dx + dy * dy <= radius_px_sq {
                img.put_pixel(x, y, Luma([255]));
            }
        }
    }

    encode_gray_to_png(&img)
}

// =============================================================================
// Internal Helpers
// =============================================================================

fn validate_region_value(name: &str, value: f64) -> Result<()> {
    if !(0.0..=1.0).contains(&value) {
        return Err(NovelAIError::Validation(format!(
            "Invalid region.{}: {} (must be between 0.0 and 1.0)",
            name, value
        )));
    }
    Ok(())
}

fn encode_gray_to_png(img: &GrayImage) -> Result<Vec<u8>> {
    let dynamic = DynamicImage::ImageLuma8(img.clone());
    let mut buf = Cursor::new(Vec::new());
    dynamic.write_to(&mut buf, ImageFormat::Png).map_err(|e| {
        NovelAIError::Image(format!("Failed to encode mask as PNG: {}", e))
    })?;
    Ok(buf.into_inner())
}
