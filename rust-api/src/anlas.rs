use crate::constants::*;
use crate::error::NovelAIError;

// =============================================================================
// Type Definitions
// =============================================================================

/// Subscription tier (0=Free, 1=Tablet, 2=Scroll, 3=Opus)
pub type SubscriptionTier = u32;

/// SMEA mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmeaMode {
    Off,
    Smea,
    SmeaDyn,
}

/// Generation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationMode {
    Txt2Img,
    Img2Img,
    Inpaint,
}

/// Augment tool type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AugmentToolType {
    Colorize,
    Declutter,
    Emotion,
    Sketch,
    Lineart,
    BgRemoval,
}

/// Generation cost calculation parameters
#[derive(Debug, Clone)]
pub struct GenerationCostParams {
    pub width: u32,
    pub height: u32,
    pub steps: u32,
    pub smea: SmeaMode,
    pub mode: GenerationMode,
    pub strength: f64,
    pub n_samples: u32,
    pub tier: SubscriptionTier,
    pub char_ref_count: u32,
    pub vibe_count: u64,
    pub vibe_unencoded_count: u64,
    pub mask_width: Option<u32>,
    pub mask_height: Option<u32>,
}

impl Default for GenerationCostParams {
    fn default() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            steps: DEFAULT_STEPS,
            smea: SmeaMode::Off,
            mode: GenerationMode::Txt2Img,
            strength: 1.0,
            n_samples: 1,
            tier: 0,
            char_ref_count: 0,
            vibe_count: 0,
            vibe_unencoded_count: 0,
            mask_width: None,
            mask_height: None,
        }
    }
}

/// Generation cost calculation result with breakdown
#[derive(Debug, Clone)]
pub struct GenerationCostResult {
    pub base_cost: u64,
    pub smea_multiplier: f64,
    pub per_image_cost: f64,
    pub strength_multiplier: f64,
    pub adjusted_cost: u64,
    pub is_opus_free: bool,
    pub billable_images: u32,
    pub generation_cost: u64,
    pub char_ref_cost: u64,
    pub vibe_encode_cost: u64,
    pub vibe_batch_cost: u64,
    pub total_cost: u64,
    pub error: bool,
    pub error_code: Option<i32>,
}

/// Augment cost calculation parameters
#[derive(Debug, Clone)]
pub struct AugmentCostParams {
    pub tool: AugmentToolType,
    pub width: u32,
    pub height: u32,
    pub tier: SubscriptionTier,
}

/// Augment cost calculation result
#[derive(Debug, Clone)]
pub struct AugmentCostResult {
    pub original_pixels: u64,
    pub adjusted_width: u64,
    pub adjusted_height: u64,
    pub adjusted_pixels: u64,
    pub base_cost: u64,
    pub final_cost: u64,
    pub is_opus_free: bool,
    pub effective_cost: u64,
}

/// Upscale cost calculation parameters
#[derive(Debug, Clone)]
pub struct UpscaleCostParams {
    pub width: u32,
    pub height: u32,
    pub tier: SubscriptionTier,
}

/// Upscale cost calculation result
#[derive(Debug, Clone)]
pub struct UpscaleCostResult {
    pub pixels: u64,
    pub cost: Option<u64>,
    pub is_opus_free: bool,
    pub error: bool,
    pub error_code: Option<i32>,
}

/// Inpaint size correction result
#[derive(Debug, Clone)]
pub struct InpaintCorrectionResult {
    pub corrected: bool,
    pub width: u64,
    pub height: u64,
}

/// Size adjustment result
#[derive(Debug, Clone)]
pub struct SizeResult {
    pub width: u64,
    pub height: u64,
    pub pixels: u64,
}

// =============================================================================
// Validation
// =============================================================================

fn assert_positive_finite_int(value: u32, name: &str) -> Result<(), NovelAIError> {
    if value == 0 {
        return Err(NovelAIError::Validation(format!(
            "{} must be a positive finite integer, got {}",
            name, value
        )));
    }
    Ok(())
}

