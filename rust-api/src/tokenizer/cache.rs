use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime};

use crate::constants::MAX_TOKENS;
use crate::error::{NovelAIError, Result};

use super::clip::NovelAIClipTokenizer;
use super::t5::{NovelAIT5Tokenizer, PureUnigram};

/// Cache TTL: 7 days
const CACHE_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Maximum response size: 50MB
const MAX_RESPONSE_SIZE: usize = 50 * 1024 * 1024;

/// Maximum decompressed output size: 50MB
const MAX_DECOMPRESSED_SIZE: u64 = 50 * 1024 * 1024;

/// CLIP tokenizer definition URL
const CLIP_TOKENIZER_URL: &str =
    "https://novelai.net/tokenizer/compressed/clip_tokenizer.def?v=2&static=true";

/// T5 tokenizer definition URL
const T5_TOKENIZER_URL: &str =
    "https://novelai.net/tokenizer/compressed/t5_tokenizer.def?v=2&static=true";

// Global singleton caches using tokio::sync::OnceCell (#9/#26 fix)
static CLIP_TOKENIZER: tokio::sync::OnceCell<Arc<NovelAIClipTokenizer>> =
    tokio::sync::OnceCell::const_new();
static T5_TOKENIZER: tokio::sync::OnceCell<Arc<NovelAIT5Tokenizer>> =
    tokio::sync::OnceCell::const_new();

/// Cached HTTP client to reuse connection pool (#48 fix)
static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Get or create the shared HTTP client.
fn get_http_client() -> Result<&'static reqwest::Client> {
    if let Some(client) = HTTP_CLIENT.get() {
        return Ok(client);
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| NovelAIError::Tokenizer(format!("Failed to create HTTP client: {}", e)))?;
    // If another thread initialized it first, that's fine - we just use theirs
    let _ = HTTP_CLIENT.set(client);
    Ok(HTTP_CLIENT.get().unwrap())
}

// =========================================================================
// Cache filename generation
// =========================================================================

/// Sanitize a string to only allow safe filename characters.
fn sanitize_filename_component(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect()
}

/// Generate a cache filename from a URL.
/// Extracts the tokenizer name and version from the URL.
pub fn get_cache_filename(url_str: &str) -> Result<String> {
    let parsed =
        url::Url::parse(url_str).map_err(|e| NovelAIError::Tokenizer(format!("Invalid URL: {}", e)))?;

    let pathname = parsed.path();
    let raw_basename = Path::new(pathname)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let raw_version = parsed
        .query_pairs()
        .find(|(k, _)| k == "v")
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let basename = sanitize_filename_component(raw_basename);
    let version = sanitize_filename_component(&raw_version);

    if basename.is_empty() {
        return Err(NovelAIError::Tokenizer(
            "Invalid tokenizer URL: empty basename after sanitization".to_string(),
        ));
    }

    Ok(format!("{}_v{}.json", basename, version))
}

// =========================================================================
// Disk cache
// =========================================================================

