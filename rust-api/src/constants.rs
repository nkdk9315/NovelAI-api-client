use serde::{Deserialize, Serialize};

// =============================================================================
// API URLs
// =============================================================================

pub fn api_url() -> String {
    std::env::var("NOVELAI_API_URL")
        .unwrap_or_else(|_| "https://image.novelai.net/ai/generate-image".to_string())
}

pub fn stream_url() -> String {
    std::env::var("NOVELAI_STREAM_URL")
        .unwrap_or_else(|_| "https://image.novelai.net/ai/generate-image-stream".to_string())
}

pub fn encode_url() -> String {
    std::env::var("NOVELAI_ENCODE_URL")
        .unwrap_or_else(|_| "https://image.novelai.net/ai/encode-vibe".to_string())
}

pub fn subscription_url() -> String {
    std::env::var("NOVELAI_SUBSCRIPTION_URL")
        .unwrap_or_else(|_| "https://api.novelai.net/user/subscription".to_string())
}

pub fn augment_url() -> String {
    std::env::var("NOVELAI_AUGMENT_URL")
        .unwrap_or_else(|_| "https://image.novelai.net/ai/augment-image".to_string())
}

pub fn upscale_url() -> String {
    std::env::var("NOVELAI_UPSCALE_URL")
        .unwrap_or_else(|_| "https://api.novelai.net/ai/upscale".to_string())
}

// =============================================================================
// Default Values
// =============================================================================

pub const DEFAULT_NEGATIVE: &str = "nsfw, lowres, artistic error, film grain, scan artifacts, worst quality, bad quality, jpeg artifacts, very displeasing, chromatic aberration, dithering, halftone, screentone";

pub const DEFAULT_MODEL: &str = "nai-diffusion-4-5-full";
pub const DEFAULT_WIDTH: u32 = 832;
pub const DEFAULT_HEIGHT: u32 = 1216;
pub const DEFAULT_STEPS: u32 = 23;
pub const DEFAULT_SCALE: f64 = 5.0;
pub const DEFAULT_SAMPLER: &str = "k_euler_ancestral";
pub const DEFAULT_NOISE_SCHEDULE: &str = "karras";
pub const DEFAULT_VIBE_STRENGTH: f64 = 0.7;
pub const DEFAULT_VIBE_INFO_EXTRACTED: f64 = 0.7;
pub const DEFAULT_IMG2IMG_STRENGTH: f64 = 0.62;
pub const DEFAULT_CFG_RESCALE: f64 = 0.0;

// Inpaint defaults
pub const DEFAULT_INPAINT_STRENGTH: f64 = 0.7;
pub const DEFAULT_INPAINT_NOISE: f64 = 0.0;
pub const DEFAULT_INPAINT_COLOR_CORRECT: bool = true;

// =============================================================================
// Validation Constants
// =============================================================================

/// Valid samplers
pub const VALID_SAMPLERS: &[&str] = &[
    "k_euler",
    "k_euler_ancestral",
    "k_dpmpp_2s_ancestral",
    "k_dpmpp_2m_sde",
    "k_dpmpp_2m",
    "k_dpmpp_sde",
];

/// Valid models
pub const VALID_MODELS: &[&str] = &[
    "nai-diffusion-4-curated-preview",
    "nai-diffusion-4-full",
    "nai-diffusion-4-5-curated",
    "nai-diffusion-4-5-full",
];

/// Valid noise schedules
pub const VALID_NOISE_SCHEDULES: &[&str] = &[
    "karras",
    "exponential",
    "polyexponential",
];

/// Sampler enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sampler {
    #[serde(rename = "k_euler")]
    KEuler,
    #[serde(rename = "k_euler_ancestral")]
    KEulerAncestral,
    #[serde(rename = "k_dpmpp_2s_ancestral")]
    KDpmpp2sAncestral,
    #[serde(rename = "k_dpmpp_2m_sde")]
    KDpmpp2mSde,
    #[serde(rename = "k_dpmpp_2m")]
    KDpmpp2m,
    #[serde(rename = "k_dpmpp_sde")]
    KDpmppSde,
}

impl Sampler {
    pub fn as_str(&self) -> &'static str {
        match self {
            Sampler::KEuler => "k_euler",
            Sampler::KEulerAncestral => "k_euler_ancestral",
            Sampler::KDpmpp2sAncestral => "k_dpmpp_2s_ancestral",
            Sampler::KDpmpp2mSde => "k_dpmpp_2m_sde",
            Sampler::KDpmpp2m => "k_dpmpp_2m",
            Sampler::KDpmppSde => "k_dpmpp_sde",
        }
    }
}

