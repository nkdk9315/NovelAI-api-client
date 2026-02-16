import Foundation

// MARK: - API URLs

public func apiURL() -> String {
    ProcessInfo.processInfo.environment["NOVELAI_API_URL"]
        ?? "https://image.novelai.net/ai/generate-image"
}

public func streamURL() -> String {
    ProcessInfo.processInfo.environment["NOVELAI_STREAM_URL"]
        ?? "https://image.novelai.net/ai/generate-image-stream"
}

public func encodeURL() -> String {
    ProcessInfo.processInfo.environment["NOVELAI_ENCODE_URL"]
        ?? "https://image.novelai.net/ai/encode-vibe"
}

public func subscriptionURL() -> String {
    ProcessInfo.processInfo.environment["NOVELAI_SUBSCRIPTION_URL"]
        ?? "https://api.novelai.net/user/subscription"
}

public func augmentURL() -> String {
    ProcessInfo.processInfo.environment["NOVELAI_AUGMENT_URL"]
        ?? "https://image.novelai.net/ai/augment-image"
}

public func upscaleURL() -> String {
    ProcessInfo.processInfo.environment["NOVELAI_UPSCALE_URL"]
        ?? "https://api.novelai.net/ai/upscale"
}

// MARK: - Enums

/// Supported diffusion models
public enum Model: String, CaseIterable, Codable, Sendable {
    case naiDiffusion4CuratedPreview = "nai-diffusion-4-curated-preview"
    case naiDiffusion4Full = "nai-diffusion-4-full"
    case naiDiffusion45Curated = "nai-diffusion-4-5-curated"
    case naiDiffusion45Full = "nai-diffusion-4-5-full"
}

/// Supported samplers
public enum Sampler: String, CaseIterable, Codable, Sendable {
    case kEuler = "k_euler"
    case kEulerAncestral = "k_euler_ancestral"
    case kDpmpp2sAncestral = "k_dpmpp_2s_ancestral"
    case kDpmpp2mSde = "k_dpmpp_2m_sde"
    case kDpmpp2m = "k_dpmpp_2m"
    case kDpmppSde = "k_dpmpp_sde"
}

/// Supported noise schedules
public enum NoiseSchedule: String, CaseIterable, Codable, Sendable {
    case karras
    case exponential
    case polyexponential
}

/// Augment request types
public enum AugmentReqType: String, CaseIterable, Codable, Sendable {
    case colorize
    case declutter
    case emotion
    case sketch
    case lineart
    case bgRemoval = "bg-removal"
}

/// Emotion keywords for augment emotion tool
public enum EmotionKeyword: String, CaseIterable, Codable, Sendable {
    case neutral, happy, sad, angry, scared, surprised
    case tired, excited, nervous, thinking, confused, shy
    case disgusted, smug, bored, laughing, irritated, aroused
    case embarrassed, love, worried, determined, hurt, playful
}

// MARK: - Default Values

public let DEFAULT_NEGATIVE: String = [
    "nsfw", "lowres", "artistic error", "film grain", "scan artifacts",
    "worst quality", "bad quality", "jpeg artifacts", "very displeasing",
    "chromatic aberration", "dithering", "halftone", "screentone",
].joined(separator: ", ")

public let DEFAULT_MODEL: Model = .naiDiffusion45Full
public let DEFAULT_WIDTH: Int = 832
public let DEFAULT_HEIGHT: Int = 1216
public let DEFAULT_STEPS: Int = 23
public let DEFAULT_SCALE: Double = 5.0
public let DEFAULT_SAMPLER: Sampler = .kEulerAncestral
public let DEFAULT_NOISE_SCHEDULE: NoiseSchedule = .karras
public let DEFAULT_VIBE_STRENGTH: Double = 0.7
public let DEFAULT_VIBE_INFO_EXTRACTED: Double = 0.7
public let DEFAULT_IMG2IMG_STRENGTH: Double = 0.62
public let DEFAULT_CFG_RESCALE: Double = 0

// Inpaint defaults
public let DEFAULT_INPAINT_STRENGTH: Double = 0.7
public let DEFAULT_INPAINT_NOISE: Double = 0
public let DEFAULT_INPAINT_COLOR_CORRECT: Bool = true

// MARK: - Validation Constants

/// Model key map for vibe files
public let MODEL_KEY_MAP: [Model: String] = [
    .naiDiffusion4CuratedPreview: "v4curated",
    .naiDiffusion4Full: "v4full",
    .naiDiffusion45Curated: "v4-5curated",
    .naiDiffusion45Full: "v4-5full",
]

// MARK: - Limits

// Prompt
public let MAX_TOKENS: Int = 512

