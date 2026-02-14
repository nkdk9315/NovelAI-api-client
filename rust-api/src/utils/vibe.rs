use crate::constants;
use crate::error::{NovelAIError, Result};
use crate::schemas::VibeItem;

use serde_json::Value;

// =============================================================================
// Result type
// =============================================================================

/// Result of processing vibes for API payload construction.
pub struct ProcessedVibes {
    pub encodings: Vec<String>,
    pub info_extracted_list: Vec<f64>,
}

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

// =============================================================================
// Public Functions
// =============================================================================

/// Load and parse a .naiv4vibe JSON file.
pub fn load_vibe_file(vibe_path: &str) -> Result<Value> {
    let safe_path = sanitize_file_path(vibe_path)?;
    let content = std::fs::read_to_string(&safe_path).map_err(|e| {
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

/// Process an array of vibe items into encodings and information_extracted lists.
pub fn process_vibes(vibes: &[VibeItem], model: &str) -> Result<ProcessedVibes> {
    let mut encodings = Vec::new();
    let mut info_extracted_list = Vec::new();

    for vibe in vibes {
        match vibe {
            VibeItem::Encoded(result) => {
                encodings.push(result.encoding.clone());
                info_extracted_list.push(result.information_extracted);
            }
            VibeItem::FilePath(path) => {
                let data = load_vibe_file(path)?;
                let (encoding, info) = extract_encoding(&data, model)?;
                encodings.push(encoding);
                info_extracted_list.push(info);
            }
            VibeItem::RawEncoding(encoding) => {
                encodings.push(encoding.clone());
                info_extracted_list.push(1.0);
            }
        }
    }

    Ok(ProcessedVibes {
        encodings,
        info_extracted_list,
    })
}