/// Get the cache directory path.
fn cache_dir() -> PathBuf {
    std::env::var("NOVELAI_CACHE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs_or_fallback().join(".cache").join("tokenizers"))
}

/// Get a reasonable base directory for caching.
fn dirs_or_fallback() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Validate that a resolved cache path is within the cache directory. (#10 fix)
fn validate_cache_path(cache_path: &Path) -> Result<()> {
    let cache_base = cache_dir();
    let resolved = cache_path
        .canonicalize()
        .unwrap_or_else(|_| cache_path.to_path_buf());
    let base_resolved = cache_base
        .canonicalize()
        .unwrap_or_else(|_| cache_base.clone());

    if !resolved.starts_with(&base_resolved) && resolved != base_resolved {
        return Err(NovelAIError::Tokenizer(format!(
            "Cache path traversal detected: {}",
            cache_path.display()
        )));
    }
    Ok(())
}

/// Read cached data from disk if it exists and is not expired.
async fn read_from_cache(cache_file: &str) -> Option<String> {
    let cache_path = cache_dir().join(cache_file);

    // Validate the cache path is within the cache directory (#10 fix)
    if validate_cache_path(&cache_path).is_err() {
        return None;
    }

    let metadata = tokio::fs::metadata(&cache_path).await.ok()?;
    let modified = metadata.modified().ok()?;
    let age = SystemTime::now().duration_since(modified).ok()?;

    if age > CACHE_TTL {
        return None;
    }

    tokio::fs::read_to_string(&cache_path).await.ok()
}

/// Write data to cache file.
async fn write_to_cache(cache_file: &str, data: &str) {
    let dir = cache_dir();
    let cache_path = dir.join(cache_file);

    // Validate the cache path is within the cache directory (#10 fix)
    if validate_cache_path(&cache_path).is_err() {
        return;
    }

    // Silently ignore errors (cache write failure is not fatal)
    let _ = tokio::fs::create_dir_all(&dir).await;
    let _ = tokio::fs::write(&cache_path, data).await;
}

// =========================================================================
// Network fetch + decompress
// =========================================================================

/// Fetch and decompress tokenizer data from a URL.
/// Uses disk cache to avoid repeated network requests.
async fn fetch_data(target_url: &str, force_refresh: bool) -> Result<String> {
    let cache_file = get_cache_filename(target_url)?;

    // Try cache first
    if !force_refresh {
        if let Some(cached_data) = read_from_cache(&cache_file).await {
            return Ok(cached_data);
        }
    }

    // Fetch from network using cached HTTP client (#48 fix)
    let client = get_http_client()?;

    let response = client
        .get(target_url)
        .header("User-Agent", "novelai-rust-api/1.0")
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() {
                NovelAIError::Tokenizer(format!(
                    "Failed to connect to tokenizer server: {}",
                    target_url
                ))
            } else if e.is_timeout() {
                NovelAIError::Tokenizer("Request timed out while fetching tokenizer data".to_string())
            } else {
                NovelAIError::Tokenizer(format!("Network error while fetching tokenizer: {}", e))
            }
        })?;

    // Check HTTP status (#11 fix)
    if !response.status().is_success() {
        return Err(NovelAIError::Tokenizer(format!(
            "HTTP request failed with status {}: {}",
            response.status().as_u16(),
            target_url
        )));
    }

    // Check Content-Length before downloading body (#12 fix)
    if let Some(content_length) = response.content_length() {
        if content_length as usize > MAX_RESPONSE_SIZE {
            return Err(NovelAIError::Tokenizer(format!(
                "Response Content-Length too large: {} bytes (max {})",
                content_length, MAX_RESPONSE_SIZE
            )));
        }
    }

    let bytes = response.bytes().await.map_err(|e| {
        NovelAIError::Tokenizer(format!("Failed to read response body: {}", e))
    })?;

    if bytes.len() > MAX_RESPONSE_SIZE {
        return Err(NovelAIError::Tokenizer(format!(
            "Response too large: {} bytes (max {})",
            bytes.len(),
            MAX_RESPONSE_SIZE
        )));
    }

    // Decompress: try raw deflate first, then standard zlib
    let data = decompress_data(&bytes)?;
    let data_str = String::from_utf8(data)
        .map_err(|e| NovelAIError::Tokenizer(format!("Invalid UTF-8 in tokenizer data: {}", e)))?;

    // Save to cache
    write_to_cache(&cache_file, &data_str).await;

    Ok(data_str)
}

/// Try decompressing data using raw deflate first, then standard zlib.
/// Limits decompressed output to MAX_DECOMPRESSED_SIZE to prevent zip bombs (#15 fix).
fn decompress_data(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::read::{DeflateDecoder, ZlibDecoder};
    use std::io::Read;

    // Try raw deflate first (with size limit)
    {
        let decoder = DeflateDecoder::new(data);
        let mut limited = decoder.take(MAX_DECOMPRESSED_SIZE + 1);
        let mut result = Vec::new();
        if limited.read_to_end(&mut result).is_ok() {
            if result.len() as u64 > MAX_DECOMPRESSED_SIZE {
                return Err(NovelAIError::Tokenizer(format!(
                    "Decompressed data exceeds size limit ({} bytes)",
                    MAX_DECOMPRESSED_SIZE
                )));
            }
            return Ok(result);
        }
    }

    // Try zlib (with size limit)
    {
        let decoder = ZlibDecoder::new(data);
        let mut limited = decoder.take(MAX_DECOMPRESSED_SIZE + 1);
        let mut result = Vec::new();
        if limited.read_to_end(&mut result).is_ok() {
            if result.len() as u64 > MAX_DECOMPRESSED_SIZE {
                return Err(NovelAIError::Tokenizer(format!(
                    "Decompressed data exceeds size limit ({} bytes)",
                    MAX_DECOMPRESSED_SIZE
                )));
            }
            return Ok(result);
        }
    }

    Err(NovelAIError::Tokenizer(
        "Failed to decompress tokenizer data (tried raw deflate and zlib)".to_string(),
    ))
}

// =========================================================================
// Public API: get tokenizers
// =========================================================================

