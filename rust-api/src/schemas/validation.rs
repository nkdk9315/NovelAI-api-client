use crate::constants::*;
use crate::error::{NovelAIError, Result};
use crate::tokenizer::cache::get_t5_tokenizer;
use crate::utils::validate_safe_path;
use super::types::*;

// =============================================================================
// Utility Functions
// =============================================================================

/// Validate that a floating-point value is in the unit range [0.0, 1.0].
fn validate_unit_range(value: f64, field: &str) -> Result<()> {
    if !(0.0..=1.0).contains(&value) {
        return Err(NovelAIError::Validation(
            format!("{} must be between 0.0 and 1.0, got {}", field, value),
        ));
    }
    Ok(())
}

/// Validate a `SaveTarget` by checking that any embedded paths are safe
/// (no path traversal).  Mutual-exclusion of save_path/save_dir is
/// impossible by construction since `SaveTarget` is an enum.
pub(crate) fn validate_save_target(save: &SaveTarget) -> Result<()> {
    match save {
        SaveTarget::None => Ok(()),
        SaveTarget::ExactPath(path) => validate_safe_path(path),
        SaveTarget::Directory { dir, filename: _ } => validate_safe_path(dir),
    }
}

/// Validate that an ImageInput is not empty.
pub(crate) fn validate_image_input_not_empty(input: &ImageInput) -> Result<()> {
    if input.is_empty() {
        return Err(NovelAIError::Validation(
            "image must not be empty".to_string(),
        ));
    }
    Ok(())
}

// =============================================================================
// CharacterConfig
// =============================================================================

impl CharacterConfig {
    pub fn validate(&self) -> Result<()> {
        if self.prompt.is_empty() {
            return Err(NovelAIError::Validation(
                "prompt must not be empty".to_string(),
            ));
        }
        validate_unit_range(self.center_x, "center_x")?;
        validate_unit_range(self.center_y, "center_y")?;
        Ok(())
    }
}

// =============================================================================
// CharacterReferenceConfig
// =============================================================================

impl CharacterReferenceConfig {
    pub fn validate(&self) -> Result<()> {
        validate_image_input_not_empty(&self.image)?;
        validate_unit_range(self.strength, "strength")?;
        validate_unit_range(self.fidelity, "fidelity")?;
        // mode is validated by the type system (CharRefMode enum)
        Ok(())
    }
}

// =============================================================================
// VibeEncodeResult
// =============================================================================

impl VibeEncodeResult {
    pub fn validate(&self) -> Result<()> {
        if self.encoding.is_empty() {
            return Err(NovelAIError::Validation(
                "encoding must not be empty".to_string(),
            ));
        }
        if self.encoding.len() > MAX_VIBE_ENCODING_LENGTH {
            return Err(NovelAIError::Validation(format!(
                "encoding length ({}) exceeds maximum ({})",
                self.encoding.len(),
                MAX_VIBE_ENCODING_LENGTH
            )));
        }
        if !self
            .encoding
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
        {
            return Err(NovelAIError::Validation(
                "encoding must be valid base64".to_string(),
            ));
        }
        validate_unit_range(self.information_extracted, "information_extracted")?;
        validate_unit_range(self.strength, "strength")?;
        if self.source_image_hash.len() != 64 {
            return Err(NovelAIError::Validation(
                "source_image_hash must be exactly 64 characters".to_string(),
            ));
        }
        if !self.source_image_hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(NovelAIError::Validation(
                "source_image_hash must contain only hex digits".to_string(),
            ));
        }
        Ok(())
    }
}

// =============================================================================
// GenerateParams
// =============================================================================

impl GenerateParams {
    pub fn validate(&self) -> Result<()> {
        self.validate_dimensions()?;
        self.validate_steps()?;
        self.validate_scale()?;
        self.validate_cfg_rescale()?;
        self.validate_seed()?;
        self.validate_img2img_params()?;
        self.validate_vibes_charref_exclusion()?;
        self.validate_vibe_params()?;
        self.validate_pixel_constraints()?;
        self.validate_save_options()?;
        self.validate_characters()?;
        self.validate_character_reference()?;
        self.validate_vibes()?;
        Ok(())
    }

