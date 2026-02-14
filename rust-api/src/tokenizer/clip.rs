use std::collections::{HashMap, HashSet};
use std::num::NonZeroUsize;
use std::sync::Mutex;

use lru::LruCache;
use regex::Regex;

/// BPE cache size limit
const BPE_CACHE_MAX_SIZE: usize = 10_000;

/// Separator for BPE rank keys (Middle Dot + Sunglasses Emoji + Middle Dot)
const BPE_SEPARATOR: &str = "\u{b7}\u{1F60E}\u{b7}";

/// Build byte-to-unicode mapping (GPT-2 style).
/// Maps each byte (0-255) to a unique Unicode character.
fn bytes_to_unicode() -> [char; 256] {
    let mut result = ['\0'; 256];

    // Direct-mapped byte ranges
    let mut bs: Vec<u8> = Vec::new();
    let mut cs: Vec<u32> = Vec::new();

    // '!' to '~' (33-126)
    for b in 33..=126u8 {
        bs.push(b);
    }
    // '¡' to '¬' (161-172)
    for b in 161..=172u8 {
        bs.push(b);
    }
    // '®' to 'ÿ' (174-255)
    for b in 174..=255u8 {
        bs.push(b);
    }

    cs.extend(bs.iter().map(|&b| b as u32));

    // Indirect-mapped: remaining bytes get chars starting at U+0100
    let mut n: u32 = 0;
    for b in 0..=255u8 {
        if !bs.contains(&b) {
            bs.push(b);
            cs.push(256 + n);
            n += 1;
        }
    }

    for i in 0..256 {
        result[bs[i] as usize] = char::from_u32(cs[i]).unwrap();
    }
    result
}

/// Generate the initial vocabulary (256 single-character tokens) in the
/// same order as the bytes_to_unicode mapping.
fn initial_vocab(byte_encoder: &[char; 256]) -> Vec<String> {
    let mut ordered_chars: Vec<char> = Vec::with_capacity(256);

    // Same byte order as bytes_to_unicode
    for b in 33..=126u8 {
        ordered_chars.push(byte_encoder[b as usize]);
    }
    for b in 161..=172u8 {
        ordered_chars.push(byte_encoder[b as usize]);
    }
    for b in 174..=255u8 {
        ordered_chars.push(byte_encoder[b as usize]);
    }
    for b in 0..=255u8 {
        let c = byte_encoder[b as usize];
        if !ordered_chars.contains(&c) {
            ordered_chars.push(c);
        }
    }

    ordered_chars.iter().map(|c| c.to_string()).collect()
}

/// NovelAI CLIP BPE tokenizer.
///
/// Implements Byte-Pair Encoding with:
/// - GPT-2 byte-to-unicode mapping
/// - BPE merge rules from definition file
/// - LRU cache for BPE results
/// - Regex-based pre-tokenization
/// - HTML entity decoding and text normalization
pub struct NovelAIClipTokenizer {
    byte_encoder: [char; 256],
    encoder: HashMap<String, u32>,
    bpe_ranks: HashMap<String, usize>,
    cache: Mutex<LruCache<String, String>>,
    pat: Regex,
}

impl NovelAIClipTokenizer {
    /// Create a new CLIP tokenizer from a definition text.
    /// The definition text contains BPE merge rules (one per line after the header).
    pub fn new(definition_text: &str) -> Self {
        let byte_encoder = bytes_to_unicode();

        let lines: Vec<&str> = definition_text.split('\n').collect();
        let end = std::cmp::min(48895, lines.len());
        let merges_raw = &lines[1..end];
        let merges: Vec<Vec<&str>> = merges_raw
            .iter()
            .map(|line| line.split(' ').collect())
            .collect();

        // Build vocab: initial (256) + initial with </w> (256) + merges + special tokens
        let mut vocab_list: Vec<String> = Vec::new();
        let init = initial_vocab(&byte_encoder);
        for token in &init {
            vocab_list.push(token.clone());
        }
        for token in &init {
            vocab_list.push(format!("{}</w>", token));
        }
        for merge_pair in &merges {
            vocab_list.push(merge_pair.join(""));
        }
        vocab_list.push("<|startoftext|>".to_string());
        vocab_list.push("<|endoftext|>".to_string());

        let mut encoder = HashMap::new();
        for (i, token) in vocab_list.iter().enumerate() {
            encoder.insert(token.clone(), i as u32);
        }

        let mut bpe_ranks = HashMap::new();
        for (i, pair) in merges.iter().enumerate() {
            let key = pair.join(BPE_SEPARATOR);
            bpe_ranks.insert(key, i);
        }

        let mut cache = LruCache::new(NonZeroUsize::new(BPE_CACHE_MAX_SIZE).unwrap());
        cache.put(
            "<|startoftext|>".to_string(),
            "<|startoftext|>".to_string(),
        );
        cache.put("<|endoftext|>".to_string(), "<|endoftext|>".to_string());

        let pat = Regex::new(
            r"<\|startoftext\|>|<\|endoftext\|>|'s|'t|'re|'ve|'m|'ll|'d|[\p{L}]+|[\p{N}]|[^\s\p{L}\p{N}]+",
        )
        .unwrap();

        Self {
            byte_encoder,
            encoder,
            bpe_ranks,
            cache: Mutex::new(cache),
            pat,
        }
    }

