use crate::constants::*;
use crate::error::{NovelAIError, Result};
use super::types::*;

// =============================================================================
// Utility Functions
// =============================================================================

/// Detect path traversal (`..`) after normalizing backslashes to forward slashes.
pub(crate) fn validate_safe_path(path: &str) -> Result<()> {
    let normalized = path.replace('\\', "/");
    if normalized.contains("..") {
        return Err(NovelAIError::Validation(
            "Path must not contain '..' (path traversal)".to_string(),
        ));
    }
    Ok(())
}

/// Validate that save_path and save_dir are not both specified.
pub(crate) fn validate_save_options_exclusive(
    save_path: &Option<String>,
    save_dir: &Option<String>,
) -> Result<()> {
    if save_path.is_some() && save_dir.is_some() {
        return Err(NovelAIError::Validation(
            "save_path and save_dir cannot be specified together. Use one or the other."
                .to_string(),
        ));
    }
    Ok(())
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
        if !(0.0..=1.0).contains(&self.center_x) {
            return Err(NovelAIError::Range(
                "center_x must be between 0.0 and 1.0".to_string(),
            ));
        }
        if !(0.0..=1.0).contains(&self.center_y) {
            return Err(NovelAIError::Range(
                "center_y must be between 0.0 and 1.0".to_string(),
            ));
        }
        Ok(())
    }
}

// =============================================================================
// CharacterReferenceConfig
// =============================================================================

impl CharacterReferenceConfig {
    pub fn validate(&self) -> Result<()> {
        validate_image_input_not_empty(&self.image)?;
        if !(0.0..=1.0).contains(&self.strength) {
            return Err(NovelAIError::Range(
                "strength must be between 0.0 and 1.0".to_string(),
            ));
        }
        if !(0.0..=1.0).contains(&self.fidelity) {
            return Err(NovelAIError::Range(
                "fidelity must be between 0.0 and 1.0".to_string(),
            ));
        }
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
        if !(0.0..=1.0).contains(&self.information_extracted) {
            return Err(NovelAIError::Range(
                "information_extracted must be between 0.0 and 1.0".to_string(),
            ));
        }
        if !(0.0..=1.0).contains(&self.strength) {
            return Err(NovelAIError::Range(
                "strength must be between 0.0 and 1.0".to_string(),
            ));
        }
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
        self.validate_action_dependencies()?;
        self.validate_vibe_params()?;
        self.validate_pixel_constraints()?;
        self.validate_save_options()?;
        self.validate_characters()?;
        self.validate_character_reference()?;
        self.validate_vibes()?;
        Ok(())
    }

    /// Stub for Session 4: async validation including token count checks.
    pub fn validate_async(&self) -> crate::error::Result<()> {
        self.validate()?;
        // TODO: Session 4 - Token count validation (positive/negative prompt token limits)
        Ok(())
    }