    /// Async validation including token count checks.
    /// Runs all synchronous validations first, then checks token counts.
    pub async fn validate_async(&self) -> Result<()> {
        self.validate()?;
        self.validate_token_counts().await?;
        Ok(())
    }

    async fn validate_token_counts(&self) -> Result<()> {
        let tokenizer = match get_t5_tokenizer(false).await {
            Ok(t) => t,
            Err(_) => return Ok(()), // Skip if tokenizer unavailable (matches TS behavior)
        };

        // Positive prompt total
        let mut positive_total = 0usize;
        if !self.prompt.is_empty() {
            positive_total += tokenizer.count_tokens(&self.prompt);
        }
        if let Some(chars) = &self.characters {
            for c in chars {
                if !c.prompt.is_empty() {
                    positive_total += tokenizer.count_tokens(&c.prompt);
                }
            }
        }
        if positive_total > MAX_TOKENS {
            return Err(NovelAIError::Validation(format!(
                "Total positive prompt token count ({}) exceeds maximum ({})",
                positive_total, MAX_TOKENS
            )));
        }

        // Negative prompt total
        let mut negative_total = 0usize;
        if let Some(neg) = &self.negative_prompt {
            if !neg.is_empty() {
                negative_total += tokenizer.count_tokens(neg);
            }
        }
        if let Some(chars) = &self.characters {
            for c in chars {
                if !c.negative_prompt.is_empty() {
                    negative_total += tokenizer.count_tokens(&c.negative_prompt);
                }
            }
        }
        if negative_total > MAX_TOKENS {
            return Err(NovelAIError::Validation(format!(
                "Total negative prompt token count ({}) exceeds maximum ({})",
                negative_total, MAX_TOKENS
            )));
        }

        Ok(())
    }

