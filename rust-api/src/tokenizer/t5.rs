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
        // Initialize min_score to INFINITY so it correctly picks the smallest score (#32 fix)
        let mut min_score: f64 = f64::INFINITY;

        for (i, (piece, score)) in vocab_entries.iter().enumerate() {
            vocab.insert(piece.clone(), *score);
            piece_to_id.insert(piece.clone(), i as u32);
            let piece_cp_len = piece.chars().count();
            if piece_cp_len > max_piece_length {
                max_piece_length = piece_cp_len;
            }
            // Skip NaN values to prevent contamination (#32 fix)
            if !score.is_nan() && *score != 0.0 && *score < min_score {
                min_score = *score;
            }
        }

        // If no valid non-zero score was found, use a reasonable default
        if min_score.is_infinite() {
            min_score = -10.0;
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
    /// Pre-tokenization: NFKC normalize -> WhitespaceSplit -> Metaspace (underline prefix)
    pub fn encode(&self, text: &str) -> Vec<u32> {
        // 1. NFKC normalization
        let normalized: String = text.nfkc().collect();

        // 2. WhitespaceSplit: split on whitespace
        let pieces: Vec<&str> = normalized.split_whitespace().collect();
        if pieces.is_empty() {
            return vec![];
        }

        // 3. Metaspace: prepend underline to each piece
        let metaspaced: Vec<String> = pieces.iter().map(|p| format!("\u{2581}{}", p)).collect();

        // 4. Viterbi on each metaspaced piece
        let mut ids: Vec<u32> = Vec::new();
        for piece in &metaspaced {
            ids.extend(self.viterbi(piece));
        }

        ids
    }

    /// Count tokens without building a full Vec of token IDs (#47 fix).
    /// Uses Viterbi internally but only counts the resulting tokens.
    pub fn count_tokens_only(&self, text: &str) -> usize {
        // 1. NFKC normalization
        let normalized: String = text.nfkc().collect();

        // 2. WhitespaceSplit: split on whitespace
        let pieces: Vec<&str> = normalized.split_whitespace().collect();
        if pieces.is_empty() {
            return 0;
        }

        // 3. Metaspace: prepend underline to each piece
        let metaspaced: Vec<String> = pieces.iter().map(|p| format!("\u{2581}{}", p)).collect();

        // 4. Viterbi count-only on each metaspaced piece
        let mut count: usize = 0;
        for piece in &metaspaced {
            count += self.viterbi_count(piece);
        }

        count
    }

    /// Viterbi algorithm for optimal Unigram segmentation.
    ///
    /// Finds the highest-scoring segmentation of the input text into vocab pieces.
    /// Pre-computes byte offsets from character indices for efficient slicing (#46 fix).
    fn viterbi(&self, text: &str) -> Vec<u32> {
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        if len == 0 {
            return vec![];
        }

        // Pre-compute byte offsets for each character index (#46 fix)
        // byte_offsets[i] = byte position where char i starts in the original text
        let byte_offsets: Vec<usize> = {
            let mut offsets = Vec::with_capacity(len + 1);
            offsets.push(0);
            for ch in text.chars() {
                let last = *offsets.last().unwrap();
                offsets.push(last + ch.len_utf8());
            }
            offsets
        };

        // best[i] = (score, prev_position) for position i (code points 0..i processed)
        let mut best: Vec<(f64, usize)> = vec![(f64::NEG_INFINITY, 0); len + 1];
        best[0] = (0.0, 0);

        for i in 1..=len {
            for l in 1..=std::cmp::min(self.max_piece_length, i) {
                // Use pre-computed byte offsets for efficient &str slicing (#46 fix)
                let start_byte = byte_offsets[i - l];
                let end_byte = byte_offsets[i];
                let substr = &text[start_byte..end_byte];
                if let Some(&score) = self.vocab.get(substr) {
                    // Skip NaN scores to prevent DP table contamination (#32 fix)
                    if score.is_nan() {
                        continue;
                    }
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

        // Backtrack to recover pieces and convert to token IDs
        let mut pieces: Vec<&str> = Vec::new();
        let mut pos = len;
        while pos > 0 {
            let prev = best[pos].1;
            let start_byte = byte_offsets[prev];
            let end_byte = byte_offsets[pos];
            pieces.push(&text[start_byte..end_byte]);
            pos = prev;
        }
        pieces.reverse();

        // Convert pieces to token IDs
        pieces
            .iter()
            .map(|p| self.piece_to_id.get(*p).copied().unwrap_or(self.unk_id))
            .collect()
    }

    /// Count-only Viterbi: returns the number of tokens without building vecs (#47 fix).
    fn viterbi_count(&self, text: &str) -> usize {
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        if len == 0 {
            return 0;
        }

        // Pre-compute byte offsets for each character index
        let byte_offsets: Vec<usize> = {
            let mut offsets = Vec::with_capacity(len + 1);
            offsets.push(0);
            for ch in text.chars() {
                let last = *offsets.last().unwrap();
                offsets.push(last + ch.len_utf8());
            }
            offsets
        };

        // best[i] = (score, prev_position) for position i
        let mut best: Vec<(f64, usize)> = vec![(f64::NEG_INFINITY, 0); len + 1];
        best[0] = (0.0, 0);

        for i in 1..=len {
            for l in 1..=std::cmp::min(self.max_piece_length, i) {
                let start_byte = byte_offsets[i - l];
                let end_byte = byte_offsets[i];
                let substr = &text[start_byte..end_byte];
                if let Some(&score) = self.vocab.get(substr) {
                    if score.is_nan() {
                        continue;
                    }
                    let candidate = best[i - l].0 + score;
                    if candidate > best[i].0 {
                        best[i] = (candidate, i - l);
                    }
                }
            }

            if best[i].0 == f64::NEG_INFINITY {
                best[i] = (best[i - 1].0 + self.unk_score, i - 1);
            }
        }

        // Count tokens by backtracking without collecting pieces
        let mut count = 0;
        let mut pos = len;
        while pos > 0 {
            let prev = best[pos].1;
            count += 1;
            pos = prev;
        }

        count
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
    /// Uses count-only code path to avoid building full token Vec (#47 fix).
    pub fn count_tokens(&self, text: &str) -> usize {
        if text.is_empty() {
            return 1; // EOS token only
        }

        let processed = preprocess_t5(text);
        self.backend.count_tokens_only(&processed) + 1 // +1 for EOS
    }
}
