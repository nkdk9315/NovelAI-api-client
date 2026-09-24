//! Qwen byte-level BPE tokenizer used by V5 models.
//!
//! Mirrors the official site's generic BPE encoder:
//! NFC normalize → split out special tokens → split regex → byte-to-unicode → BPE merges.
//! No EOS is appended and prompt weighting syntax is NOT stripped (the site counts raw text).

use std::collections::HashMap;
use std::sync::Mutex;

use fancy_regex::Regex;
use serde::Deserialize;
use unicode_normalization::UnicodeNormalization;

use crate::error::{NovelAIError, Result};

use super::clip::bytes_to_unicode;

const BPE_CACHE_MAX_SIZE: usize = 10_000;

#[derive(Deserialize)]
struct QwenConfig {
    #[serde(rename = "splitRegex")]
    split_regex: String,
    normalization: Option<String>,
}

#[derive(Deserialize)]
struct QwenDefinition {
    config: QwenConfig,
    #[serde(rename = "specialTokens", default)]
    special_tokens: Vec<String>,
    vocab: HashMap<String, u32>,
    merges: Vec<(String, String)>,
}

pub struct NovelAIQwenTokenizer {
    vocab: HashMap<String, u32>,
    bpe_ranks: HashMap<(String, String), usize>,
    specials: Vec<String>,
    split_regex: Regex,
    nfc: bool,
    byte_encoder: [char; 256],
    cache: Mutex<HashMap<String, Vec<u32>>>,
}

impl NovelAIQwenTokenizer {
    /// Build from the decompressed `qwen35_tokenizer.def` JSON.
    pub fn from_json(data: &str) -> Result<Self> {
        let def: QwenDefinition = serde_json::from_str(data)
            .map_err(|e| NovelAIError::Tokenizer(format!("Failed to parse Qwen tokenizer JSON: {}", e)))?;
        let split_regex = Regex::new(&def.config.split_regex)
            .map_err(|e| NovelAIError::Tokenizer(format!("Invalid Qwen split regex: {}", e)))?;
        let bpe_ranks = def
            .merges
            .into_iter()
            .enumerate()
            .map(|(rank, pair)| (pair, rank))
            .collect();
        // Longest first so that overlapping special tokens match greedily
        let mut specials: Vec<String> = def
            .special_tokens
            .into_iter()
            .filter(|s| def.vocab.contains_key(s))
            .collect();
        specials.sort_by(|a, b| b.len().cmp(&a.len()));

        Ok(Self {
            vocab: def.vocab,
            bpe_ranks,
            specials,
            split_regex,
            nfc: def.config.normalization.as_deref() == Some("NFC"),
            byte_encoder: bytes_to_unicode(),
            cache: Mutex::new(HashMap::new()),
        })
    }

    /// Split text into (segment, is_special) parts.
    fn split_specials<'a>(&self, text: &'a str) -> Vec<(&'a str, bool)> {
        let mut parts = Vec::new();
        let mut start = 0;
        let mut i = 0;
        while i < text.len() {
            if let Some(s) = self.specials.iter().find(|s| text[i..].starts_with(s.as_str())) {
                if start < i {
                    parts.push((&text[start..i], false));
                }
                parts.push((&text[i..i + s.len()], true));
                i += s.len();
                start = i;
            } else {
                i += text[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            }
        }
        if start < text.len() {
            parts.push((&text[start..], false));
        }
        parts
    }

    fn bpe(&self, word: &str) -> Vec<u32> {
        if let Some(ids) = self.cache.lock().ok().and_then(|c| c.get(word).cloned()) {
            return ids;
        }

        let mut symbols: Vec<String> = word.chars().map(|c| c.to_string()).collect();
        while symbols.len() > 1 {
            let mut best: Option<(usize, usize)> = None; // (rank, index)
            for i in 0..symbols.len() - 1 {
                if let Some(&rank) = self.bpe_ranks.get(&(symbols[i].clone(), symbols[i + 1].clone())) {
                    if best.map_or(true, |(r, _)| rank < r) {
                        best = Some((rank, i));
                    }
                }
            }
            let Some((_, index)) = best else { break };

            // Merge every occurrence of the best pair, left to right
            let (left, right) = (symbols[index].clone(), symbols[index + 1].clone());
            let mut merged = Vec::with_capacity(symbols.len());
            let mut i = 0;
            while i < symbols.len() {
                if i + 1 < symbols.len() && symbols[i] == left && symbols[i + 1] == right {
                    merged.push(format!("{}{}", left, right));
                    i += 2;
                } else {
                    merged.push(symbols[i].clone());
                    i += 1;
                }
            }
            symbols = merged;
        }

        let ids: Vec<u32> = symbols.iter().filter_map(|s| self.vocab.get(s).copied()).collect();
        if let Ok(mut cache) = self.cache.lock() {
            if cache.len() >= BPE_CACHE_MAX_SIZE {
                cache.clear();
            }
            cache.insert(word.to_string(), ids.clone());
        }
        ids
    }

    /// Encode text into token IDs (no EOS).
    pub fn encode(&self, text: &str) -> Vec<u32> {
        let normalized: String = if self.nfc { text.nfc().collect() } else { text.to_string() };
        let mut ids = Vec::new();
        for (part, special) in self.split_specials(&normalized) {
            if special {
                ids.push(self.vocab[part]);
                continue;
            }
            for m in self.split_regex.find_iter(part).flatten() {
                let unicode: String = m.as_str().bytes().map(|b| self.byte_encoder[b as usize]).collect();
                ids.extend(self.bpe(&unicode));
            }
        }
        ids
    }

    /// Token count as shown by the official site (no EOS).
    pub fn count_tokens(&self, text: &str) -> usize {
        self.encode(text).len()
    }
}