    fn validate_dimensions(&self) -> Result<()> {
        if !(MIN_DIMENSION..=MAX_GENERATION_DIMENSION).contains(&self.width) {
            return Err(NovelAIError::Validation(format!(
                "width must be between {} and {}",
                MIN_DIMENSION, MAX_GENERATION_DIMENSION
            )));
        }
        if !self.width.is_multiple_of(64) {
            return Err(NovelAIError::Validation(
                "Width must be a multiple of 64".to_string(),
            ));
        }
        if !(MIN_DIMENSION..=MAX_GENERATION_DIMENSION).contains(&self.height) {
            return Err(NovelAIError::Validation(format!(
                "height must be between {} and {}",
                MIN_DIMENSION, MAX_GENERATION_DIMENSION
            )));
        }
        if !self.height.is_multiple_of(64) {
            return Err(NovelAIError::Validation(
                "Height must be a multiple of 64".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_steps(&self) -> Result<()> {
        if !(MIN_STEPS..=MAX_STEPS).contains(&self.steps) {
            return Err(NovelAIError::Validation(format!(
                "steps must be between {} and {}",
                MIN_STEPS, MAX_STEPS
            )));
        }
        Ok(())
    }

    fn validate_scale(&self) -> Result<()> {
        if !(MIN_SCALE..=MAX_SCALE).contains(&self.scale) {
            return Err(NovelAIError::Validation(format!(
                "scale must be between {} and {}",
                MIN_SCALE, MAX_SCALE
            )));
        }
        Ok(())
    }

    fn validate_cfg_rescale(&self) -> Result<()> {
        validate_unit_range(self.cfg_rescale, "cfg_rescale")
    }

    fn validate_seed(&self) -> Result<()> {
        if let Some(seed) = self.seed {
            if seed > MAX_SEED as u64 {
                return Err(NovelAIError::Validation(format!(
                    "seed must be between 0 and {}",
                    MAX_SEED
                )));
            }
        }
        Ok(())
    }

    /// Validate parameters embedded in action variants (Img2Img, Infill).
    fn validate_img2img_params(&self) -> Result<()> {
        match &self.action {
            GenerateAction::Generate => Ok(()),
            GenerateAction::Img2Img { source_image, strength, noise } => {
                validate_image_input_not_empty(source_image)?;
                validate_unit_range(*strength, "img2img strength")?;
                validate_unit_range(*noise, "img2img noise")?;
                Ok(())
            }
            GenerateAction::Infill {
                source_image,
                mask,
                mask_strength,
                color_correct: _,
                hybrid_strength,
                hybrid_noise,
            } => {
                validate_image_input_not_empty(source_image)?;
                validate_image_input_not_empty(mask)?;
                if !(0.01..=1.0).contains(mask_strength) {
                    return Err(NovelAIError::Validation(
                        "mask_strength must be between 0.01 and 1.0".to_string(),
                    ));
                }
                if let Some(hs) = hybrid_strength {
                    if !(0.01..=0.99).contains(hs) {
                        return Err(NovelAIError::Validation(
                            "hybrid_img2img_strength must be between 0.01 and 0.99".to_string(),
                        ));
                    }
                }
                if let Some(hn) = hybrid_noise {
                    if !(0.0..=0.99).contains(hn) {
                        return Err(NovelAIError::Validation(
                            "hybrid_img2img_noise must be between 0.0 and 0.99".to_string(),
                        ));
                    }
                }
                Ok(())
            }
        }
    }

    /// Validate that vibes and character_reference are not used together.
    fn validate_vibes_charref_exclusion(&self) -> Result<()> {
        let has_vibes = self
            .vibes
            .as_ref()
            .is_some_and(|v| !v.is_empty());
        if has_vibes && self.character_reference.is_some() {
            return Err(NovelAIError::Validation(
                "vibes and character_reference cannot be used together.".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate individual VibeConfig items (strength and info_extracted ranges).
    fn validate_vibe_params(&self) -> Result<()> {
        if let Some(vibes) = &self.vibes {
            for (i, vibe_config) in vibes.iter().enumerate() {
                validate_unit_range(
                    vibe_config.strength,
                    &format!("vibes[{}].strength", i),
                )?;
                validate_unit_range(
                    vibe_config.info_extracted,
                    &format!("vibes[{}].info_extracted", i),
                )?;
            }
        }
        Ok(())
    }

    fn validate_pixel_constraints(&self) -> Result<()> {
        let total_pixels = self.width as u64 * self.height as u64;
        if total_pixels > MAX_PIXELS {
            return Err(NovelAIError::Validation(format!(
                "Total pixels ({}) exceeds limit ({}). Current: {}x{}",
                total_pixels, MAX_PIXELS, self.width, self.height
            )));
        }
        Ok(())
    }

    fn validate_save_options(&self) -> Result<()> {
        validate_save_target(&self.save)
    }

    fn validate_characters(&self) -> Result<()> {
        if let Some(ref characters) = self.characters {
            if characters.len() > MAX_CHARACTERS {
                return Err(NovelAIError::Validation(format!(
                    "characters count ({}) exceeds maximum ({})",
                    characters.len(),
                    MAX_CHARACTERS
                )));
            }
            for character in characters {
                character.validate()?;
            }
        }
        Ok(())
    }

    fn validate_character_reference(&self) -> Result<()> {
        if let Some(ref char_ref) = self.character_reference {
            char_ref.validate()?;
        }
        Ok(())
    }

    fn validate_vibes(&self) -> Result<()> {
        if let Some(ref vibes) = self.vibes {
            if vibes.len() > MAX_VIBES {
                return Err(NovelAIError::Validation(format!(
                    "vibes count ({}) exceeds maximum ({})",
                    vibes.len(),
                    MAX_VIBES
                )));
            }
            for vibe_config in vibes {
                match &vibe_config.item {
                    VibeItem::FilePath(path) => {
                        if path.as_os_str().is_empty() {
                            return Err(NovelAIError::Validation(
                                "vibe file path must not be empty".to_string(),
                            ));
                        }
                        validate_safe_path(&path.to_string_lossy())?;
                    }
                    VibeItem::Encoded(result) => {
                        result.validate()?;
                    }
                    VibeItem::RawEncoding(encoding) => {
                        if encoding.is_empty() {
                            return Err(NovelAIError::Validation(
                                "vibe raw encoding must not be empty".to_string(),
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

// =============================================================================
// EncodeVibeParams
// =============================================================================

impl EncodeVibeParams {
    pub fn validate(&self) -> Result<()> {
        validate_image_input_not_empty(&self.image)?;
        validate_unit_range(self.information_extracted, "information_extracted")?;
        validate_unit_range(self.strength, "strength")?;

        // Validate save target (path traversal checks)
        validate_save_target(&self.save)?;

        Ok(())
    }
}

// =============================================================================
// AugmentParams
// =============================================================================

impl AugmentParams {
    pub fn validate(&self) -> Result<()> {
        validate_image_input_not_empty(&self.image)?;

        let req_type_str = self.req_type.as_str();

        // Types that require defry
        let requires_defry = matches!(
            self.req_type,
            AugmentReqType::Colorize | AugmentReqType::Emotion
        );

        // Types that disallow prompt and defry
        let no_extra_params = matches!(
            self.req_type,
            AugmentReqType::Declutter
                | AugmentReqType::Sketch
                | AugmentReqType::Lineart
                | AugmentReqType::BgRemoval
        );

        // colorize / emotion: defry is required
        if requires_defry && self.defry.is_none() {
            return Err(NovelAIError::Validation(format!(
                "defry (0-5) is required for {}",
                req_type_str
            )));
        }

        // emotion: prompt is required
        if self.req_type == AugmentReqType::Emotion {
            match &self.prompt {
                None => {
                    return Err(NovelAIError::Validation(
                        "prompt is required for emotion (e.g., 'happy;;', 'sad;;')".to_string(),
                    ));
                }
                Some(p) => {
                    if !EMOTION_KEYWORDS.contains(&p.as_str()) {
                        return Err(NovelAIError::Validation(format!(
                            "Invalid emotion keyword '{}'. Valid: {}",
                            p,
                            EMOTION_KEYWORDS.join(", ")
                        )));
                    }
                }
            }
        }

        // declutter, sketch, lineart, bg-removal: prompt and defry must not be specified
        if no_extra_params {
            if let Some(ref prompt) = self.prompt {
                if !prompt.is_empty() {
                    return Err(NovelAIError::Validation(format!(
                        "prompt cannot be used with {}",
                        req_type_str
                    )));
                }
            }
            if self.defry.is_some() {
                return Err(NovelAIError::Validation(format!(
                    "defry cannot be used with {}",
                    req_type_str
                )));
            }
        }

        // defry range check
        if let Some(defry) = self.defry {
            if !(MIN_DEFRY..=MAX_DEFRY).contains(&defry) {
                return Err(NovelAIError::Validation(format!(
                    "defry must be between {} and {}",
                    MIN_DEFRY, MAX_DEFRY
                )));
            }
        }

        // Validate save target (path traversal checks)
        validate_save_target(&self.save)?;

        Ok(())
    }
}

// =============================================================================
// UpscaleParams
// =============================================================================

impl UpscaleParams {
    pub fn validate(&self) -> Result<()> {
        validate_image_input_not_empty(&self.image)?;

        if !VALID_UPSCALE_SCALES.contains(&self.scale) {
            return Err(NovelAIError::Validation(
                "scale must be one of: 2, 4".to_string(),
            ));
        }

        // Validate save target (path traversal checks)
        validate_save_target(&self.save)?;

        Ok(())
    }
}

// =============================================================================
// GenerateResult
// =============================================================================

impl GenerateResult {
    pub fn validate(&self) -> Result<()> {
        if self.image_data.is_empty() {
            return Err(NovelAIError::Validation(
                "image_data must not be empty".to_string(),
            ));
        }
        if self.seed > MAX_SEED as u64 {
            return Err(NovelAIError::Validation(format!(
                "seed must be between 0 and {}",
                MAX_SEED
            )));
        }
        Ok(())
    }
}

// =============================================================================
// UpscaleResult
// =============================================================================

impl UpscaleResult {
    pub fn validate(&self) -> Result<()> {
        if self.image_data.is_empty() {
            return Err(NovelAIError::Validation(
                "image_data must not be empty".to_string(),
            ));
        }
        if !VALID_UPSCALE_SCALES.contains(&self.scale) {
            return Err(NovelAIError::Validation(
                "scale must be one of: 2, 4".to_string(),
            ));
        }
        if self.output_width == 0 {
            return Err(NovelAIError::Validation(
                "output_width must be greater than 0".to_string(),
            ));
        }
        if self.output_height == 0 {
            return Err(NovelAIError::Validation(
                "output_height must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }
}