/// Get or create the CLIP tokenizer (fetches from network if not cached).
pub async fn get_clip_tokenizer(force_refresh: bool) -> Result<Arc<NovelAIClipTokenizer>> {
    // For force_refresh, skip the OnceCell and fetch directly
    if force_refresh {
        let data_str = fetch_data(CLIP_TOKENIZER_URL, true).await?;
        let json: serde_json::Value = serde_json::from_str(&data_str)
            .map_err(|e| NovelAIError::Tokenizer(format!("Failed to parse CLIP tokenizer JSON: {}", e)))?;

        let text = json
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                NovelAIError::Tokenizer("CLIP tokenizer data missing \"text\" field".to_string())
            })?;

        let tokenizer = Arc::new(NovelAIClipTokenizer::new(text));
        return Ok(tokenizer);
    }

    // Use OnceCell for thread-safe single initialization (#9/#26 fix)
    let tokenizer = CLIP_TOKENIZER
        .get_or_try_init(|| async {
            let data_str = fetch_data(CLIP_TOKENIZER_URL, false).await?;
            let json: serde_json::Value = serde_json::from_str(&data_str)
                .map_err(|e| {
                    NovelAIError::Tokenizer(format!("Failed to parse CLIP tokenizer JSON: {}", e))
                })?;

            let text = json
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    NovelAIError::Tokenizer(
                        "CLIP tokenizer data missing \"text\" field".to_string(),
                    )
                })?;

            Ok(Arc::new(NovelAIClipTokenizer::new(text))) as Result<Arc<NovelAIClipTokenizer>>
        })
        .await?;

    Ok(tokenizer.clone())
}

/// Get or create the T5 tokenizer (fetches from network if not cached).
pub async fn get_t5_tokenizer(force_refresh: bool) -> Result<Arc<NovelAIT5Tokenizer>> {
    // For force_refresh, skip the OnceCell and fetch directly
    if force_refresh {
        let data_str = fetch_data(T5_TOKENIZER_URL, true).await?;
        return parse_t5_tokenizer(&data_str);
    }

    // Use OnceCell for thread-safe single initialization (#9/#26 fix)
    let tokenizer = T5_TOKENIZER
        .get_or_try_init(|| async {
            let data_str = fetch_data(T5_TOKENIZER_URL, false).await?;
            parse_t5_tokenizer(&data_str)
        })
        .await?;

    Ok(tokenizer.clone())
}

/// Parse T5 tokenizer JSON data into a tokenizer instance.
/// Handles JSON errors gracefully without unwrap() (#13 fix).
fn parse_t5_tokenizer(data_str: &str) -> Result<Arc<NovelAIT5Tokenizer>> {
    let json: serde_json::Value = serde_json::from_str(data_str)
        .map_err(|e| NovelAIError::Tokenizer(format!("Failed to parse T5 tokenizer JSON: {}", e)))?;

    // Validate JSON structure
    let model = json.get("model").ok_or_else(|| {
        NovelAIError::Tokenizer("T5 tokenizer data missing \"model\" field".to_string())
    })?;

    let vocab_arr = model
        .get("vocab")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            NovelAIError::Tokenizer(
                "T5 tokenizer data missing or invalid \"model.vocab\" array".to_string(),
            )
        })?;

    let unk_id = model
        .get("unk_id")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            NovelAIError::Tokenizer(
                "T5 tokenizer data missing or invalid \"model.unk_id\" number".to_string(),
            )
        })? as u32;

    // Parse vocab entries with graceful error handling instead of unwrap() (#13 fix)
    let vocab: Vec<(String, f64)> = vocab_arr
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let arr = entry.as_array().ok_or_else(|| {
                NovelAIError::Tokenizer(format!(
                    "T5 vocab entry {} is not an array",
                    idx
                ))
            })?;
            let piece = arr
                .first()
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    NovelAIError::Tokenizer(format!(
                        "T5 vocab entry {} missing string piece",
                        idx
                    ))
                })?
                .to_string();
            let score = arr
                .get(1)
                .and_then(|v| v.as_f64())
                .ok_or_else(|| {
                    NovelAIError::Tokenizer(format!(
                        "T5 vocab entry {} missing numeric score",
                        idx
                    ))
                })?;
            Ok((piece, score))
        })
        .collect::<Result<Vec<_>>>()?;

    let unigram = PureUnigram::new(vocab, unk_id);
    let tokenizer = Arc::new(NovelAIT5Tokenizer::from_pure_unigram(unigram));
    Ok(tokenizer)
}

/// Clear all cached tokenizer instances.
/// Note: With OnceCell, this is a no-op since OnceCell does not support clearing.
/// Force refresh should be used instead via the force_refresh parameter.
pub fn clear_tokenizer_cache() {
    // OnceCell does not support clearing; use force_refresh=true in
    // get_clip_tokenizer / get_t5_tokenizer to bypass the cache.
}

/// Validate that the token count does not exceed MAX_TOKENS (512).
pub async fn validate_token_count(text: &str) -> Result<usize> {
    let tokenizer = get_t5_tokenizer(false).await?;
    let token_count = tokenizer.count_tokens(text);

    if token_count > MAX_TOKENS {
        return Err(NovelAIError::TokenValidation {
            token_count,
            max_tokens: MAX_TOKENS,
        });
    }

    Ok(token_count)
}