impl Default for Sampler {
    fn default() -> Self {
        Sampler::KEulerAncestral
    }
}

/// Model enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Model {
    #[serde(rename = "nai-diffusion-4-curated-preview")]
    NaiDiffusion4CuratedPreview,
    #[serde(rename = "nai-diffusion-4-full")]
    NaiDiffusion4Full,
    #[serde(rename = "nai-diffusion-4-5-curated")]
    NaiDiffusion45Curated,
    #[serde(rename = "nai-diffusion-4-5-full")]
    NaiDiffusion45Full,
}

impl Model {
    pub fn as_str(&self) -> &'static str {
        match self {
            Model::NaiDiffusion4CuratedPreview => "nai-diffusion-4-curated-preview",
            Model::NaiDiffusion4Full => "nai-diffusion-4-full",
            Model::NaiDiffusion45Curated => "nai-diffusion-4-5-curated",
            Model::NaiDiffusion45Full => "nai-diffusion-4-5-full",
        }
    }

    /// Get the model key used in Vibe files
    pub fn model_key(&self) -> &'static str {
        match self {
            Model::NaiDiffusion4CuratedPreview => "v4curated",
            Model::NaiDiffusion4Full => "v4full",
            Model::NaiDiffusion45Curated => "v4-5curated",
            Model::NaiDiffusion45Full => "v4-5full",
        }
    }
}

impl Default for Model {
    fn default() -> Self {
        Model::NaiDiffusion45Full
    }
}

/// Get model key from model name string
pub fn model_key_from_str(model: &str) -> Option<&'static str> {
    match model {
        "nai-diffusion-4-curated-preview" => Some("v4curated"),
        "nai-diffusion-4-full" => Some("v4full"),
        "nai-diffusion-4-5-curated" => Some("v4-5curated"),
        "nai-diffusion-4-5-full" => Some("v4-5full"),
        _ => None,
    }
}

/// Noise schedule enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoiseSchedule {
    #[serde(rename = "karras")]
    Karras,
    #[serde(rename = "exponential")]
    Exponential,
    #[serde(rename = "polyexponential")]
    Polyexponential,
}

impl NoiseSchedule {
    pub fn as_str(&self) -> &'static str {
        match self {
            NoiseSchedule::Karras => "karras",
            NoiseSchedule::Exponential => "exponential",
            NoiseSchedule::Polyexponential => "polyexponential",
        }
    }
}

impl Default for NoiseSchedule {
    fn default() -> Self {
        NoiseSchedule::Karras
    }
}

/// Augment request type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AugmentReqType {
    #[serde(rename = "colorize")]
    Colorize,
    #[serde(rename = "declutter")]
    Declutter,
    #[serde(rename = "emotion")]
    Emotion,
    #[serde(rename = "sketch")]
    Sketch,
    #[serde(rename = "lineart")]
    Lineart,
    #[serde(rename = "bg-removal")]
    BgRemoval,
}

impl AugmentReqType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AugmentReqType::Colorize => "colorize",
            AugmentReqType::Declutter => "declutter",
            AugmentReqType::Emotion => "emotion",
            AugmentReqType::Sketch => "sketch",
            AugmentReqType::Lineart => "lineart",
            AugmentReqType::BgRemoval => "bg-removal",
        }
    }
}

/// Augment request types list
pub const AUGMENT_REQ_TYPES: &[&str] = &[
    "colorize",
    "declutter",
    "emotion",
    "sketch",
    "lineart",
    "bg-removal",
];

/// Emotion keywords for emotion augment tool
pub const EMOTION_KEYWORDS: &[&str] = &[
    "neutral", "happy", "sad", "angry", "scared", "surprised",
    "tired", "excited", "nervous", "thinking", "confused", "shy",
    "disgusted", "smug", "bored", "laughing", "irritated", "aroused",
    "embarrassed", "love", "worried", "determined", "hurt", "playful",
];

// =============================================================================
// Limits
// =============================================================================

// Prompt
pub const MAX_TOKENS: usize = 512;

// Pixels
pub const MAX_PIXELS: u64 = 3_145_728; // 2048 * 1536
pub const MIN_DIMENSION: u32 = 64;
pub const MAX_GENERATION_DIMENSION: u32 = 2048;

// Characters
pub const MAX_CHARACTERS: usize = 6;

// Vibe
pub const MAX_VIBES: usize = 10;

// Generation parameters
pub const MIN_STEPS: u32 = 1;
pub const MAX_STEPS: u32 = 50;
pub const MIN_SCALE: f64 = 0.0;
pub const MAX_SCALE: f64 = 10.0;
pub const MAX_SEED: u32 = 4_294_967_295; // 2^32 - 1

