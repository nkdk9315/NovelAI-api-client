use crate::error::{NovelAIError, Result};
use crate::utils::image::{load_image_safe, encode_to_png};

use image::{DynamicImage, GrayImage, Luma};

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

/// Size of one mask cell in pixels (the model works on an 8x-downscaled latent).
pub const MASK_CELL: u32 = 8;

/// Normalize a mask for the API: full target size, binary, snapped to 8px cells.
///
/// The mask is first reduced to one value per 8x8 cell (area average) and
/// thresholded at 50%, then scaled back up with nearest-neighbour. Any input
/// size works (a 1/8 cell grid or a full-size brush mask).
///
/// Why: the official site sends the full-size mask. A 1/8-size mask, or a
/// full-size mask whose edges fall between cells (soft/antialiased or
/// unaligned), makes V5 inpainting draw a grey frame along the mask border.
pub fn resize_mask_image(
    mask: &[u8],
    target_width: u32,
    target_height: u32,
) -> Result<Vec<u8>> {
    let cols = (target_width / MASK_CELL).max(1);
    let rows = (target_height / MASK_CELL).max(1);

    let img = load_image_safe(mask)?;

    let cells = img
        .resize_exact(cols, rows, image::imageops::FilterType::Triangle)
        .to_luma8();
    let binary = GrayImage::from_fn(cols, rows, |x, y| {
        Luma([if cells.get_pixel(x, y)[0] >= 128 { 255 } else { 0 }])
    });
    let full = image::imageops::resize(
        &binary,
        target_width,
        target_height,
        image::imageops::FilterType::Nearest,
    );

    encode_to_png(&DynamicImage::ImageLuma8(full))
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
    encode_to_png(&dynamic)
}
