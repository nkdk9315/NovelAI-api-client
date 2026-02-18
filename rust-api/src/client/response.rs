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
/// 3. Framed msgpack parse → extract last frame's data/image field (full resolution)
/// 4. Embedded PNG search (last occurrence) → slice to IEND
/// 5. Raw msgpack stream parse → extract data/image field
///
/// Framed msgpack is prioritized over embedded PNG search because the framed
/// parser correctly extracts the last frame (full-resolution image), while PNG
/// search may match a preview image embedded earlier in the stream.
pub fn parse_stream_response(content: &[u8], logger: &dyn Logger) -> Result<Vec<u8>> {
    // 1. Check for ZIP signature (PK)
    if content.len() > 1 && content[0] == 0x50 && content[1] == 0x4b {
        return parse_zip_response(content);
    }

    // 2. Check for PNG signature at start
    if content.len() > 8 && content[..8] == PNG_SIGNATURE {
        return Ok(content.to_vec());
    }

    // 3. Try framed msgpack parsing (4-byte length-prefixed binary frames)
    //    This must run before embedded PNG search because framed msgpack correctly
    //    extracts the last frame (full resolution), while PNG search may find a
    //    preview image embedded earlier in the stream.
    match try_parse_framed_msgpack(content) {
        FramedMsgpackResult::ImageData(data) => return Ok(data),
        FramedMsgpackResult::Error { message } => {
            return Err(NovelAIError::Api {
                status_code: 500,
                message,
            });
        }
        FramedMsgpackResult::None => {}
    }

    // 4. Search for embedded PNG (last occurrence = full-resolution image)
    if let Some(png_start) = rfind_subsequence(content, &PNG_SIGNATURE) {
        let iend_marker: [u8; 4] = [0x49, 0x45, 0x4e, 0x44];
        if let Some(iend_offset) = find_subsequence(&content[png_start..], &iend_marker) {
            // IEND chunk: 4 bytes "IEND" + 4 bytes CRC
            let end = png_start + iend_offset + 8;
            return Ok(content[png_start..end.min(content.len())].to_vec());
        }
        return Ok(content[png_start..].to_vec());
    }

    // 5. Fallback: raw msgpack stream parsing
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

/// Result of attempting to parse framed msgpack data.
enum FramedMsgpackResult {
    /// Successfully extracted image data from a frame.
    ImageData(Vec<u8>),
    /// Found an error event in a frame.
    Error { message: String },
    /// No actionable data found in frames.
    None,
}

/// Split binary content into frames based on 4-byte big-endian length prefixes.
///
/// Each frame is: [4-byte BE length][payload of that length].
/// Returns a vector of payload byte slices.
fn parse_binary_frames(content: &[u8]) -> Vec<&[u8]> {
    let mut frames = Vec::new();
    let mut offset = 0;
    while offset + 4 <= content.len() {
        let len = u32::from_be_bytes([
            content[offset],
            content[offset + 1],
            content[offset + 2],
            content[offset + 3],
        ]) as usize;
        offset += 4;
        if len == 0 || offset + len > content.len() {
            break;
        }
        frames.push(&content[offset..offset + len]);
        offset += len;
    }
    frames
}

/// Try to parse 4-byte length-prefixed binary frames as msgpack.
///
/// For each frame, decodes as msgpack and checks for:
/// - Error events (`event_type == "error"` with a `message` field)
/// - Image data (`data` or `image` binary/string fields)
///
/// Returns the first actionable result found.
fn try_parse_framed_msgpack(content: &[u8]) -> FramedMsgpackResult {
    if content.len() > MAX_MSGPACK_PARSE_SIZE || content.len() < 4 {
        return FramedMsgpackResult::None;
    }

    let frames = parse_binary_frames(content);
    if frames.is_empty() {
        return FramedMsgpackResult::None;
    }

    // Collect image data from the last frame that has it (highest resolution)
    let mut last_image_data: Option<Vec<u8>> = None;

    for frame in &frames {
        let mut cursor = std::io::Cursor::new(*frame);
        if let Ok(val) = rmpv::decode::read_value(&mut cursor) {
            if let rmpv::Value::Map(ref entries) = val {
                let mut event_type: Option<&str> = None;
                let mut error_message: Option<String> = None;
                let mut frame_image_data: Option<Vec<u8>> = None;

                for (key, value) in entries {
                    let key_str = match key {
                        rmpv::Value::String(s) => match s.as_str() {
                            Some(s) => s,
                            None => continue,
                        },
                        _ => continue,
                    };

                    match key_str {
                        "event_type" | "event" => {
                            if let rmpv::Value::String(s) = value {
                                event_type = s.as_str();
                            }
                        }
                        "message" | "error" => {
                            let msg = match value {
                                rmpv::Value::String(s) => {
                                    s.as_str().map(|s| s.to_string())
                                }
                                _ => None,
                            };
                            if msg.is_some() {
                                error_message = msg;
                            }
                        }
                        "data" | "image" => {
                            let candidate = match value {
                                rmpv::Value::Binary(data) => Some(data.clone()),
                                rmpv::Value::String(s) => Some(s.as_bytes().to_vec()),
                                _ => None,
                            };
                            if candidate.is_some() {
                                frame_image_data = candidate;
                            }
                        }
                        _ => {}
                    }
                }

                // Check for error event first
                if let Some(et) = event_type {
                    if et == "error" {
                        let message = error_message.unwrap_or_else(|| {
                            "Unknown error from API stream".to_string()
                        });
                        return FramedMsgpackResult::Error { message };
                    }
                }

                // Track image data (keep the last one found for highest resolution)
                if frame_image_data.is_some() {
                    last_image_data = frame_image_data;
                }
            }
        }
    }

    match last_image_data {
        Some(data) => FramedMsgpackResult::ImageData(data),
        None => FramedMsgpackResult::None,
    }
}

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
