use std::collections::HashMap;
use unicode_normalization::UnicodeNormalization;

use super::preprocess::preprocess_t5;

/// Pure Rust Unigram tokenizer implementation.
///
/// Used as the primary T5 tokenizer backend. Implements:
/// - NFKC normalization
/// - WhitespaceSplit + Metaspace pre-tokenization
/// - Viterbi algorithm for optimal Unigram segmentation
pub struct PureUnigram {
    vocab: HashMap<String, f64>,
    piece_to_id: HashMap<String, u32>,
    unk_id: u32,
    unk_score: f64,
    max_piece_length: usize,
}

impl PureUnigram {
    /// Create a new PureUnigram tokenizer from vocabulary entries.
    ///
    /// `vocab_entries` is a list of (piece, log_score) pairs.
    /// `unk_id` is the token ID for unknown characters.
    pub fn new(vocab_entries: Vec<(String, f64)>, unk_id: u32) -> Self {
        let mut vocab = HashMap::new();
        let mut piece_to_id = HashMap::new();
        let mut max_piece_length: usize = 0;
        let mut min_score: f64 = 0.0;

        for (i, (piece, score)) in vocab_entries.iter().enumerate() {
            vocab.insert(piece.clone(), *score);
            piece_to_id.insert(piece.clone(), i as u32);
            let piece_cp_len = piece.chars().count();
            if piece_cp_len > max_piece_length {
                max_piece_length = piece_cp_len;
            }
            if *score != 0.0 && *score < min_score {
                min_score = *score;
            }
        }

        // SentencePiece uses min_score - kUnkPenalty (10.0) for unknown characters
        let unk_score = min_score - 10.0;

        Self {
            vocab,
            piece_to_id,
            unk_id,
            unk_score,
            max_piece_length,
        }
    }

    /// Look up the token ID for a given piece string.
    pub fn token_to_id(&self, token: &str) -> Option<u32> {
        self.piece_to_id.get(token).copied()
    }

    /// Encode text into token IDs using Unigram model with Viterbi algorithm.
    ///
    /// Pre-tokenization: NFKC normalize -> WhitespaceSplit -> Metaspace (▁ prefix)
    pub fn encode(&self, text: &str) -> Vec<u32> {
        // 1. NFKC normalization
        let normalized: String = text.nfkc().collect();

        // 2. WhitespaceSplit: split on whitespace
        let pieces: Vec<&str> = normalized.split_whitespace().collect();
        if pieces.is_empty() {
            return vec![];
        }

        // 3. Metaspace: prepend ▁ to each piece
        let metaspaced: Vec<String> = pieces.iter().map(|p| format!("\u{2581}{}", p)).collect();

        // 4. Viterbi on each metaspaced piece
        let mut ids: Vec<u32> = Vec::new();
        for piece in &metaspaced {
            ids.extend(self.viterbi(piece));
        }

        ids
    }

    /// Viterbi algorithm for optimal Unigram segmentation.
    ///
    /// Finds the highest-scoring segmentation of the input text into vocab pieces.
    /// Uses code point iteration to correctly handle BMP-external characters (e.g., emoji).
    fn viterbi(&self, text: &str) -> Vec<u32> {
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        if len == 0 {
            return vec![];
        }

        // best[i] = (score, prev_position) for position i (code points 0..i processed)
        let mut best: Vec<(f64, usize)> = vec![(f64::NEG_INFINITY, 0); len + 1];
        best[0] = (0.0, 0);

        for i in 1..=len {
            for l in 1..=std::cmp::min(self.max_piece_length, i) {
                let substr: String = chars[i - l..i].iter().collect();
                if let Some(&score) = self.vocab.get(&substr) {
                    let candidate = best[i - l].0 + score;
                    if candidate > best[i].0 {
                        best[i] = (candidate, i - l);
                    }
                }
            }

            // If no vocab match found, single char fallback to unk
            if best[i].0 == f64::NEG_INFINITY {
                best[i] = (best[i - 1].0 + self.unk_score, i - 1);
            }
        }

        // Backtrack to recover pieces
        let mut pieces: Vec<String> = Vec::new();
        let mut pos = len;
        while pos > 0 {
            let prev = best[pos].1;
            let piece: String = chars[prev..pos].iter().collect();
            pieces.push(piece);
            pos = prev;
        }
        pieces.reverse();

        // Convert pieces to token IDs
        pieces
            .iter()
            .map(|p| self.piece_to_id.get(p.as_str()).copied().unwrap_or(self.unk_id))
            .collect()
    }
}

/// NovelAI T5 tokenizer.
///
/// Wraps a PureUnigram backend. Handles T5-specific preprocessing
/// and EOS token appending.
pub struct NovelAIT5Tokenizer {
    backend: PureUnigram,
    eos_token_id: u32,
}

impl NovelAIT5Tokenizer {
    /// Create from a PureUnigram backend.
    pub fn from_pure_unigram(unigram: PureUnigram) -> Self {
        let eos_id = unigram.token_to_id("</s>").unwrap_or(1);
        Self {
            backend: unigram,
            eos_token_id: eos_id,
        }
    }

    /// Encode text using official NovelAI T5 logic.
    /// Returns the full token array INCLUDING EOS (for model input).
    pub fn encode(&self, text: &str) -> Vec<u32> {
        if text.is_empty() {
            return vec![self.eos_token_id];
        }

        let processed = preprocess_t5(text);
        let mut ids = self.backend.encode(&processed);
        ids.push(self.eos_token_id);
        ids
    }

    /// Count tokens matching official NovelAI UI display.
    /// Returns token count INCLUDING EOS token.
    pub fn count_tokens(&self, text: &str) -> usize {
        self.encode(text).len()
    }
}