    fn bpe(&self, token: &str) -> String {
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(cached) = cache.get(token) {
                return cached.clone();
            }
        }

        let chars: Vec<char> = token.chars().collect();
        if chars.is_empty() {
            return String::new();
        }

        // Build initial word: all chars except last as-is, last char gets </w> appended
        let mut word: Vec<String> = Vec::new();
        for (i, &c) in chars.iter().enumerate() {
            if i < chars.len() - 1 {
                word.push(c.to_string());
            } else {
                word.push(format!("{}</w>", c));
            }
        }

        let mut pairs = Self::get_pairs(&word);
        if pairs.is_empty() {
            return format!("{}</w>", token);
        }

        loop {
            // Find the pair with the lowest rank
            let mut best_pair: Option<(String, String)> = None;
            let mut min_rank = usize::MAX;

            for pair in &pairs {
                let key = format!("{}{}{}", pair.0, BPE_SEPARATOR, pair.1);
                if let Some(&rank) = self.bpe_ranks.get(&key) {
                    if rank < min_rank {
                        min_rank = rank;
                        best_pair = Some(pair.clone());
                    }
                }
            }

            let (first, second) = match best_pair {
                Some(ref pair) => {
                    let key = format!("{}{}{}", pair.0, BPE_SEPARATOR, pair.1);
                    if !self.bpe_ranks.contains_key(&key) {
                        break;
                    }
                    pair.clone()
                }
                None => break,
            };

            // Merge the best pair in the word
            let mut new_word: Vec<String> = Vec::new();
            let mut i = 0;
            while i < word.len() {
                let j = word[i..].iter().position(|w| *w == first).map(|p| p + i);

                match j {
                    None => {
                        new_word.extend_from_slice(&word[i..]);
                        break;
                    }
                    Some(j) => {
                        new_word.extend_from_slice(&word[i..j]);
                        i = j;

                        if word[i] == first && i < word.len() - 1 && word[i + 1] == second {
                            new_word.push(format!("{}{}", first, second));
                            i += 2;
                        } else {
                            new_word.push(word[i].clone());
                            i += 1;
                        }
                    }
                }
            }

            word = new_word;
            if word.len() == 1 {
                break;
            }
            pairs = Self::get_pairs(&word);
        }

        let result = word.join(" ");
        let mut cache = self.cache.lock().unwrap();
        cache.put(token.to_string(), result.clone());
        result
    }

    fn get_pairs(word: &[String]) -> Vec<(String, String)> {
        let mut seen = HashSet::new();
        let mut pairs = Vec::new();

        if word.len() < 2 {
            return pairs;
        }

        for i in 0..word.len() - 1 {
            let key = format!("{}\0{}", word[i], word[i + 1]);
            if seen.insert(key) {
                pairs.push((word[i].clone(), word[i + 1].clone()));
            }
        }

        pairs
    }

    /// Encode text into CLIP token IDs.
    ///
    /// Applies:
    /// 1. Double HTML entity decode
    /// 2. Whitespace normalization
    /// 3. Lowercase conversion
    /// 4. Regex pre-tokenization
    /// 5. Byte-to-unicode translation
    /// 6. BPE encoding
    pub fn encode(&self, text: &str) -> Vec<u32> {
        // Double HTML entity decode, then trim
        let decoded = html_escape::decode_html_entities(text);
        let decoded = html_escape::decode_html_entities(&decoded);
        let decoded = decoded.trim();

        if decoded.is_empty() {
            return vec![];
        }

        // Normalize whitespace, trim, lowercase
        let normalized: String = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
        let lowered = normalized.to_lowercase();

        let mut bpe_tokens: Vec<u32> = Vec::new();

        for mat in self.pat.find_iter(&lowered) {
            let token = mat.as_str();

            // Encode token text to UTF-8 bytes, then translate through byte_encoder
            let token_bytes = token.as_bytes();
            let token_translated: String = token_bytes
                .iter()
                .map(|&b| self.byte_encoder[b as usize])
                .collect();

            let bpe_res = self.bpe(&token_translated);

            for bpe_token in bpe_res.split(' ') {
                if let Some(&id) = self.encoder.get(bpe_token) {
                    bpe_tokens.push(id);
                }
            }
        }

        bpe_tokens
    }
}
