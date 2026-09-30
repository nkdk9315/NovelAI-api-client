//! Incremental reading of the streaming endpoint (`stream: "msgpack"`), so a
//! caller can show the denoising previews while the image is generated.
//!
//! The body is `[u32 BE length][msgpack map]` frames: one `intermediate`
//! frame per sampling step (JPEG preview), then `final` (or `error`). The
//! whole body is still collected and handed to `parse_stream_response`, so
//! the final image goes through the same fallback chain as before.

use crate::constants;
use crate::error::{NovelAIError, Result};

/// One `intermediate` frame of the generation stream.
#[derive(Debug, Clone, PartialEq)]
pub struct GenerateProgress {
    /// `step_ix`: the sampling step this preview comes from
    pub step: u32,
    /// `sigma`: noise level left at this step (falls toward 0)
    pub sigma: Option<f64>,
    /// Preview image (JPEG)
    pub image: Vec<u8>,
}

/// Receives each preview while `generate_with_progress` runs.
pub type ProgressFn<'a> = dyn Fn(GenerateProgress) + Send + Sync + 'a;

/// Finds the frames of a growing body as they complete.
#[derive(Debug, Default)]
pub struct FrameScanner {
    /// Offset of the first frame not yet looked at
    next: usize,
}

impl FrameScanner {
    /// Previews in the frames of `buf` that completed since the last call.
    /// A body that is not framed (ZIP, bare PNG) yields nothing: its first
    /// bytes read as a length far beyond what ever arrives.
    pub fn scan(&mut self, buf: &[u8]) -> Vec<GenerateProgress> {
        let mut out = Vec::new();
        while let Some((frame, end)) = complete_frame(buf, self.next) {
            out.extend(intermediate_preview(frame));
            self.next = end;
        }
        out
    }
}

/// The frame starting at `at` once all of it has arrived, and the offset after it.
fn complete_frame(buf: &[u8], at: usize) -> Option<(&[u8], usize)> {
    let header: [u8; 4] = buf.get(at..at.checked_add(4)?)?.try_into().ok()?;
    let len = u32::from_be_bytes(header) as usize;
    let end = (at + 4).checked_add(len)?;
    if len == 0 || end > buf.len() {
        return None;
    }
    Some((&buf[at + 4..end], end))
}

/// The preview in a frame, if it is an `intermediate` event with an image.
pub fn intermediate_preview(frame: &[u8]) -> Option<GenerateProgress> {
    let rmpv::Value::Map(entries) = rmpv::decode::read_value(&mut &frame[..]).ok()? else {
        return None;
    };
    let field = |name: &str| entries.iter().find(|(k, _)| k.as_str() == Some(name)).map(|(_, v)| v);
    let event = field("event_type").or_else(|| field("event"))?.as_str()?;
    if event != "intermediate" {
        return None;
    }
    let rmpv::Value::Binary(image) = field("image")? else {
        return None;
    };
    Some(GenerateProgress {
        step: field("step_ix").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        sigma: field("sigma").and_then(|v| v.as_f64()),
        image: image.clone(),
    })
}

/// Read the whole body like `response::get_response_buffer`, calling
/// `on_progress` for each preview as soon as its frame has arrived.
pub async fn read_with_progress(mut response: reqwest::Response, on_progress: &ProgressFn<'_>) -> Result<Vec<u8>> {
    if let Some(len) = response.content_length() {
        if len > constants::MAX_RESPONSE_SIZE as u64 {
            return Err(NovelAIError::Parse(format!(
                "Response Content-Length too large: {} bytes (max {})",
                len,
                constants::MAX_RESPONSE_SIZE
            )));
        }
    }
    let mut buf = Vec::new();
    let mut scanner = FrameScanner::default();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| NovelAIError::Other(format!("Failed to read response body: {}", e)))?
    {
        buf.extend_from_slice(&chunk);
        if buf.len() > constants::MAX_RESPONSE_SIZE {
            return Err(NovelAIError::Parse(format!(
                "Response too large: {} bytes (max {})",
                buf.len(),
                constants::MAX_RESPONSE_SIZE
            )));
        }
        for preview in scanner.scan(&buf) {
            on_progress(preview);
        }
    }
    Ok(buf)
}
