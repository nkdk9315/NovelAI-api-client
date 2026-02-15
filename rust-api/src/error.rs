use thiserror::Error;

/// Unified error type for the NovelAI API client library.
#[derive(Debug, Error)]
pub enum NovelAIError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Image processing error: {0}")]
    Image(String),

    #[error("Image file size ({file_size_mb:.2} MB) exceeds maximum allowed size ({max_size_mb} MB){}", .file_source.as_deref().map(|s| format!(": {s}")).unwrap_or_default())]
    ImageFileSize {
        file_size_mb: f64,
        max_size_mb: u32,
        file_source: Option<String>,
    },

    #[error("Tokenizer error: {0}")]
    Tokenizer(String),

    #[error("Token validation error: token count {token_count} exceeds max {max_tokens}")]
    TokenValidation {
        token_count: usize,
        max_tokens: usize,
    },

    #[error("API error: {status_code} - {message}")]
    Api {
        status_code: u16,
        message: String,
    },

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, NovelAIError>;