fn assert_finite_range(value: f64, min: f64, max: f64, name: &str) -> Result<(), NovelAIError> {
    if !value.is_finite() || value < min || value > max {
        return Err(NovelAIError::Validation(format!(
            "{} must be a finite number between {} and {}, got {}",
            name, min, max, value
        )));
    }
    Ok(())
}

// =============================================================================
// Basic Calculation Functions
// =============================================================================

/// Calculate V4 model base cost.
/// Linear cost calculation based on pixel count and step count.
pub fn calc_v4_base_cost(width: u64, height: u64, steps: u64) -> u64 {
    let pixels = width * height;
    let cost = V4_COST_COEFF_LINEAR * pixels as f64 + V4_COST_COEFF_STEP * pixels as f64 * steps as f64;
    cost.ceil() as u64
}

/// Return the cost multiplier for the given SMEA mode.
pub fn get_smea_multiplier(mode: SmeaMode) -> f64 {
    match mode {
        SmeaMode::SmeaDyn => 1.4,
        SmeaMode::Smea => 1.2,
        SmeaMode::Off => 1.0,
    }
}

/// Check if the generation qualifies for Opus free generation.
pub fn is_opus_free_generation(
    width: u32,
    height: u32,
    steps: u32,
    char_ref_count: u32,
    tier: SubscriptionTier,
    vibe_count: u64,
) -> bool {
    char_ref_count == 0
        && vibe_count == 0
        && (width as u64) * (height as u64) <= OPUS_FREE_PIXELS
        && steps <= OPUS_FREE_MAX_STEPS
        && tier >= OPUS_MIN_TIER
}

/// Calculate Vibe batch cost.
/// Only vibes beyond the free threshold are charged.
pub fn calc_vibe_batch_cost(enabled_vibe_count: u64) -> u64 {
    if enabled_vibe_count > VIBE_FREE_THRESHOLD {
        (enabled_vibe_count - VIBE_FREE_THRESHOLD) * VIBE_BATCH_PRICE
    } else {
        0
    }
}

/// Calculate character reference cost.
pub fn calc_char_ref_cost(char_ref_count: u32, n_samples: u32) -> u64 {
    CHAR_REF_PRICE * char_ref_count as u64 * n_samples as u64
}

// =============================================================================
// Pixel Adjustment Functions
// =============================================================================

/// Expand to minimum pixels (maintaining aspect ratio).
/// No grid snap, Math.floor only.
pub fn expand_to_min_pixels(width: u64, height: u64, min_pixels: u64) -> SizeResult {
    let pixels = width * height;
    if pixels >= min_pixels {
        return SizeResult { width, height, pixels };
    }
    let scale = (min_pixels as f64 / pixels as f64).sqrt();
    let new_w = (width as f64 * scale).ceil() as u64;
    let mut new_h = (height as f64 * scale).floor() as u64;
    // Ensure we actually meet the minPixels requirement
    if new_w * new_h < min_pixels {
        new_h = (height as f64 * scale).ceil() as u64;
    }
    SizeResult {
        width: new_w,
        height: new_h,
        pixels: new_w * new_h,
    }
}

/// Clamp to maximum pixels (maintaining aspect ratio).
/// No grid snap, Math.floor only.
pub fn clamp_to_max_pixels(width: u64, height: u64, max_pixels: u64) -> SizeResult {
    let pixels = width * height;
    if pixels <= max_pixels {
        return SizeResult { width, height, pixels };
    }
    let scale = (max_pixels as f64 / pixels as f64).sqrt();
    let new_w = (width as f64 * scale).floor() as u64;
    let new_h = (height as f64 * scale).floor() as u64;
    SizeResult {
        width: new_w,
        height: new_h,
        pixels: new_w * new_h,
    }
}

// =============================================================================
// Inpaint Correction
// =============================================================================

