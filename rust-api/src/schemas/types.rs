use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::constants::*;

// =============================================================================
// Enums
// =============================================================================

/// Image generation action type.
///
/// Uses data-carrying variants so that action-specific fields are always
/// present exactly when they are relevant, making illegal states
/// unrepresentable at compile time.
#[derive(Debug, Clone, Default)]
pub enum GenerateAction {
    #[default]
    Generate,
    Img2Img {
        source_image: ImageInput,
        strength: f64,
        noise: f64,
    },
    Infill {
        source_image: ImageInput,
        mask: ImageInput,
        mask_strength: f64,
        color_correct: bool,
        hybrid_strength: Option<f64>,
        hybrid_noise: Option<f64>,
    },
}

impl GenerateAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            GenerateAction::Generate => "generate",
            GenerateAction::Img2Img { .. } => "img2img",
            GenerateAction::Infill { .. } => "infill",
        }
    }

    pub fn is_generate(&self) -> bool {
        matches!(self, GenerateAction::Generate)
    }

    pub fn is_img2img(&self) -> bool {
        matches!(self, GenerateAction::Img2Img { .. })
    }

    pub fn is_infill(&self) -> bool {
        matches!(self, GenerateAction::Infill { .. })
    }
}

/// Character reference mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CharRefMode {
    #[serde(rename = "character")]
    Character,
    #[serde(rename = "character&style")]
    #[default]
    CharacterAndStyle,
    #[serde(rename = "style")]
    Style,
}

impl CharRefMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            CharRefMode::Character => "character",
            CharRefMode::CharacterAndStyle => "character&style",
            CharRefMode::Style => "style",
        }
    }
}

/// Image input type (file path, base64, data URL, or raw bytes).
/// Does not implement Serialize/Deserialize as it is converted to base64 at payload construction time.
#[derive(Debug, Clone)]
pub enum ImageInput {
    FilePath(PathBuf),
    Base64(String),
    DataUrl(String),
    Bytes(Vec<u8>),
}

impl ImageInput {
    pub fn is_empty(&self) -> bool {
        match self {
            ImageInput::FilePath(p) => p.as_os_str().is_empty(),
            ImageInput::Base64(s) | ImageInput::DataUrl(s) => s.is_empty(),
            ImageInput::Bytes(b) => b.is_empty(),
        }
    }
}

/// Vibe item: either a pre-encoded VibeEncodeResult, a file path, or a raw encoding string
#[derive(Debug, Clone)]
pub enum VibeItem {
    Encoded(VibeEncodeResult),
    FilePath(PathBuf),
    RawEncoding(String),
}

/// Save target for generated/encoded output files.
///
/// Replaces the `save_path: Option<String>` / `save_dir: Option<String>`
/// pair, eliminating the mutual-exclusion runtime check.
#[derive(Debug, Clone, Default)]
pub enum SaveTarget {
    /// Do not save (default).
    #[default]
    None,
    /// Save to this exact file path.
    ExactPath(String),
    /// Save into a directory, optionally with a custom filename.
    Directory {
        dir: String,
        filename: Option<String>,
    },
}

// =============================================================================
// VibeConfig
// =============================================================================

/// Consolidated vibe configuration that bundles a vibe item with its
/// associated strength and information_extracted values.
///
/// Replaces the old parallel-vector design (`vibes`, `vibe_strengths`,
/// `vibe_info_extracted`) which could represent illegal states such as
/// length mismatches or one vector being `Some` while another is `None`.
#[derive(Debug, Clone)]
pub struct VibeConfig {
    pub item: VibeItem,
    pub strength: f64,
    pub info_extracted: f64,
}

// =============================================================================
// CharacterConfig
// =============================================================================

#[derive(Debug, Clone)]
pub struct CharacterConfig {
    pub prompt: String,
    pub center_x: f64,
    pub center_y: f64,
    pub negative_prompt: String,
}

impl Default for CharacterConfig {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            center_x: 0.5,
            center_y: 0.5,
            negative_prompt: String::new(),
        }
    }
}

// =============================================================================
// CharacterReferenceConfig
// =============================================================================

#[derive(Debug, Clone)]
pub struct CharacterReferenceConfig {
    pub image: ImageInput,
    pub strength: f64,
    pub fidelity: f64,
    pub mode: CharRefMode,
}

// =============================================================================
// VibeEncodeResult
// =============================================================================

#[derive(Debug, Clone)]
pub struct VibeEncodeResult {
    pub encoding: String,
    pub model: Model,
    pub information_extracted: f64,
    pub strength: f64,
    pub source_image_hash: String,
    /// ISO 8601 date string (chrono dependency avoided)
    pub created_at: String,
    pub saved_path: Option<String>,
    pub anlas_remaining: Option<u64>,
    pub anlas_consumed: Option<u64>,
}

// =============================================================================
// GenerateParams
// =============================================================================

