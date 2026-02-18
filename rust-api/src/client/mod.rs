use std::path::Path;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use secrecy::{ExposeSecret, SecretString};
use sha2::{Digest, Sha256};

use crate::constants;
use crate::error::{NovelAIError, Result};
use crate::schemas::*;
use crate::utils;

pub mod payload;
pub mod response;
pub mod retry;

// =============================================================================
// Logger Trait
// =============================================================================

pub trait Logger: Send + Sync {
    fn warn(&self, message: &str);
    fn error(&self, message: &str);
}

pub struct DefaultLogger;

impl Logger for DefaultLogger {
    fn warn(&self, message: &str) {
        eprintln!("[WARN] {}", message);
    }
    fn error(&self, message: &str) {
        eprintln!("[ERROR] {}", message);
    }
}

// =============================================================================
// AnlasBalance
// =============================================================================

#[derive(Debug, Clone)]
pub struct AnlasBalance {
    pub fixed: u64,
    pub purchased: u64,
    pub total: u64,
    pub tier: u32,
}

// =============================================================================
// NovelAIClient
// =============================================================================

pub struct NovelAIClient {
    api_key: SecretString,
    http_client: reqwest::Client,
    logger: Box<dyn Logger>,
    /// When true, an extra HTTP request is made before and after each API call
    /// to track anlas balance consumption. Defaults to true for backward
    /// compatibility, but can be disabled to halve per-operation latency.
    track_balance: bool,
}

