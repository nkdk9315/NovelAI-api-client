use crate::constants;
use crate::error::{NovelAIError, Result};
use crate::schemas::ImageInput;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use image::ImageReader;
use std::io::Cursor;
use std::sync::LazyLock;
use regex::Regex;

// =============================================================================
// Internal Regex
// =============================================================================

static DATA_URL_PREFIX_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^data:image/[\w+.\-]+;base64,").unwrap()
});

static BASE64_ONLY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Za-z0-9+/\-_]+=*$").unwrap()
});

static IMAGE_EXTENSION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\.(png|jpg|jpeg|webp|gif|bmp|naiv4vibe)$").unwrap()
});

// =============================================================================
// Internal Helpers
// =============================================================================

const MAX_IMAGE_PIXELS: u64 = 16_384 * 16_384; // ~268M pixels

const MAX_BASE64_INPUT_LEN: usize = 14 * 1024 * 1024; // ~10MB decoded

/// Load an image from a byte buffer with a dimension safety check to prevent
/// decompression bombs. Reads only the header first, then performs the full
/// decode only when the pixel count is within the limit.
pub(crate) fn load_image_safe(buffer: &[u8]) -> Result<image::DynamicImage> {
    let reader = ImageReader::new(Cursor::new(buffer))
        .with_guessed_format()
        .map_err(|e| NovelAIError::Image(format!("Format detection failed: {}", e)))?;
    let (w, h) = reader.into_dimensions()
        .map_err(|e| NovelAIError::Image(format!("Cannot read dimensions: {}", e)))?;
    if (w as u64) * (h as u64) > MAX_IMAGE_PIXELS {
        return Err(NovelAIError::Image(
            format!("Image dimensions {}x{} exceed maximum pixel count", w, h),
        ));
    }
    image::load_from_memory(buffer)
        .map_err(|e| NovelAIError::Image(e.to_string()))
}

/// Encode a `DynamicImage` to PNG bytes.
pub(crate) fn encode_to_png(img: &image::DynamicImage) -> Result<Vec<u8>> {
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).map_err(|e| {
        NovelAIError::Image(format!("Failed to encode image as PNG: {}", e))
    })?;
    Ok(buf.into_inner())
}

/// Decode a base64 image string, stripping optional data URL prefix.
fn decode_base64_image(base64_str: &str) -> Result<Vec<u8>> {
    if base64_str.len() > MAX_BASE64_INPUT_LEN {
        return Err(NovelAIError::ImageFileSize {
            file_size_mb: base64_str.len() as f64 / 1_000_000.0,
            max_size_mb: 10,
            file_source: Some("base64 input".to_string()),
        });
    }
    let stripped = DATA_URL_PREFIX_REGEX.replace(base64_str, "").into_owned();
    if stripped.is_empty()
        || !stripped
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
    {
        return Err(NovelAIError::Image(
            "Invalid Base64 string: contains characters outside the Base64 alphabet or is empty"
                .to_string(),
        ));
    }
    BASE64
        .decode(stripped.as_bytes())
        .map_err(|e| NovelAIError::Image(format!("Failed to decode Base64: {}", e)))
}

// =============================================================================
// Public Functions
// =============================================================================

/// Validate image data size against MAX_REF_IMAGE_SIZE_MB.
pub fn validate_image_data_size(data: &[u8], source: Option<&str>) -> Result<()> {
    let size_mb = data.len() as f64 / (1024.0 * 1024.0);
    if size_mb > constants::MAX_REF_IMAGE_SIZE_MB as f64 {
        return Err(NovelAIError::ImageFileSize {
            file_size_mb: size_mb,
            max_size_mb: constants::MAX_REF_IMAGE_SIZE_MB,
            file_source: source.map(|s| s.to_string()),
        });
    }
    Ok(())
}

/// Convert an ImageInput to a byte buffer.
pub fn get_image_buffer(input: &ImageInput) -> Result<Vec<u8>> {
    match input {
        ImageInput::Bytes(data) => Ok(data.clone()),
        ImageInput::FilePath(path) => {
            let path_str = path.to_string_lossy();
            crate::utils::validate_safe_path(&path_str)?;
            std::fs::read(path).map_err(|_| {
                NovelAIError::Image(format!(
                    "Image file not found or not readable: {}",
                    path.display()
                ))
            })
        }
        ImageInput::Base64(b64) => decode_base64_image(b64),
        ImageInput::DataUrl(url) => decode_base64_image(url),
    }
}

/// Get image dimensions from image data.
/// Returns (width, height, buffer).
///
/// Uses header-only reading via `ImageReader::into_dimensions()` to avoid
/// fully decoding the image just to obtain its size.
pub fn get_image_dimensions(input: &ImageInput) -> Result<(u32, u32, Vec<u8>)> {
    let buffer = get_image_buffer(input)?;
    let source_string;
    let source = match input {
        ImageInput::FilePath(path) => {
            source_string = path.to_string_lossy().into_owned();
            Some(source_string.as_str())
        }
        _ => None,
    };
    validate_image_data_size(&buffer, source)?;

    let reader = ImageReader::new(Cursor::new(&buffer))
        .with_guessed_format()
        .map_err(|_| {
            NovelAIError::Image(
                "Could not determine image dimensions. The file may be corrupted or not a valid image."
                    .to_string(),
            )
        })?;

    let (width, height) = reader.into_dimensions().map_err(|_| {
        NovelAIError::Image(
            "Could not determine image dimensions. The file may be corrupted or not a valid image."
                .to_string(),
        )
    })?;

    if width == 0 || height == 0 {
        return Err(NovelAIError::Image(
            "Could not determine image dimensions. The file may be corrupted or not a valid image."
                .to_string(),
        ));
    }

    Ok((width, height, buffer))
}

/// Heuristically determine if a string looks like a file path.
/// Base64 strings can contain '/' but typically don't look like paths.
pub fn looks_like_file_path(s: &str) -> bool {
    // If it starts with data URL prefix, it's definitely not a path
    if s.starts_with("data:") {
        return false;
    }

    // Short-circuit: long Base64-only strings are not paths
    if BASE64_ONLY_REGEX.is_match(s) && s.len() > 64 {
        return false;
    }

    // Absolute paths (Unix) — require extension or directory structure
    if let Some(rest) = s.strip_prefix('/') {
        if IMAGE_EXTENSION_REGEX.is_match(s) {
            return true;
        }
        // Has at least two path segments (e.g., /dir/file)
        if rest.contains('/') {
            return true;
        }
        return false;
    }

    // Windows absolute paths (e.g., C:\...)
    let bytes = s.as_bytes();
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
    {
        return true;
    }

    // Relative paths with directory separators and file extension
    if (s.contains('/') || s.contains('\\')) && IMAGE_EXTENSION_REGEX.is_match(s) {
        return true;
    }

    // If it has a file extension, assume path
    if IMAGE_EXTENSION_REGEX.is_match(s) {
        return true;
    }

    // Default: if it contains directory separator, try as path
    s.contains('/') || s.contains('\\')
}

/// Convert an ImageInput to a base64 string.
pub fn get_image_base64(input: &ImageInput) -> Result<String> {
    let buffer = get_image_buffer(input)?;
    Ok(BASE64.encode(&buffer))
}