#[derive(Debug, Clone)]
pub struct GenerateParams {
    pub prompt: String,
    pub action: GenerateAction,
    pub characters: Option<Vec<CharacterConfig>>,
    pub vibes: Option<Vec<VibeConfig>>,
    pub character_reference: Option<CharacterReferenceConfig>,
    pub negative_prompt: Option<String>,
    pub save: SaveTarget,
    pub model: Model,
    pub width: u32,
    pub height: u32,
    pub steps: u32,
    pub scale: f64,
    pub cfg_rescale: f64,
    /// Seed value (0..=MAX_SEED). Uses u64 to avoid overflow during comparison.
    pub seed: Option<u64>,
    pub sampler: Sampler,
    pub noise_schedule: NoiseSchedule,
}

impl Default for GenerateParams {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            action: GenerateAction::default(),
            characters: None,
            vibes: None,
            character_reference: None,
            negative_prompt: None,
            save: SaveTarget::default(),
            model: Model::default(),
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            steps: DEFAULT_STEPS,
            scale: DEFAULT_SCALE,
            cfg_rescale: DEFAULT_CFG_RESCALE,
            seed: None,
            sampler: Sampler::default(),
            noise_schedule: NoiseSchedule::default(),
        }
    }
}

// =============================================================================
// GenerateResult
// =============================================================================

#[derive(Debug, Clone)]
pub struct GenerateResult {
    pub image_data: Vec<u8>,
    pub seed: u64,
    pub anlas_remaining: Option<u64>,
    pub anlas_consumed: Option<u64>,
    pub saved_path: Option<String>,
}

// =============================================================================
// EncodeVibeParams
// =============================================================================

#[derive(Debug, Clone)]
pub struct EncodeVibeParams {
    pub image: ImageInput,
    pub model: Model,
    pub information_extracted: f64,
    pub strength: f64,
    pub save: SaveTarget,
}

impl Default for EncodeVibeParams {
    fn default() -> Self {
        Self {
            image: ImageInput::Bytes(Vec::new()),
            model: Model::default(),
            information_extracted: DEFAULT_VIBE_INFO_EXTRACTED,
            strength: DEFAULT_VIBE_STRENGTH,
            save: SaveTarget::default(),
        }
    }
}

// =============================================================================
// AugmentParams / AugmentResult
// =============================================================================

#[derive(Debug, Clone)]
pub struct AugmentParams {
    pub req_type: AugmentReqType,
    pub image: ImageInput,
    pub prompt: Option<String>,
    pub defry: Option<u32>,
    pub save: SaveTarget,
}

#[derive(Debug, Clone)]
pub struct AugmentResult {
    pub image_data: Vec<u8>,
    pub req_type: AugmentReqType,
    pub anlas_remaining: Option<u64>,
    pub anlas_consumed: Option<u64>,
    pub saved_path: Option<String>,
}

// =============================================================================
// UpscaleParams / UpscaleResult
// =============================================================================

#[derive(Debug, Clone)]
pub struct UpscaleParams {
    pub image: ImageInput,
    pub scale: u32,
    pub save: SaveTarget,
}

impl Default for UpscaleParams {
    fn default() -> Self {
        Self {
            image: ImageInput::Bytes(Vec::new()),
            scale: DEFAULT_UPSCALE_SCALE,
            save: SaveTarget::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpscaleResult {
    pub image_data: Vec<u8>,
    pub scale: u32,
    pub output_width: u32,
    pub output_height: u32,
    pub anlas_remaining: Option<u64>,
    pub anlas_consumed: Option<u64>,
    pub saved_path: Option<String>,
}

// =============================================================================
// AnlasBalanceResponse
// =============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnlasBalanceResponse {
    #[serde(rename = "trainingStepsLeft", default)]
    pub training_steps_left: TrainingStepsLeft,
    #[serde(default)]
    pub tier: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrainingStepsLeft {
    #[serde(rename = "fixedTrainingStepsLeft", default)]
    pub fixed: u64,
    #[serde(rename = "purchasedTrainingSteps", default)]
    pub purchased: u64,
}

// =============================================================================
// Caption Dict (for v4_prompt payload construction)
// =============================================================================

#[derive(Debug, Clone)]
pub struct CaptionCenter {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone)]
pub struct CaptionDict {
    pub char_caption: String,
    pub centers: Vec<CaptionCenter>,
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Convert a CharacterConfig to a caption dict for v4_prompt positive prompt
pub fn character_to_caption_dict(config: &CharacterConfig) -> CaptionDict {
    CaptionDict {
        char_caption: config.prompt.clone(),
        centers: vec![CaptionCenter {
            x: config.center_x,
            y: config.center_y,
        }],
    }
}

/// Convert a CharacterConfig to a caption dict for v4_negative_prompt
pub fn character_to_negative_caption_dict(config: &CharacterConfig) -> CaptionDict {
    CaptionDict {
        char_caption: config.negative_prompt.clone(),
        centers: vec![CaptionCenter {
            x: config.center_x,
            y: config.center_y,
        }],
    }
}