    fn validate_dimensions(&self) -> Result<()> {
        if !(MIN_DIMENSION..=MAX_GENERATION_DIMENSION).contains(&self.width) {
            return Err(NovelAIError::Range(format!(
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
            return Err(NovelAIError::Range(format!(
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
            return Err(NovelAIError::Range(format!(
                "steps must be between {} and {}",
                MIN_STEPS, MAX_STEPS
            )));
        }
        Ok(())
    }

    fn validate_scale(&self) -> Result<()> {
        if !(MIN_SCALE..=MAX_SCALE).contains(&self.scale) {
            return Err(NovelAIError::Range(format!(
                "scale must be between {} and {}",
                MIN_SCALE, MAX_SCALE
            )));
        }
        Ok(())
    }

    fn validate_cfg_rescale(&self) -> Result<()> {
        if !(0.0..=1.0).contains(&self.cfg_rescale) {
            return Err(NovelAIError::Range(
                "cfg_rescale must be between 0.0 and 1.0".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_seed(&self) -> Result<()> {
        if let Some(seed) = self.seed {
            if seed > MAX_SEED as u64 {
                return Err(NovelAIError::Range(format!(
                    "seed must be between 0 and {}",
                    MAX_SEED
                )));
            }
        }
        Ok(())
    }

    fn validate_img2img_params(&self) -> Result<()> {
        if !(0.0..=1.0).contains(&self.img2img_strength) {
            return Err(NovelAIError::Range(
                "img2img_strength must be between 0.0 and 1.0".to_string(),
            ));
        }
        if !(0.0..=1.0).contains(&self.img2img_noise) {
            return Err(NovelAIError::Range(
                "img2img_noise must be between 0.0 and 1.0".to_string(),
            ));
        }
        if let Some(mask_strength) = self.mask_strength {
            if !(0.01..=1.0).contains(&mask_strength) {
                return Err(NovelAIError::Range(
                    "mask_strength must be between 0.01 and 1.0".to_string(),
                ));
            }
        }
        if let Some(hybrid_strength) = self.hybrid_img2img_strength {
            if !(0.01..=0.99).contains(&hybrid_strength) {
                return Err(NovelAIError::Range(
                    "hybrid_img2img_strength must be between 0.01 and 0.99".to_string(),
                ));
            }
        }
        if let Some(hybrid_noise) = self.hybrid_img2img_noise {
            if !(0.0..=0.99).contains(&hybrid_noise) {
                return Err(NovelAIError::Range(
                    "hybrid_img2img_noise must be between 0.0 and 0.99".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn validate_action_dependencies(&self) -> Result<()> {
        // vibes and character_reference cannot be used together
        let has_vibes = self
            .vibes
            .as_ref()
            .is_some_and(|v| !v.is_empty());
        if has_vibes && self.character_reference.is_some() {
            return Err(NovelAIError::Validation(
                "vibes and character_reference cannot be used together.".to_string(),
            ));
        }

        // action=Img2Img requires source_image
        if self.action == GenerateAction::Img2Img && self.source_image.is_none() {
            return Err(NovelAIError::Validation(
                "source_image is required for img2img action".to_string(),
            ));
        }

        // action=Infill requires source_image, mask, and mask_strength
        if self.action == GenerateAction::Infill {
            if self.source_image.is_none() {
                return Err(NovelAIError::Validation(
                    "source_image is required for infill action".to_string(),
                ));
            }
            if self.mask.is_none() {
                return Err(NovelAIError::Validation(
                    "mask is required for infill action".to_string(),
                ));
            }
            if self.mask_strength.is_none() {
                return Err(NovelAIError::Validation(
                    "mask_strength is required for infill action".to_string(),
                ));
            }
        }

        // mask can only be used with action='infill'
        if self.mask.is_some() && self.action != GenerateAction::Infill {
            return Err(NovelAIError::Validation(
                "mask can only be used with action='infill'".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_vibe_params(&self) -> Result<()> {
        let has_vibes = self
            .vibes
            .as_ref()
            .is_some_and(|v| !v.is_empty());

        // vibe_strengths without vibes
        let has_vibe_strengths = self
            .vibe_strengths
            .as_ref()
            .is_some_and(|v| !v.is_empty());
        if has_vibe_strengths && !has_vibes {
            return Err(NovelAIError::Validation(
                "vibe_strengths cannot be specified without vibes".to_string(),
            ));
        }

        // vibe_info_extracted without vibes
        let has_vibe_info = self
            .vibe_info_extracted
            .as_ref()
            .is_some_and(|v| !v.is_empty());
        if has_vibe_info && !has_vibes {
            return Err(NovelAIError::Validation(
                "vibe_info_extracted cannot be specified without vibes".to_string(),
            ));
        }

        // Length mismatch checks
        if let Some(vibes) = &self.vibes {
            if let Some(strengths) = &self.vibe_strengths {
                if vibes.len() != strengths.len() {
                    return Err(NovelAIError::Validation(format!(
                        "Mismatch between vibes count ({}) and vibe_strengths count ({})",
                        vibes.len(),
                        strengths.len()
                    )));
                }
            }
            if let Some(info) = &self.vibe_info_extracted {
                if vibes.len() != info.len() {
                    return Err(NovelAIError::Validation(format!(
                        "Mismatch between vibes count ({}) and vibe_info_extracted count ({})",
                        vibes.len(),
                        info.len()
                    )));
                }
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
        validate_save_options_exclusive(&self.save_path, &self.save_dir)?;
        if let Some(ref path) = self.save_path {
            validate_safe_path(path)?;
        }
        if let Some(ref dir) = self.save_dir {
            validate_safe_path(dir)?;
        }
        Ok(())
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
            for vibe in vibes {
                match vibe {
                    VibeItem::FilePath(path) => {
                        if path.is_empty() {
                            return Err(NovelAIError::Validation(
                                "vibe file path must not be empty".to_string(),
                            ));
                        }
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

        if !(0.0..=1.0).contains(&self.information_extracted) {
            return Err(NovelAIError::Range(
                "information_extracted must be between 0.0 and 1.0".to_string(),
            ));
        }
        if !(0.0..=1.0).contains(&self.strength) {
            return Err(NovelAIError::Range(
                "strength must be between 0.0 and 1.0".to_string(),
            ));
        }

        // save_path and save_dir are mutually exclusive
        validate_save_options_exclusive(&self.save_path, &self.save_dir)?;

        // Path traversal checks
        if let Some(ref path) = self.save_path {
            validate_safe_path(path)?;
        }
        if let Some(ref dir) = self.save_dir {
            validate_safe_path(dir)?;
        }

        // save_filename and save_path cannot be used together
        if self.save_filename.is_some() && self.save_path.is_some() {
            return Err(NovelAIError::Validation(
                "save_filename and save_path cannot be specified together. Use save_dir with save_filename instead."
                    .to_string(),
            ));
        }

        // save_filename requires save_dir
        if self.save_filename.is_some() && self.save_dir.is_none() {
            return Err(NovelAIError::Validation(
                "save_filename requires save_dir to be specified.".to_string(),
            ));
        }

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
                return Err(NovelAIError::Range(format!(
                    "defry must be between {} and {}",
                    MIN_DEFRY, MAX_DEFRY
                )));
            }
        }

        // save_path and save_dir exclusivity + path traversal
        validate_save_options_exclusive(&self.save_path, &self.save_dir)?;
        if let Some(ref path) = self.save_path {
            validate_safe_path(path)?;
        }
        if let Some(ref dir) = self.save_dir {
            validate_safe_path(dir)?;
        }

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

        // save_path and save_dir exclusivity + path traversal
        validate_save_options_exclusive(&self.save_path, &self.save_dir)?;
        if let Some(ref path) = self.save_path {
            validate_safe_path(path)?;
        }
        if let Some(ref dir) = self.save_dir {
            validate_safe_path(dir)?;
        }

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
            return Err(NovelAIError::Range(format!(
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
