pub mod image;
pub mod mask;
pub mod vibe;
pub mod charref;

use crate::error::{NovelAIError, Result};
use std::path::{Component, Path};

/// Validate that a path does not contain traversal segments (`..`).
/// Uses segment-level checking to avoid false positives on names like `..abc`.
pub(crate) fn validate_safe_path(path: &str) -> Result<()> {
    let normalized = path.replace('\\', "/");
    let p = Path::new(&normalized);
    for component in p.components() {
        if matches!(component, Component::ParentDir) {
            return Err(NovelAIError::Validation(
                "Path must not contain '..' (path traversal)".to_string(),
            ));
        }
    }
    Ok(())
}