/// Calculate inpaint mask size correction.
/// If the mask is smaller than the threshold, expand to OPUS_FREE_PIXELS and grid snap.
pub fn calc_inpaint_size_correction(mask_width: u64, mask_height: u64) -> InpaintCorrectionResult {
    if mask_width == 0 || mask_height == 0 {
        return InpaintCorrectionResult {
            corrected: false,
            width: mask_width,
            height: mask_height,
        };
    }

    let pixels = mask_width as f64 * mask_height as f64;
    let threshold = OPUS_FREE_PIXELS as f64 * INPAINT_THRESHOLD_RATIO;

    if pixels >= threshold {
        return InpaintCorrectionResult {
            corrected: false,
            width: mask_width,
            height: mask_height,
        };
    }

    let scale = (OPUS_FREE_PIXELS as f64 / pixels).sqrt();
    let grid = GRID_SIZE as f64;
    let new_w = (((mask_width as f64 * scale).floor() / grid).floor() * grid).max(grid);
    let new_h = (((mask_height as f64 * scale).floor() / grid).floor() * grid).max(grid);

    InpaintCorrectionResult {
        corrected: true,
        width: new_w as u64,
        height: new_h as u64,
    }
}

// =============================================================================
// Main: Generation Cost Calculation
// =============================================================================

/// Calculate generation cost (main orchestrator).
/// Integrates all cost elements and returns the final cost with breakdown.
pub fn calculate_generation_cost(params: &GenerationCostParams) -> Result<GenerationCostResult, NovelAIError> {
    // Input validation
    assert_positive_finite_int(params.width, "width")?;
    assert_positive_finite_int(params.height, "height")?;
    assert_positive_finite_int(params.steps, "steps")?;

    match params.mode {
        GenerationMode::Img2Img | GenerationMode::Inpaint => {
            assert_finite_range(params.strength, 0.0, 1.0, "strength")?;
        }
        _ => {}
    }

    // Validate maskWidth/maskHeight pair
    if params.mode == GenerationMode::Inpaint {
        let has_mask_w = params.mask_width.is_some();
        let has_mask_h = params.mask_height.is_some();
        if has_mask_w != has_mask_h {
            return Err(NovelAIError::Validation(
                "maskWidth and maskHeight must both be specified or both omitted for inpaint mode".to_string(),
            ));
        }
    }

    // Determine effective width/height (inpaint mask correction)
    let mut effective_width = params.width as u64;
    let mut effective_height = params.height as u64;

    if params.mode == GenerationMode::Inpaint {
        if let (Some(mw), Some(mh)) = (params.mask_width, params.mask_height) {
            let correction = calc_inpaint_size_correction(mw as u64, mh as u64);
            if correction.corrected {
                effective_width = correction.width;
                effective_height = correction.height;
            }
        }
    }

    // Base cost calculation
    let base_cost = calc_v4_base_cost(effective_width, effective_height, params.steps as u64);

    // SMEA multiplier
    let smea_multiplier = get_smea_multiplier(params.smea);
    let per_image_cost = base_cost as f64 * smea_multiplier;

    // Strength multiplier (txt2img is always 1.0)
    let strength_multiplier = match params.mode {
        GenerationMode::Txt2Img => 1.0,
        GenerationMode::Img2Img | GenerationMode::Inpaint => params.strength,
    };

    // Adjusted cost (minimum MIN_COST_PER_IMAGE guaranteed)
    let adjusted_cost = std::cmp::max(
        (per_image_cost * strength_multiplier).ceil() as u64,
        MIN_COST_PER_IMAGE,
    );

    // Error check (exceeds max cost)
    if adjusted_cost > MAX_COST_PER_IMAGE {
        return Err(NovelAIError::Validation(format!(
            "Adjusted cost per image ({}) exceeds maximum allowed ({})",
            adjusted_cost, MAX_COST_PER_IMAGE
        )));
    }

    // Opus free check (using corrected dimensions for consistency with cost calculation)
    let is_opus_free = is_opus_free_generation(
        effective_width as u32,
        effective_height as u32,
        params.steps,
        params.char_ref_count,
        params.tier,
        params.vibe_count,
    );

    // Billable images
    let billable_images = if is_opus_free && params.n_samples > 0 {
        params.n_samples - 1
    } else {
        params.n_samples
    };
    let generation_cost = adjusted_cost * billable_images as u64;

    // Vibe cost (disabled when using character reference or inpaint)
    let (vibe_encode_cost, vibe_batch_cost) = if params.char_ref_count == 0 && params.mode != GenerationMode::Inpaint {
        (
            params.vibe_unencoded_count * VIBE_ENCODE_PRICE,
            calc_vibe_batch_cost(params.vibe_count),
        )
    } else {
        (0, 0)
    };

    // Character reference cost
    let char_ref_cost = if params.char_ref_count > 0 {
        calc_char_ref_cost(params.char_ref_count, params.n_samples)
    } else {
        0
    };

    let total_cost = generation_cost + char_ref_cost + vibe_encode_cost + vibe_batch_cost;

    Ok(GenerationCostResult {
        base_cost,
        smea_multiplier,
        per_image_cost,
        strength_multiplier,
        adjusted_cost,
        is_opus_free,
        billable_images,
        generation_cost,
        char_ref_cost,
        vibe_encode_cost,
        vibe_batch_cost,
        total_cost,
        error: false,
        error_code: None,
    })
}

