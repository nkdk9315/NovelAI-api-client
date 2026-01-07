/**
 * NovelAI Client Constants
 * 定数・デフォルト値
 */

// =============================================================================
// API URLs
// =============================================================================

export const API_URL = "https://image.novelai.net/ai/generate-image";
export const STREAM_URL = "https://image.novelai.net/ai/generate-image-stream";
export const ENCODE_URL = "https://image.novelai.net/ai/encode-vibe";
export const SUBSCRIPTION_URL = "https://api.novelai.net/user/subscription";
export const AUGMENT_URL = "https://image.novelai.net/ai/augment-image";
export const UPSCALE_URL = "https://api.novelai.net/ai/upscale";


// =============================================================================
// デフォルト値
// =============================================================================

export const DEFAULT_NEGATIVE = [
    "nsfw, lowres, artistic error, film grain, scan artifacts, ",
    "worst quality, bad quality, jpeg artifacts, very displeasing, ",
    "chromatic aberration, dithering, halftone, screentone"
].join("");

export const DEFAULT_MODEL = "nai-diffusion-4-5-full";
export const DEFAULT_WIDTH = 832;
export const DEFAULT_HEIGHT = 1216;
export const DEFAULT_STEPS = 23;
export const DEFAULT_SCALE = 5.0;
export const DEFAULT_SAMPLER = "k_euler_ancestral";
export const DEFAULT_NOISE_SCHEDULE = "karras";
export const DEFAULT_VIBE_STRENGTH = 0.7;
export const DEFAULT_VIBE_INFO_EXTRACTED = 0.7;
export const DEFAULT_IMG2IMG_STRENGTH = 0.62;
export const DEFAULT_CFG_RESCALE = 0;

// Inpaint defaults
export const DEFAULT_INPAINT_STRENGTH = 0.7;
export const DEFAULT_INPAINT_NOISE = 0;
export const DEFAULT_INPAINT_COLOR_CORRECT = true;


// =============================================================================
// バリデーション定数
// =============================================================================

// サンプラー
export const VALID_SAMPLERS = [
    "k_euler",
    "k_euler_ancestral",
    "k_dpmpp_2s_ancestral",
    "k_dpmpp_2m_sde",
    "k_dpmpp_2m",
    "k_dpmpp_sde",
] as const;

// モデル
export const VALID_MODELS = [
    "nai-diffusion-4-curated-preview",
    "nai-diffusion-4-full",
    "nai-diffusion-4-5-curated",
    "nai-diffusion-4-5-full",
] as const;

// ノイズスケジュール
export const VALID_NOISE_SCHEDULES = [
    "karras",
    "exponential",
    "polyexponential",
] as const;

// モデルキーマップ（Vibeファイル用）
export const MODEL_KEY_MAP: Record<string, string> = {
    "nai-diffusion-4-curated-preview": "v4curated",
    "nai-diffusion-4-full": "v4full",
    "nai-diffusion-4-5-curated": "v4-5curated",
    "nai-diffusion-4-5-full": "v4-5full",
};


// =============================================================================
// 制限値
// =============================================================================

// プロンプト
export const MAX_PROMPT_CHARS = 2000;  // 文字数制限（512トークン×4文字の目安）
export const MAX_TOKENS = 512;  // トークン数制限（T5 Tokenizer）


// ピクセル
export const MAX_PIXELS = 3_145_728;  // 2048 * 1536 (サーバー側生成制限)
export const MIN_DIMENSION = 64;
export const MAX_DIMENSION = 1024;

// キャラクター
export const MAX_CHARACTERS = 6;

// Vibe
export const MAX_VIBES = 10;  // 5以上は1Vibeあたり2Anlas消費

// 生成パラメータ
export const MIN_STEPS = 1;
export const MAX_STEPS = 50;
export const MIN_SCALE = 0.0;
export const MAX_SCALE = 10.0;
export const MAX_SEED = 4294967295;  // 2^32 - 1

// 参照画像
export const MAX_REF_IMAGE_SIZE_MB = 10;
export const MAX_REF_IMAGE_DIMENSION = 4096;

// キャラクター参照画像サイズ
export const CHARREF_PORTRAIT_SIZE = { width: 1024, height: 1536 };  // 縦長
export const CHARREF_LANDSCAPE_SIZE = { width: 1536, height: 1024 };  // 横長
export const CHARREF_SQUARE_SIZE = { width: 1472, height: 1472 };  // 正方形


// =============================================================================
// Augment ツール定数
// =============================================================================

// Augmentツールタイプ
export const AUGMENT_REQ_TYPES = [
  "colorize",
  "declutter",
  "emotion",
  "sketch",
  "lineart",
  "bg-removal",
] as const;

// 表情キーワード (emotion用)
export const EMOTION_KEYWORDS = [
  "neutral", "happy", "sad", "angry", "scared", "surprised",
  "tired", "excited", "nervous", "thinking", "confused", "shy",
  "disgusted", "smug", "bored", "laughing", "irritated", "aroused",
  "embarrassed", "love", "worried", "determined", "hurt", "playful",
] as const;

// Defry範囲 (0=最強変更, 5=最弱変更)
export const MIN_DEFRY = 0;
export const MAX_DEFRY = 5;
export const DEFAULT_DEFRY = 3;

// Upscaleスケール
export const VALID_UPSCALE_SCALES = [2, 4] as const;
export const DEFAULT_UPSCALE_SCALE = 4;


// =============================================================================
// Enhance (品質アップ) プリセット
// =============================================================================

// レベル別の strength/noise プリセット（UI上のレベル1～5に対応）
export const ENHANCE_LEVEL_PRESETS = {
  1: { strength: 0.2, noise: 0 },
  2: { strength: 0.4, noise: 0 },
  3: { strength: 0.5, noise: 0 },
  4: { strength: 0.6, noise: 0 },
  5: { strength: 0.7, noise: 0.1 },
} as const;

export type EnhanceLevel = keyof typeof ENHANCE_LEVEL_PRESETS;
