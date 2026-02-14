use regex::Regex;
use std::sync::OnceLock;

fn bracket_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[\[\]{}]").unwrap())
}

fn weight_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Match weight syntax: NUMBER::content::
    // Uses non-greedy .+? to match content between :: pairs.
    // (?s) makes . match newlines.
    RE.get_or_init(|| Regex::new(r"(?s)(-?\d+\.?\d*)?::(.+?)::").unwrap())
}

/// Preprocess text for T5 tokenizer.
///
/// Based on official NovelAI JavaScript (9423.2de67be589ffa59d.js),
/// T5 preprocessing ONLY removes brackets and weight syntax.
/// Unlike CLIP, it does NOT:
/// - Decode HTML entities
/// - Normalize whitespace
/// - Convert to lowercase
pub fn preprocess_t5(text: &str) -> String {
    // 1. Remove brackets [] and {}
    let text = bracket_re().replace_all(text, "");

    // 2. Remove weighting syntax (e.g., "2::content::", "1.5::content::", "-1::content::")
    let text = weight_re().replace_all(&text, "$2");

    text.into_owned()
}