// =============================================================================
// Augment Cost Calculation
// =============================================================================

/// Calculate augment tool cost.
/// Clamp to MAX_PIXELS -> expand to AUGMENT_MIN_PIXELS -> V4 base cost.
/// bg-removal has additional multiplier and addend.
pub fn calculate_augment_cost(params: &AugmentCostParams) -> Result<AugmentCostResult, NovelAIError> {
    assert_positive_finite_int(params.width, "width")?;
    assert_positive_finite_int(params.height, "height")?;

    let tier = params.tier;
    let original_pixels = params.width as u64 * params.height as u64;

    // Clamp to MAX_PIXELS
    let clamped = clamp_to_max_pixels(params.width as u64, params.height as u64, MAX_PIXELS);

    // Expand to AUGMENT_MIN_PIXELS
    let expanded = expand_to_min_pixels(clamped.width, clamped.height, AUGMENT_MIN_PIXELS);

    // Base cost (fixed steps)
    let base_cost = calc_v4_base_cost(expanded.width, expanded.height, AUGMENT_FIXED_STEPS as u64);

    // bg-removal has special calculation
    let final_cost = if params.tool == AugmentToolType::BgRemoval {
        let cost = BG_REMOVAL_MULTIPLIER as f64 * base_cost as f64 + BG_REMOVAL_ADDEND as f64;
        cost.ceil() as u64
    } else {
        base_cost
    };

    // Opus free check (bg-removal is always charged)
    let is_opus_free = params.tool != AugmentToolType::BgRemoval
        && expanded.width * expanded.height <= OPUS_FREE_PIXELS
        && tier >= OPUS_MIN_TIER;

    let effective_cost = if is_opus_free { 0 } else { final_cost };

    Ok(AugmentCostResult {
        original_pixels,
        adjusted_width: expanded.width,
        adjusted_height: expanded.height,
        adjusted_pixels: expanded.width * expanded.height,
        base_cost,
        final_cost,
        is_opus_free,
        effective_cost,
    })
}

// =============================================================================
// Upscale Cost Calculation
// =============================================================================

/// Calculate upscale cost.
/// Table lookup based on pixel count.
pub fn calculate_upscale_cost(params: &UpscaleCostParams) -> Result<UpscaleCostResult, NovelAIError> {
    assert_positive_finite_int(params.width, "width")?;
    assert_positive_finite_int(params.height, "height")?;

    let tier = params.tier;
    let pixels = params.width as u64 * params.height as u64;

    // Opus free check
    if tier >= OPUS_MIN_TIER && pixels <= UPSCALE_OPUS_FREE_PIXELS {
        return Ok(UpscaleCostResult {
            pixels,
            cost: Some(0),
            is_opus_free: true,
            error: false,
            error_code: None,
        });
    }

    // Look up cost in table (ascending, return first match)
    for &(threshold, price) in UPSCALE_COST_TABLE {
        if pixels <= threshold {
            return Ok(UpscaleCostResult {
                pixels,
                cost: Some(price),
                is_opus_free: false,
                error: false,
                error_code: None,
            });
        }
    }

    // No match in table -> error
    Ok(UpscaleCostResult {
        pixels,
        cost: None,
        is_opus_free: false,
        error: true,
        error_code: Some(-3),
    })
}
