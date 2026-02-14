use crate::constants;
use crate::error::{NovelAIError, Result};
use crate::schemas::ImageInput;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
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

/// Sanitize a file path, checking for path traversal.
fn sanitize_file_path(file_path: &str) -> Result<String> {
    let normalized = file_path.replace('\\', "/");
    if normalized.contains("..") {
        return Err(NovelAIError::Validation(format!(
            "Invalid file path (path traversal detected): {}",
            file_path
        )));
    }
    Ok(normalized)
}

/// Decode a base64 image string, stripping optional data URL prefix.
fn decode_base64_image(base64_str: &str) -> Result<Vec<u8>> {
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
            let safe_path = sanitize_file_path(path)?;
            std::fs::read(&safe_path).map_err(|_| {
                NovelAIError::Image(format!(
                    "Image file not found or not readable: {}",
                    path
                ))
            })
        }
        ImageInput::Base64(b64) => decode_base64_image(b64),
        ImageInput::DataUrl(url) => decode_base64_image(url),
    }
}

/// Get image dimensions from image data.
/// Returns (width, height, buffer).
pub fn get_image_dimensions(input: &ImageInput) -> Result<(u32, u32, Vec<u8>)> {
    let buffer = get_image_buffer(input)?;
    let source = match input {
        ImageInput::FilePath(path) => Some(path.as_str()),
        _ => None,
    };
    validate_image_data_size(&buffer, source)?;

    let img = image::load_from_memory(&buffer).map_err(|_| {
        NovelAIError::Image(
            "Could not determine image dimensions. The file may be corrupted or not a valid image."
                .to_string(),
        )
    })?;

    let width = img.width();
    let height = img.height();

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
