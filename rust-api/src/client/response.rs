use std::io::Read;

use crate::client::Logger;
use crate::constants;
use crate::error::{NovelAIError, Result};

/// Maximum buffer size allowed before attempting MessagePack parsing.
const MAX_MSGPACK_PARSE_SIZE: usize = 50 * 1024 * 1024; // 50MB

/// Read HTTP response body with size validation.
///
/// Performs a Content-Length pre-check before downloading, then a post-check
/// on the actual downloaded bytes to guard against missing/lying headers.
pub async fn get_response_buffer(response: reqwest::Response) -> Result<Vec<u8>> {
    // Pre-check: reject early based on Content-Length header to avoid
    // downloading an oversized response body into memory.
    if let Some(content_length) = response.content_length() {
        if content_length > constants::MAX_RESPONSE_SIZE as u64 {
            return Err(NovelAIError::Parse(format!(
                "Response Content-Length too large: {} bytes (max {})",
                content_length,
                constants::MAX_RESPONSE_SIZE
            )));
        }
    }

    let bytes = response.bytes().await.map_err(|e| {
        NovelAIError::Other(format!("Failed to read response body: {}", e))
    })?;

    // Post-check: the Content-Length header could be absent or spoofed,
    // so verify actual downloaded size.
    if bytes.len() > constants::MAX_RESPONSE_SIZE {
        return Err(NovelAIError::Parse(format!(
            "Response too large: {} bytes (max {})",
            bytes.len(),
            constants::MAX_RESPONSE_SIZE
        )));
    }

    Ok(bytes.to_vec())
}

/// Extract an image from a ZIP response.
///
/// Security: checks entry count, decompressed size, and compression ratio.
pub fn parse_zip_response(content: &[u8]) -> Result<Vec<u8>> {
    let reader = std::io::Cursor::new(content);
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| {
        NovelAIError::Parse(format!("Failed to open ZIP response: {}", e))
    })?;

    if archive.len() > constants::MAX_ZIP_ENTRIES {
        return Err(NovelAIError::Parse(format!(
            "Too many ZIP entries: {} (max {})",
            archive.len(),
            constants::MAX_ZIP_ENTRIES
        )));
    }

    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(|e| {
            NovelAIError::Parse(format!("Failed to read ZIP entry: {}", e))
        })?;

        let name = file.name().to_lowercase();
        let is_image = name.ends_with(".png")
            || name.ends_with(".webp")
            || name.ends_with(".jpg")
            || name.ends_with(".jpeg");

        if !is_image {
            continue;
        }

        // Use .take() to limit actual decompression size instead of trusting
        // the ZIP header's declared size, which can be spoofed (ZIP bomb defense).
        let max_size = constants::MAX_DECOMPRESSED_IMAGE_SIZE as u64;
        let mut limited_reader = file.take(max_size + 1);
        let mut image_data = Vec::new();
        limited_reader.read_to_end(&mut image_data).map_err(|e| {
            NovelAIError::Parse(format!("Failed to decompress ZIP entry: {}", e))
        })?;
        if image_data.len() > constants::MAX_DECOMPRESSED_IMAGE_SIZE {
            return Err(NovelAIError::Parse(format!(
                "Decompressed image exceeds size limit ({} bytes max)",
                constants::MAX_DECOMPRESSED_IMAGE_SIZE
            )));
        }
        return Ok(image_data);
    }

    Err(NovelAIError::Parse(
        "No image found in response ZIP".to_string(),
    ))
}

/// Parse a stream response using fallback chain:
/// 1. ZIP signature (PK) → parse_zip_response
/// 2. PNG signature at start → return as-is
/// 3. PNG magic byte search (last occurrence) → slice to IEND
/// 4. msgpack parse → extract data/image field
///
/// PNG search is prioritized over msgpack because streaming responses contain
/// msgpack preview messages followed by a raw full-resolution PNG at the end.
/// The msgpack `data`/`image` fields hold low-resolution previews, so we must
/// extract the trailing PNG first.
pub fn parse_stream_response(content: &[u8], logger: &dyn Logger) -> Result<Vec<u8>> {
    // 1. Check for ZIP signature (PK)
    if content.len() > 1 && content[0] == 0x50 && content[1] == 0x4b {
        return parse_zip_response(content);
    }

    // 2. Check for PNG signature at start
    if content.len() > 8 && content[..8] == PNG_SIGNATURE {
        return Ok(content.to_vec());
    }

    // 3. Search for embedded PNG (last occurrence = full-resolution image)
    if let Some(png_start) = rfind_subsequence(content, &PNG_SIGNATURE) {
        let iend_marker: [u8; 4] = [0x49, 0x45, 0x4e, 0x44];
        if let Some(iend_offset) = find_subsequence(&content[png_start..], &iend_marker) {
            // IEND chunk: 4 bytes "IEND" + 4 bytes CRC
            let end = png_start + iend_offset + 8;
            return Ok(content[png_start..end.min(content.len())].to_vec());
        }
        return Ok(content[png_start..].to_vec());
    }

    // 4. Fallback: msgpack stream parsing
    match try_parse_msgpack(content) {
        Some(data) => return Ok(data),
        None => {
            logger.warn(
                "[NovelAI] msgpack parse failed, falling back to error",
            );
        }
    }

    Err(NovelAIError::Parse(format!(
        "Cannot parse stream response (length: {})",
        content.len()
    )))
}

// =============================================================================
// Internal Helpers
// =============================================================================

const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

/// Try to parse msgpack stream, extracting 'data' or 'image' binary fields.
///
/// This is the last-resort fallback for non-streaming responses that happen
/// to be msgpack-encoded. Returns the first match found.
///
/// Limits the buffer size before parsing to prevent stack overflow or
/// excessive memory consumption from deeply nested or oversized payloads.
fn try_parse_msgpack(content: &[u8]) -> Option<Vec<u8>> {
    // Reject oversized buffers before attempting to parse to limit memory
    // and stack usage from malicious deeply-nested msgpack structures.
    if content.len() > MAX_MSGPACK_PARSE_SIZE {
        return None;
    }

    let mut cursor = std::io::Cursor::new(content);

    while let Ok(val) = rmpv::decode::read_value(&mut cursor) {
        if let rmpv::Value::Map(entries) = val {
            for (key, value) in entries {
                let key_str = match &key {
                    rmpv::Value::String(s) => match s.as_str() {
                        Some(s) => s,
                        None => continue,
                    },
                    _ => continue,
                };
                if key_str == "data" || key_str == "image" {
                    let candidate = match value {
                        rmpv::Value::Binary(data) => Some(data),
                        rmpv::Value::String(s) => Some(s.into_bytes()),
                        _ => None,
                    };
                    if let Some(data) = candidate {
                        return Some(data);
                    }
                }
            }
        }
    }
    None
}

/// Find the first occurrence of needle in haystack.
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Find the last occurrence of needle in haystack.
fn rfind_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .rposition(|window| window == needle)
}