// Pixels
public let MAX_PIXELS: Int = 3_145_728  // 2048 * 1536
public let MIN_DIMENSION: Int = 64
public let MAX_GENERATION_DIMENSION: Int = 2048

// Characters
public let MAX_CHARACTERS: Int = 6

// Vibes
public let MAX_VIBES: Int = 10

// Generation parameters
public let MIN_STEPS: Int = 1
public let MAX_STEPS: Int = 50
public let MIN_SCALE: Double = 0.0
public let MAX_SCALE: Double = 10.0
public let MAX_SEED: UInt32 = 4_294_967_295  // 2^32 - 1

// Reference images
public let MAX_REF_IMAGE_SIZE_MB: Int = 10
public let MAX_REF_IMAGE_DIMENSION: Int = 4096

// Character reference image sizes
public let CHARREF_PORTRAIT_SIZE = (width: 1024, height: 1536)
public let CHARREF_LANDSCAPE_SIZE = (width: 1536, height: 1024)
public let CHARREF_SQUARE_SIZE = (width: 1472, height: 1472)
public let CHARREF_PORTRAIT_THRESHOLD: Double = 0.8
public let CHARREF_LANDSCAPE_THRESHOLD: Double = 1.25

// MARK: - Augment Constants

// Defry range
public let MIN_DEFRY: Int = 0
public let MAX_DEFRY: Int = 5
public let DEFAULT_DEFRY: Int = 3

// Upscale
public let VALID_UPSCALE_SCALES: [Int] = [2, 4]
public let DEFAULT_UPSCALE_SCALE: Int = 4

// MARK: - Enhance Presets

public struct EnhanceLevelPreset: Sendable {
    public let strength: Double
    public let noise: Double
}

public let ENHANCE_LEVEL_PRESETS: [Int: EnhanceLevelPreset] = [
    1: EnhanceLevelPreset(strength: 0.2, noise: 0),
    2: EnhanceLevelPreset(strength: 0.4, noise: 0),
    3: EnhanceLevelPreset(strength: 0.5, noise: 0),
    4: EnhanceLevelPreset(strength: 0.6, noise: 0),
    5: EnhanceLevelPreset(strength: 0.7, noise: 0.1),
]

// MARK: - Anlas Cost Constants

// Opus free conditions
public let OPUS_FREE_PIXELS: Int = 1_048_576        // 1024x1024
public let OPUS_FREE_MAX_STEPS: Int = 28
public let OPUS_MIN_TIER: Int = 3

// Per-image cost limits
public let MAX_COST_PER_IMAGE: Int = 140
public let MIN_COST_PER_IMAGE: Int = 2

// Grid size (for inpaint) — NovelAI API requires dimensions to be multiples of 64
public let GRID_SIZE: Int = 64

// Vibe costs
public let VIBE_BATCH_PRICE: Int = 2
public let VIBE_FREE_THRESHOLD: Int = 4
public let VIBE_ENCODE_PRICE: Int = 2

// Character reference cost
public let CHAR_REF_PRICE: Int = 5

// Inpaint threshold — masks below this fraction of OPUS_FREE_PIXELS get size-corrected
public let INPAINT_THRESHOLD_RATIO: Double = 0.8

// V4 cost coefficients — empirically derived from NovelAI pricing model
public let V4_COST_COEFF_LINEAR: Double = 2.951823174884865e-6
public let V4_COST_COEFF_STEP: Double = 5.753298233447344e-7

// Augment fixed parameters
public let AUGMENT_FIXED_STEPS: Int = 28
public let AUGMENT_MIN_PIXELS: Int = 1_048_576

// Background removal special calculation
public let BG_REMOVAL_MULTIPLIER: Int = 3
public let BG_REMOVAL_ADDEND: Int = 5

// Upscale cost table [(maxPixels, cost)] in ascending order
public let UPSCALE_COST_TABLE: [(maxPixels: Int, cost: Int)] = [
    (262_144, 1),
    (409_600, 2),
    (524_288, 3),
    (786_432, 5),
    (1_048_576, 7),
]

// Upscale Opus free pixel limit
public let UPSCALE_OPUS_FREE_PIXELS: Int = 409_600

// MARK: - Network & Security Constants

public let DEFAULT_REQUEST_TIMEOUT_MS: Int = 60_000
public let MAX_DECOMPRESSED_IMAGE_SIZE: Int = 50 * 1024 * 1024  // 50MB
public let MAX_RESPONSE_SIZE: Int = 200 * 1024 * 1024           // 200MB
public let MAX_ZIP_ENTRIES: Int = 10
public let MAX_COMPRESSION_RATIO: Int = 100
public let MAX_VIBE_ENCODING_LENGTH: Int = 5_000_000            // ~3.5MB base64