impl NovelAIClient {
    pub fn new(api_key: Option<&str>, logger: Option<Box<dyn Logger>>) -> Result<Self> {
        let api_key_str = api_key
            .map(|s| s.to_string())
            .or_else(|| std::env::var("NOVELAI_API_KEY").ok())
            .unwrap_or_default();

        if api_key_str.is_empty() {
            return Err(NovelAIError::Validation(
                "API key is required. Set NOVELAI_API_KEY environment variable or pass apiKey parameter.".to_string(),
            ));
        }

        let logger = logger.unwrap_or_else(|| Box::new(DefaultLogger));

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_millis(constants::DEFAULT_REQUEST_TIMEOUT_MS))
            .build()
            .map_err(|e| NovelAIError::Other(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            api_key: SecretString::new(api_key_str),
            http_client,
            logger,
            track_balance: true,
        })
    }

    /// Enable or disable automatic balance tracking around API calls.
    ///
    /// When enabled (default), each generate/encode/augment/upscale call makes
    /// two extra HTTP requests (before and after) to track anlas consumption.
    /// Disable this to reduce latency if you don't need balance tracking.
    pub fn set_track_balance(&mut self, enabled: bool) {
        self.track_balance = enabled;
    }

    // =========================================================================
    // Anlas Balance
    // =========================================================================

    pub async fn get_anlas_balance(&self) -> Result<AnlasBalance> {
        let response = retry::fetch_with_retry(
            &self.http_client,
            &constants::subscription_url(),
            reqwest::Method::GET,
            None,
            self.api_key.expose_secret(),
            "GetAnlasBalance",
            &*self.logger,
        )
        .await?;

        let data: AnlasBalanceResponse = response.json().await.map_err(|e| {
            NovelAIError::Parse(format!(
                "Failed to parse subscription response: {}",
                e
            ))
        })?;

        let fixed = data.training_steps_left.fixed;
        let purchased = data.training_steps_left.purchased;

        Ok(AnlasBalance {
            fixed,
            purchased,
            total: fixed + purchased,
            tier: data.tier,
        })
    }

    /// Get balance after an operation and compute consumed anlas.
    async fn get_anlas_after(
        &self,
        anlas_before: Option<u64>,
    ) -> (Option<u64>, Option<u64>) {
        match self.get_anlas_balance().await {
            Ok(balance) => {
                let remaining = balance.total;
                let consumed =
                    anlas_before.map(|before| before.saturating_sub(remaining));
                (Some(remaining), consumed)
            }
            Err(e) => {
                self.logger.warn(&format!(
                    "[NovelAI] Failed to get final Anlas balance: {}",
                    e
                ));
                (None, None)
            }
        }
    }

    // =========================================================================
    // Encode Vibe
    // =========================================================================

    pub async fn encode_vibe(
        &self,
        params: &EncodeVibeParams,
    ) -> Result<VibeEncodeResult> {
        let image_buffer = utils::image::get_image_buffer(&params.image)?;
        let b64_image = BASE64.encode(&image_buffer);

        // SHA256 hash of source image
        let mut hasher = Sha256::new();
        hasher.update(&image_buffer);
        let source_hash = format!("{:x}", hasher.finalize());

        // Get initial balance (only if tracking is enabled)
        let anlas_before = self.try_get_balance_if_tracking().await;

        let payload = serde_json::json!({
            "image": b64_image,
            "information_extracted": params.information_extracted,
            "model": params.model.as_str(),
        });

        let body = payload.to_string();
        let response = retry::fetch_with_retry(
            &self.http_client,
            &constants::encode_url(),
            reqwest::Method::POST,
            Some(&body),
            self.api_key.expose_secret(),
            "VibeEncode",
            &*self.logger,
        )
        .await?;

        let response_bytes = response::get_response_buffer(response).await?;
        let encoding = BASE64.encode(&response_bytes);

        let (anlas_remaining, anlas_consumed) =
            self.get_anlas_after_if_tracking(anlas_before).await;

        let mut result = VibeEncodeResult {
            encoding,
            model: params.model,
            information_extracted: params.information_extracted,
            strength: params.strength,
            source_image_hash: source_hash.clone(),
            created_at: iso_timestamp(),
            saved_path: None,
            anlas_remaining,
            anlas_consumed,
        };

        // Save if requested
        match &params.save {
            SaveTarget::ExactPath(save_path) => {
                match Self::save_vibe(&result, save_path).await {
                    Ok(()) => result.saved_path = Some(save_path.clone()),
                    Err(e) => self.logger.warn(&format!(
                        "[NovelAI] Failed to save vibe file: {}",
                        e
                    )),
                }
            }
            SaveTarget::Directory { dir, filename } => {
                if let Err(e) = Self::ensure_dir(Path::new(dir)).await {
                    self.logger.warn(&format!(
                        "[NovelAI] Failed to create save directory: {}",
                        e
                    ));
                } else {
                    let fname = if let Some(ref custom) = filename {
                        let base = custom.trim_end_matches(".naiv4vibe");
                        format!("{}.naiv4vibe", base)
                    } else {
                        let ts = file_timestamp();
                        format!("{}_{}.naiv4vibe", &source_hash[..12], ts)
                    };
                    let save_path =
                        Path::new(dir).join(&fname).to_string_lossy().to_string();
                    match Self::save_vibe(&result, &save_path).await {
                        Ok(()) => result.saved_path = Some(save_path),
                        Err(e) => self.logger.warn(&format!(
                            "[NovelAI] Failed to save vibe file: {}",
                            e
                        )),
                    }
                }
            }
            SaveTarget::None => {}
        }

        Ok(result)
    }

    // =========================================================================
    // Generate
    // =========================================================================

    pub async fn generate(
        &self,
        params: &GenerateParams,
    ) -> Result<GenerateResult> {
        let seed = params
            .seed
            .unwrap_or_else(|| rand::random::<u32>() as u64);

        // Build the JSON payload
        let (body, use_stream) = self.build_generate_payload(params, seed)?;

        // Get initial balance (only if tracking is enabled)
        let anlas_before = self.try_get_balance_if_tracking().await;

        // Send the request
        let api_url = if use_stream {
            constants::stream_url()
        } else {
            constants::api_url()
        };

        let response = retry::fetch_with_retry(
            &self.http_client,
            &api_url,
            reqwest::Method::POST,
            Some(&body),
            self.api_key.expose_secret(),
            "Generation",
            &*self.logger,
        )
        .await?;

        // Parse and assemble result
        let image_data = self.process_generate_response(response, use_stream).await?;

        let (anlas_remaining, anlas_consumed) =
            self.get_anlas_after_if_tracking(anlas_before).await;

        let char_configs = params.characters.as_deref().unwrap_or(&[]);
        let mut result = GenerateResult {
            image_data,
            seed,
            anlas_remaining,
            anlas_consumed,
            saved_path: None,
        };

        // Save
        self.try_save_generated_image(&mut result, params, char_configs, seed)
            .await;

        Ok(result)
    }

    /// Build the JSON payload for a generate request.
    ///
    /// Returns the serialized JSON body and a flag indicating whether
    /// the streaming endpoint should be used.
    fn build_generate_payload(
        &self,
        params: &GenerateParams,
        seed: u64,
    ) -> Result<(String, bool)> {
        let negative_prompt = params
            .negative_prompt
            .as_deref()
            .unwrap_or(constants::DEFAULT_NEGATIVE);

        // Process character reference
        let char_ref_data = if let Some(ref char_ref) = params.character_reference {
            Some(utils::charref::process_character_references(
                std::slice::from_ref(char_ref),
            )?)
        } else {
            None
        };

        // Process vibes
        let mut vibe_encodings: Vec<String> = Vec::new();
        let mut vibe_info_list: Vec<f64> = Vec::new();
        let mut vibe_strengths_list: Option<Vec<f64>> = None;

        if let Some(ref vibes) = params.vibes {
            if !vibes.is_empty() {
                let processed =
                    utils::vibe::process_vibes(vibes, params.model.as_str())?;
                vibe_encodings = processed.encodings;
                vibe_info_list = processed.info_extracted_list;
                vibe_strengths_list = Some(processed.strengths);
            }
        }

        // Character configs
        let char_configs = params.characters.as_deref().unwrap_or(&[]);
        let char_captions: Vec<CaptionDict> =
            char_configs.iter().map(character_to_caption_dict).collect();
        let char_neg_captions: Vec<CaptionDict> = char_configs
            .iter()
            .map(character_to_negative_caption_dict)
            .collect();

        // Build payload
        let mut payload_val =
            payload::build_base_payload(params, seed, negative_prompt);
        payload::apply_img2img_params(&mut payload_val, params, seed)?;
        payload::apply_infill_params(&mut payload_val, params, seed)?;
        payload::apply_vibe_params(
            &mut payload_val,
            &vibe_encodings,
            &vibe_strengths_list,
            &vibe_info_list,
        );
        if let Some(ref crd) = char_ref_data {
            payload::apply_char_ref_params(&mut payload_val, crd);
        }
        payload::build_v4_prompt_structure(
            &mut payload_val,
            &params.prompt,
            negative_prompt,
            &char_captions,
            &char_neg_captions,
        );
        payload::apply_character_prompts(&mut payload_val, char_configs);

        let use_stream = params.character_reference.is_some()
            || params.action.is_infill()
            || params.action.is_img2img();

        let body = serde_json::to_string(&payload_val)?;
        Ok((body, use_stream))
    }

    /// Process the HTTP response from a generate request, extracting image data.
    async fn process_generate_response(
        &self,
        response: reqwest::Response,
        use_stream: bool,
    ) -> Result<Vec<u8>> {
        let response_buffer = response::get_response_buffer(response).await?;
        if use_stream {
            response::parse_stream_response(&response_buffer, &*self.logger)
        } else {
            response::parse_zip_response(&response_buffer)
        }
    }

    // =========================================================================
    // Augment Image
    // =========================================================================

    pub async fn augment_image(
        &self,
        params: &AugmentParams,
    ) -> Result<AugmentResult> {
        let (width, height, image_buffer) =
            utils::image::get_image_dimensions(&params.image)?;

        // Reject images exceeding MAX_PIXELS (matches official site behavior)
        let total_pixels = (width as u64) * (height as u64);
        if total_pixels > constants::MAX_PIXELS {
            return Err(NovelAIError::Validation(format!(
                "Image resolution too high for augment ({}x{} = {} pixels, max: {}). \
                 Resize the image to {} pixels or fewer before augmenting.",
                width, height, total_pixels, constants::MAX_PIXELS, constants::MAX_PIXELS
            )));
        }
        let b64_image = BASE64.encode(&image_buffer);

        let anlas_before = self.try_get_balance_if_tracking().await;

        let mut payload = serde_json::json!({
            "req_type": params.req_type.as_str(),
            "use_new_shared_trial": true,
            "width": width,
            "height": height,
            "image": b64_image,
        });

        // Add prompt/defry for colorize and emotion
        match params.req_type {
            constants::AugmentReqType::Colorize => {
                if let Some(ref prompt) = params.prompt {
                    payload["prompt"] =
                        serde_json::Value::String(prompt.clone());
                }
                payload["defry"] = serde_json::json!(
                    params.defry.unwrap_or(constants::DEFAULT_DEFRY)
                );
            }
            constants::AugmentReqType::Emotion => {
                if let Some(ref prompt) = params.prompt {
                    payload["prompt"] =
                        serde_json::Value::String(format!("{};;", prompt));
                }
                payload["defry"] = serde_json::json!(
                    params.defry.unwrap_or(constants::DEFAULT_DEFRY)
                );
            }
            _ => {}
        }

        let body = payload.to_string();
        let response = retry::fetch_with_retry(
            &self.http_client,
            &constants::augment_url(),
            reqwest::Method::POST,
            Some(&body),
            self.api_key.expose_secret(),
            "Augment",
            &*self.logger,
        )
        .await?;

        let response_buffer =
            response::get_response_buffer(response).await?;
        let image_data = response::parse_zip_response(&response_buffer)?;

        let (anlas_remaining, anlas_consumed) =
            self.get_anlas_after_if_tracking(anlas_before).await;

        let mut result = AugmentResult {
            image_data,
            req_type: params.req_type,
            anlas_remaining,
            anlas_consumed,
            saved_path: None,
        };

        // Save
        self.try_save_result_image(
            &result.image_data,
            &mut result.saved_path,
            &params.save,
            params.req_type.as_str(),
            "augmented image",
        )
        .await;

        Ok(result)
    }

    // =========================================================================
    // Upscale Image
    // =========================================================================

    pub async fn upscale_image(
        &self,
        params: &UpscaleParams,
    ) -> Result<UpscaleResult> {
        let (width, height, image_buffer) =
            utils::image::get_image_dimensions(&params.image)?;

        // Validate upscale pixel limit (matches official site behavior)
        let pixels = (width as u64) * (height as u64);
        if pixels > constants::UPSCALE_MAX_PIXELS {
            return Err(NovelAIError::Validation(format!(
                "Image resolution too high for upscale ({}x{} = {} pixels, max: {}). \
                Resize the image to {} pixels or fewer before upscaling.",
                width, height, pixels, constants::UPSCALE_MAX_PIXELS, constants::UPSCALE_MAX_PIXELS
            )));
        }

        let b64_image = BASE64.encode(&image_buffer);

        let anlas_before = self.try_get_balance_if_tracking().await;

        let payload = serde_json::json!({
            "image": b64_image,
            "width": width,
            "height": height,
            "scale": params.scale,
        });

        let body = payload.to_string();
        let response = retry::fetch_with_retry(
            &self.http_client,
            &constants::upscale_url(),
            reqwest::Method::POST,
            Some(&body),
            self.api_key.expose_secret(),
            "Upscale",
            &*self.logger,
        )
        .await?;

        let response_buffer =
            response::get_response_buffer(response).await?;

        // Response may be ZIP or raw image
        let image_data = if response_buffer.len() > 1
            && response_buffer[0] == 0x50
            && response_buffer[1] == 0x4b
        {
            response::parse_zip_response(&response_buffer)?
        } else {
            response_buffer
        };

        let (anlas_remaining, anlas_consumed) =
            self.get_anlas_after_if_tracking(anlas_before).await;

        let output_width = width * params.scale;
        let output_height = height * params.scale;

        let mut result = UpscaleResult {
            image_data,
            scale: params.scale,
            output_width,
            output_height,
            anlas_remaining,
            anlas_consumed,
            saved_path: None,
        };

        // Save
        let prefix = format!("upscale_{}x", params.scale);
        self.try_save_result_image(
            &result.image_data,
            &mut result.saved_path,
            &params.save,
            &prefix,
            "upscaled image",
        )
        .await;

        Ok(result)
    }

    // =========================================================================
    // Private Helpers: Balance
    // =========================================================================

    /// Get balance before an operation, only if balance tracking is enabled.
    async fn try_get_balance_if_tracking(&self) -> Option<u64> {
        if !self.track_balance {
            return None;
        }
        match self.get_anlas_balance().await {
            Ok(balance) => Some(balance.total),
            Err(e) => {
                self.logger.warn(&format!(
                    "[NovelAI] Failed to get initial Anlas balance: {}",
                    e
                ));
                None
            }
        }
    }

    /// Get balance after an operation and compute consumed anlas,
    /// only if balance tracking is enabled.
    async fn get_anlas_after_if_tracking(
        &self,
        anlas_before: Option<u64>,
    ) -> (Option<u64>, Option<u64>) {
        if !self.track_balance {
            return (None, None);
        }
        self.get_anlas_after(anlas_before).await
    }

    // =========================================================================
    // Private Helpers: File Save
    // =========================================================================

    fn validate_save_path(save_path: &str) -> Result<std::path::PathBuf> {
        let normalized = std::path::Path::new(save_path);
        let normalized_str = normalized.to_string_lossy();
        if normalized_str.contains("..") {
            return Err(NovelAIError::Validation(format!(
                "Invalid save path (path traversal detected): {}",
                save_path
            )));
        }
        Ok(normalized.to_path_buf())
    }

    async fn ensure_dir(dir: &Path) -> Result<()> {
        tokio::fs::create_dir_all(dir)
            .await
            .map_err(NovelAIError::Io)
    }

    async fn save_to_file(save_path: &str, data: &[u8]) -> Result<()> {
        let path = Self::validate_save_path(save_path)?;
        if let Some(parent) = path.parent() {
            Self::ensure_dir(parent).await?;
        }
        tokio::fs::write(&path, data)
            .await
            .map_err(NovelAIError::Io)
    }

    async fn save_vibe(
        result: &VibeEncodeResult,
        save_path: &str,
    ) -> Result<()> {
        let model_key = result.model.model_key();
        let hash = &result.source_image_hash;
        let name = if hash.len() >= 12 {
            format!("{}-{}", &hash[..6], &hash[hash.len() - 6..])
        } else {
            hash.clone()
        };

        let vibe_data = serde_json::json!({
            "identifier": "novelai-vibe-transfer",
            "version": 1,
            "type": "encoding",
            "id": result.source_image_hash,
            "encodings": {
                (model_key): {
                    "unknown": {
                        "encoding": result.encoding,
                        "params": {
                            "information_extracted": result.information_extracted,
                        }
                    }
                }
            },
            "name": name,
            "createdAt": result.created_at,
            "importInfo": {
                "model": result.model.as_str(),
                "information_extracted": result.information_extracted,
                "strength": result.strength,
            }
        });

        let json_str = serde_json::to_string_pretty(&vibe_data)?;
        Self::save_to_file(save_path, json_str.as_bytes()).await
    }

    /// Save generated image (generate-specific logic for filenames).
    async fn try_save_generated_image(
        &self,
        result: &mut GenerateResult,
        params: &GenerateParams,
        char_configs: &[CharacterConfig],
        seed: u64,
    ) {
        match &params.save {
            SaveTarget::ExactPath(save_path) => {
                match Self::save_to_file(save_path, &result.image_data).await {
                    Ok(()) => result.saved_path = Some(save_path.clone()),
                    Err(e) => self.logger.warn(&format!(
                        "[NovelAI] Failed to save image: {}",
                        e
                    )),
                }
            }
            SaveTarget::Directory { dir: save_dir, .. } => {
                if let Err(e) = Self::ensure_dir(Path::new(save_dir)).await {
                    self.logger.warn(&format!(
                        "[NovelAI] Failed to save image: {}",
                        e
                    ));
                    return;
                }
                let mut prefix = if params.action.is_img2img() {
                    "img2img".to_string()
                } else {
                    "gen".to_string()
                };
                if !char_configs.is_empty() {
                    prefix.push_str("_multi");
                }
                let ts = file_timestamp();
                let filename = format!("{}_{}_{}.png", prefix, ts, seed);
                let save_path = Path::new(save_dir)
                    .join(&filename)
                    .to_string_lossy()
                    .to_string();
                match Self::save_to_file(&save_path, &result.image_data).await {
                    Ok(()) => result.saved_path = Some(save_path),
                    Err(e) => self.logger.warn(&format!(
                        "[NovelAI] Failed to save image: {}",
                        e
                    )),
                }
            }
            SaveTarget::None => {}
        }
    }

    /// Generic save for augment/upscale results.
    async fn try_save_result_image(
        &self,
        image_data: &[u8],
        saved_path: &mut Option<String>,
        save: &SaveTarget,
        prefix: &str,
        description: &str,
    ) {
        match save {
            SaveTarget::ExactPath(sp) => {
                match Self::save_to_file(sp, image_data).await {
                    Ok(()) => *saved_path = Some(sp.clone()),
                    Err(e) => self.logger.warn(&format!(
                        "[NovelAI] Failed to save {}: {}",
                        description, e
                    )),
                }
            }
            SaveTarget::Directory { dir: sd, .. } => {
                if let Err(e) = Self::ensure_dir(Path::new(sd)).await {
                    self.logger.warn(&format!(
                        "[NovelAI] Failed to save {}: {}",
                        description, e
                    ));
                    return;
                }
                let ts = file_timestamp();
                let rand_hex = format!("{:04x}", rand::random::<u16>());
                let filename = format!("{}_{}_{}.png", prefix, ts, rand_hex);
                let sp = Path::new(sd)
                    .join(&filename)
                    .to_string_lossy()
                    .to_string();
                match Self::save_to_file(&sp, image_data).await {
                    Ok(()) => *saved_path = Some(sp),
                    Err(e) => self.logger.warn(&format!(
                        "[NovelAI] Failed to save {}: {}",
                        description, e
                    )),
                }
            }
            SaveTarget::None => {}
        }
    }
}

// =============================================================================
// Timestamp Utilities (no chrono dependency)
// =============================================================================

/// Generate an ISO 8601 timestamp string (e.g., "2026-02-14T09:49:00.000Z").
fn iso_timestamp() -> String {
    let (y, mo, d, h, mi, s) = utc_now();
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.000Z",
        y, mo, d, h, mi, s
    )
}

/// Generate a compact timestamp for filenames (e.g., "20260214094900").
fn file_timestamp() -> String {
    let (y, mo, d, h, mi, s) = utc_now();
    format!(
        "{:04}{:02}{:02}{:02}{:02}{:02}",
        y, mo, d, h, mi, s
    )
}

/// Get current UTC date/time components using Hinnant's algorithm.
fn utc_now() -> (i64, u32, u32, u64, u64, u64) {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Civil date from Unix timestamp (Howard Hinnant's algorithm)
    let z = (secs / 86400) as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    let time_of_day = secs % 86400;
    let h = time_of_day / 3600;
    let mi = (time_of_day % 3600) / 60;
    let s = time_of_day % 60;

    (y, m, d, h, mi, s)
}
