use serde::{Deserialize, Serialize};

use crate::constants::*;

// =============================================================================
// Enums
// =============================================================================

/// Image generation action type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GenerateAction {
    #[default]
    Generate,
    Img2Img,
    Infill,
}

impl GenerateAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            GenerateAction::Generate => "generate",
            GenerateAction::Img2Img => "img2img",
            GenerateAction::Infill => "infill",
        }
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
    FilePath(String),
    Base64(String),
    DataUrl(String),
    Bytes(Vec<u8>),
}

impl ImageInput {
    pub fn is_empty(&self) -> bool {
        match self {
            ImageInput::FilePath(s) | ImageInput::Base64(s) | ImageInput::DataUrl(s) => s.is_empty(),
            ImageInput::Bytes(b) => b.is_empty(),
        }
    }
}

/// Vibe item: either a pre-encoded VibeEncodeResult, a file path, or a raw encoding string
#[derive(Debug, Clone)]
pub enum VibeItem {
    Encoded(VibeEncodeResult),
    FilePath(String),
    RawEncoding(String),
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
    pub source_image: Option<ImageInput>,
    pub img2img_strength: f64,
    pub img2img_noise: f64,
    pub mask: Option<ImageInput>,
    pub mask_strength: Option<f64>,
    pub inpaint_color_correct: bool,
    pub hybrid_img2img_strength: Option<f64>,
    pub hybrid_img2img_noise: Option<f64>,
    pub characters: Option<Vec<CharacterConfig>>,
    pub vibes: Option<Vec<VibeItem>>,
    pub vibe_strengths: Option<Vec<f64>>,
    pub vibe_info_extracted: Option<Vec<f64>>,
    pub character_reference: Option<CharacterReferenceConfig>,
    pub negative_prompt: Option<String>,
    pub save_path: Option<String>,
    pub save_dir: Option<String>,
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
            source_image: None,
            img2img_strength: DEFAULT_IMG2IMG_STRENGTH,
            img2img_noise: 0.0,
            mask: None,
            mask_strength: None,
            inpaint_color_correct: DEFAULT_INPAINT_COLOR_CORRECT,
            hybrid_img2img_strength: None,
            hybrid_img2img_noise: None,
            characters: None,
            vibes: None,
            vibe_strengths: None,
            vibe_info_extracted: None,
            character_reference: None,
            negative_prompt: None,
            save_path: None,
            save_dir: None,
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
    pub save_path: Option<String>,
    pub save_dir: Option<String>,
    pub save_filename: Option<String>,
}

impl Default for EncodeVibeParams {
    fn default() -> Self {
        Self {
            image: ImageInput::Bytes(Vec::new()),
            model: Model::default(),
            information_extracted: DEFAULT_VIBE_INFO_EXTRACTED,
            strength: DEFAULT_VIBE_STRENGTH,
            save_path: None,
            save_dir: None,
            save_filename: None,
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
    pub save_path: Option<String>,
    pub save_dir: Option<String>,
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
    pub save_path: Option<String>,
    pub save_dir: Option<String>,
}

impl Default for UpscaleParams {
    fn default() -> Self {
        Self {
            image: ImageInput::Bytes(Vec::new()),
            scale: DEFAULT_UPSCALE_SCALE,
            save_path: None,
            save_dir: None,
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
