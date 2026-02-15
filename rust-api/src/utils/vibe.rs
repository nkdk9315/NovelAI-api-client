use crate::constants;
use crate::error::{NovelAIError, Result};
use crate::schemas::{VibeConfig, VibeItem};

use serde_json::Value;

// =============================================================================
// Result type
// =============================================================================

/// Result of processing vibes for API payload construction.
pub struct ProcessedVibes {
    pub encodings: Vec<String>,
    pub strengths: Vec<f64>,
    pub info_extracted_list: Vec<f64>,
}

// =============================================================================
// Constants
// =============================================================================

/// Maximum vibe file size (10 MB).
const MAX_VIBE_FILE_SIZE: u64 = 10 * 1024 * 1024;

// =============================================================================
// Public Functions
// =============================================================================

/// Load and parse a .naiv4vibe JSON file.
pub fn load_vibe_file(vibe_path: &str) -> Result<Value> {
    crate::utils::validate_safe_path(vibe_path)?;
    let file_size = std::fs::metadata(vibe_path)
        .map_err(|e| NovelAIError::Image(format!("Failed to read vibe file '{}': {}", vibe_path, e)))?
        .len();
    if file_size > MAX_VIBE_FILE_SIZE {
        return Err(NovelAIError::ImageFileSize {
            file_size_mb: file_size as f64 / (1024.0 * 1024.0),
            max_size_mb: 10,
            file_source: Some(vibe_path.to_string()),
        });
    }
    let content = std::fs::read_to_string(vibe_path).map_err(|e| {
        NovelAIError::Image(format!("Failed to read vibe file '{}': {}", vibe_path, e))
    })?;
    serde_json::from_str(&content).map_err(|e| {
        NovelAIError::Image(format!("Failed to parse vibe file '{}': {}", vibe_path, e))
    })
}

/// Extract encoding and information_extracted from vibe data for a given model.
pub fn extract_encoding(vibe_data: &Value, model: &str) -> Result<(String, f64)> {
    let model_key = constants::model_key_from_str(model).unwrap_or("v4-5full");

    let empty_map = serde_json::Map::new();
    let encodings = vibe_data
        .get("encodings")
        .and_then(|v| v.as_object())
        .unwrap_or(&empty_map);

    let model_encodings = encodings
        .get(model_key)
        .and_then(|v| v.as_object());

    let model_encodings = match model_encodings {
        Some(m) if !m.is_empty() => m,
        _ => {
            return Err(NovelAIError::Image(format!(
                "No encoding found for model key: {}",
                model_key
            )));
        }
    };

    // Get the first key's data
    let (_, encoding_data) = model_encodings.iter().next().unwrap();

    let encoding = encoding_data
        .get("encoding")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let params = encoding_data
        .get("params")
        .and_then(|v| v.as_object());

    let mut information_extracted = params
        .and_then(|p| p.get("information_extracted"))
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);

    // importInfo.information_extracted takes priority
    if let Some(import_info_extracted) = vibe_data
        .get("importInfo")
        .and_then(|v| v.get("information_extracted"))
        .and_then(|v| v.as_f64())
    {
        information_extracted = import_info_extracted;
    }

    Ok((encoding, information_extracted))
}

/// Process an array of vibe configurations into encodings, strengths, and
/// information_extracted lists ready for the API payload.
///
/// For each `VibeConfig`, the encoding is extracted from the inner `VibeItem`,
/// while `strength` and `info_extracted` are taken directly from the config.
pub fn process_vibes(vibes: &[VibeConfig], model: &str) -> Result<ProcessedVibes> {
    let mut encodings = Vec::new();
    let mut strengths = Vec::new();
    let mut info_extracted_list = Vec::new();

    for vibe in vibes {
        match &vibe.item {
            VibeItem::Encoded(result) => {
                encodings.push(result.encoding.clone());
            }
            VibeItem::FilePath(path) => {
                let data = load_vibe_file(&path.to_string_lossy())?;
                let (encoding, _file_info) = extract_encoding(&data, model)?;
                encodings.push(encoding);
            }
            VibeItem::RawEncoding(encoding) => {
                encodings.push(encoding.clone());
            }
        }
        strengths.push(vibe.strength);
        info_extracted_list.push(vibe.info_extracted);
    }

    Ok(ProcessedVibes {
        encodings,
        strengths,
        info_extracted_list,
    })
}