// Reference image
pub const MAX_REF_IMAGE_SIZE_MB: u32 = 10;
pub const MAX_REF_IMAGE_DIMENSION: u32 = 4096;

// Character reference image sizes
pub const CHARREF_PORTRAIT_SIZE: (u32, u32) = (1024, 1536);
pub const CHARREF_LANDSCAPE_SIZE: (u32, u32) = (1536, 1024);
pub const CHARREF_SQUARE_SIZE: (u32, u32) = (1472, 1472);
pub const CHARREF_PORTRAIT_THRESHOLD: f64 = 0.8;
pub const CHARREF_LANDSCAPE_THRESHOLD: f64 = 1.25;

// Defry range
pub const MIN_DEFRY: u32 = 0;
pub const MAX_DEFRY: u32 = 5;
pub const DEFAULT_DEFRY: u32 = 3;

// Upscale
pub const VALID_UPSCALE_SCALES: &[u32] = &[2, 4];
pub const DEFAULT_UPSCALE_SCALE: u32 = 4;

// =============================================================================
// Enhance Level Presets
// =============================================================================

pub struct EnhanceLevelPreset {
    pub strength: f64,
    pub noise: f64,
}

pub const ENHANCE_LEVEL_PRESETS: &[(u32, EnhanceLevelPreset)] = &[
    (1, EnhanceLevelPreset { strength: 0.2, noise: 0.0 }),
    (2, EnhanceLevelPreset { strength: 0.4, noise: 0.0 }),
    (3, EnhanceLevelPreset { strength: 0.5, noise: 0.0 }),
    (4, EnhanceLevelPreset { strength: 0.6, noise: 0.0 }),
    (5, EnhanceLevelPreset { strength: 0.7, noise: 0.1 }),
];

pub fn get_enhance_preset(level: u32) -> Option<&'static EnhanceLevelPreset> {
    ENHANCE_LEVEL_PRESETS
        .iter()
        .find(|(l, _)| *l == level)
        .map(|(_, preset)| preset)
}

// =============================================================================
// Anlas Cost Calculation Constants
// =============================================================================

// Opus free conditions
pub const OPUS_FREE_PIXELS: u64 = 1_048_576; // 1024*1024
pub const OPUS_FREE_MAX_STEPS: u32 = 28;
pub const OPUS_MIN_TIER: u32 = 3;

// Per-image cost limits
pub const MAX_COST_PER_IMAGE: u64 = 140;
pub const MIN_COST_PER_IMAGE: u64 = 2;

// Grid size (for inpaint)
pub const GRID_SIZE: u64 = 64;

// Vibe cost
pub const VIBE_BATCH_PRICE: u64 = 2;
pub const VIBE_FREE_THRESHOLD: u64 = 4;
pub const VIBE_ENCODE_PRICE: u64 = 2;

// Character reference cost
pub const CHAR_REF_PRICE: u64 = 5;

// Inpaint threshold
pub const INPAINT_THRESHOLD_RATIO: f64 = 0.8;

// V4 cost calculation coefficients
pub const V4_COST_COEFF_LINEAR: f64 = 2.951823174884865e-6;
pub const V4_COST_COEFF_STEP: f64 = 5.753298233447344e-7;

// Augment fixed parameters
pub const AUGMENT_FIXED_STEPS: u32 = 28;
pub const AUGMENT_MIN_PIXELS: u64 = 1_048_576;

// Background removal special calculation
pub const BG_REMOVAL_MULTIPLIER: u64 = 3;
pub const BG_REMOVAL_ADDEND: u64 = 5;

// Upscale cost table [max_pixels, cost] (ascending)
pub const UPSCALE_COST_TABLE: &[(u64, u64)] = &[
    (262_144, 1),
    (409_600, 2),
    (524_288, 3),
    (786_432, 5),
    (1_048_576, 7),
];

// Upscale Opus free pixel limit
pub const UPSCALE_OPUS_FREE_PIXELS: u64 = 409_600;

// =============================================================================
// Network & Security Constants
// =============================================================================

pub const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 60_000; // 60 seconds
pub const MAX_DECOMPRESSED_IMAGE_SIZE: usize = 50 * 1024 * 1024; // 50MB
pub const MAX_RESPONSE_SIZE: usize = 200 * 1024 * 1024; // 200MB
pub const MAX_ZIP_ENTRIES: usize = 10;
pub const MAX_COMPRESSION_RATIO: u64 = 100;
pub const MAX_VIBE_ENCODING_LENGTH: usize = 5_000_000; // ~3.5MB base64
