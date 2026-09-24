/**
 * NovelAI Client Constants
 * 定数・デフォルト値
 */

// =============================================================================
// API URLs
// =============================================================================

export const API_URL = process.env.NOVELAI_API_URL ?? "https://image.novelai.net/ai/generate-image";
export const STREAM_URL = process.env.NOVELAI_STREAM_URL ?? "https://image.novelai.net/ai/generate-image-stream";
export const ENCODE_URL = process.env.NOVELAI_ENCODE_URL ?? "https://image.novelai.net/ai/encode-vibe";
export const SUBSCRIPTION_URL = process.env.NOVELAI_SUBSCRIPTION_URL ?? "https://image.novelai.net/user/subscription";
export const AUGMENT_URL = process.env.NOVELAI_AUGMENT_URL ?? "https://image.novelai.net/ai/augment-image";
export const UPSCALE_URL = process.env.NOVELAI_UPSCALE_URL ?? "https://image.novelai.net/ai/upscale";

// Cloudflare は一部の既定 User-Agent を弾くため明示する
export const USER_AGENT = "novelai-api-client-ts/1.0.0";


// =============================================================================
// デフォルト値
// =============================================================================

export const DEFAULT_NEGATIVE = [
  "nsfw", "lowres", "artistic error", "film grain", "scan artifacts",
  "worst quality", "bad quality", "jpeg artifacts", "very displeasing",
  "chromatic aberration", "dithering", "halftone", "screentone"
].join(", ");

// V5 のデフォルトネガティブ (公式サイトの V5 "heavy" UC プリセット + nsfw)
export const DEFAULT_NEGATIVE_V5 = [
  "nsfw", "lowres", "artistic error", "film grain", "scan artifacts",
  "worst quality", "bad quality", "jpeg artifacts", "very displeasing",
  "chromatic aberration", "dithering", "halftone", "screentone",
  "multiple views", "logo", "too many watermarks", "negative space", "blank page"
].join(", ");

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

// 出力画像形式 (公式ドキュメント: png / webp。webp はロスレスでアルファ・メタデータ付き)
export const VALID_IMAGE_FORMATS = ["png", "webp"] as const;
export type ImageFormat = typeof VALID_IMAGE_FORMATS[number];
export const DEFAULT_IMAGE_FORMAT: ImageFormat = "png";

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
  "nai-diffusion-5-curated",
  "nai-diffusion-5-full",
] as const;

export type Model = typeof VALID_MODELS[number];

// V5 モデル (Vibe / CharRef 非対応、透過対応、Qwen トークナイザー、コスト1.5倍)
export const V5_MODELS = [
  "nai-diffusion-5-curated",
  "nai-diffusion-5-full",
] as const;

export function isV5Model(model: string): boolean {
  return (V5_MODELS as readonly string[]).includes(model);
}

// Vibe Transfer / Character Reference に対応するモデル (V4 / V4.5)
export const VIBE_MODELS = [
  "nai-diffusion-4-curated-preview",
  "nai-diffusion-4-full",
  "nai-diffusion-4-5-curated",
  "nai-diffusion-4-5-full",
] as const;

// inpaint 用モデル名。V5 curated には inpaint モデルがなく、公式サイトも 4.5 curated を使う
const INPAINT_MODEL_OVERRIDES: Record<string, string> = {
  "nai-diffusion-5-curated": "nai-diffusion-4-5-curated-inpainting",
};

export function getInpaintModel(model: string): string {
  if (model.endsWith("-inpainting")) return model;
  return INPAINT_MODEL_OVERRIDES[model] ?? `${model}-inpainting`;
}

// V5 の品質タグ (プロンプト末尾に追加するもの。クライアントは自動では付けない)
export const V5_QUALITY_TAGS = {
  standard: "very aesthetic, masterpiece, no text",
  light: "very aesthetic, amazing quality, no text",
} as const;

// 透過背景 (V5) でプロンプトに追加するタグ
export const TRANSPARENT_BACKGROUND_TAG = "transparent background";

// ノイズスケジュール
export const VALID_NOISE_SCHEDULES = [
  "karras",
  "exponential",
  "polyexponential",
] as const;

// モデルキーマップ（Vibeファイル用）
export const MODEL_KEY_MAP = {
  "nai-diffusion-4-curated-preview": "v4curated",
  "nai-diffusion-4-full": "v4full",
  "nai-diffusion-4-5-curated": "v4-5curated",
  "nai-diffusion-4-5-full": "v4-5full",
} as const;


// =============================================================================
// 制限値
// =============================================================================

// プロンプト
export const MAX_TOKENS = 512;  // トークン数制限（V4 / V4.5, T5 Tokenizer）
export const MAX_TOKENS_V5_FULL = 1471;     // nai-diffusion-5-full (Qwen Tokenizer)
export const MAX_TOKENS_V5_CURATED = 703;   // nai-diffusion-5-curated (Qwen Tokenizer)

/** モデルごとのプロンプトトークン上限 (公式サイトと同じ値) */
export function getMaxTokens(model: string): number {
  if (model.startsWith("nai-diffusion-5-full")) return MAX_TOKENS_V5_FULL;
  if (model.startsWith("nai-diffusion-5-curated")) return MAX_TOKENS_V5_CURATED;
  return MAX_TOKENS;
}


// ピクセル
export const MAX_PIXELS = 3_145_728;  // 2048 * 1536 (サーバー側生成制限)
export const MIN_DIMENSION = 64;
export const MAX_GENERATION_DIMENSION = 2048;  // 単辺の最大値（MAX_PIXELSと合わせて制約）

// キャラクター
export const MAX_CHARACTERS = 6;      // V4 / V4.5
export const MAX_CHARACTERS_V5 = 32;  // V5

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
export const CHARREF_PORTRAIT_SIZE = { width: 1024, height: 1536 } as const;  // 縦長
export const CHARREF_LANDSCAPE_SIZE = { width: 1536, height: 1024 } as const;  // 横長
export const CHARREF_SQUARE_SIZE = { width: 1472, height: 1472 } as const;  // 正方形
export const CHARREF_PORTRAIT_THRESHOLD = 0.8;
export const CHARREF_LANDSCAPE_THRESHOLD = 1.25;


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
  "declutter-keep-bubbles",
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

// Upscaleスケール (サーバーは常に2倍で返す。倍率指定のパラメータはない)
export const VALID_UPSCALE_SCALES = [2] as const;
export const DEFAULT_UPSCALE_SCALE = 2;

// Upscaleリクエストの固定値 (公式サイトと同じ)
export const UPSCALE_MODEL = "nai-diffusion-5-curated";
export const UPSCALE_DECLARED_BLUR_SIGMA = 0;

// Upscale入力画像の最大ピクセル数（UPSCALE_COST_TABLEの最大値に対応）
// これを超える画像はAPIが 400 "Image resolution too high" を返す
export const UPSCALE_MAX_PIXELS = 1_048_576;  // 1024 × 1024


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


// =============================================================================
// Anlas コスト計算定数
// =============================================================================

// Opus無料条件
export const OPUS_FREE_PIXELS = 1_048_576;        // 1024×1024
export const OPUS_FREE_MAX_STEPS = 28;
export const OPUS_MIN_TIER = 3;

// 1枚あたりコスト制限
export const MAX_COST_PER_IMAGE = 140;
export const MIN_COST_PER_IMAGE = 2;

// グリッドサイズ（Inpaint用）
export const GRID_SIZE = 64;

// Vibeコスト
export const VIBE_BATCH_PRICE = 2;
export const VIBE_FREE_THRESHOLD = 4;
export const VIBE_ENCODE_PRICE = 2;

// キャラクター参照コスト
export const CHAR_REF_PRICE = 5;

// Inpaint閾値
export const INPAINT_THRESHOLD_RATIO = 0.8;

// V4コスト計算係数
export const V4_COST_COEFF_LINEAR = 2.951823174884865e-6;
export const V4_COST_COEFF_STEP = 5.753298233447344e-7;

// V5 はV4式の1.5倍
export const V5_COST_MULTIPLIER = 1.5;

// V5 の Opus 使用量から残り枚数を見積もる係数 (公式サイト: 1% ≈ 17.3枚)
export const OPUS_USAGE_IMAGES_PER_PERCENT = 17.3;
// 残り少ないとみなす閾値 (%)
export const OPUS_USAGE_LOW_PERCENT = 5;

// Augment固定パラメータ
export const AUGMENT_FIXED_STEPS = 28;
export const AUGMENT_MIN_PIXELS = 1_048_576;

// 背景削除特別計算
export const BG_REMOVAL_MULTIPLIER = 3;
export const BG_REMOVAL_ADDEND = 5;

// アップスケールコストテーブル [最大ピクセル数, コスト]（昇順）
// Opus無料はない (2026-09 の公式サイトで確認)
export const UPSCALE_COST_TABLE: ReadonlyArray<readonly [number, number]> = [
  [1_048_576, 1],
  [1_747_627, 2],
  [2_446_678, 3],
  [3_145_728, 4],
] as const;


// =============================================================================
// ネットワーク・セキュリティ定数
// =============================================================================

export const DEFAULT_REQUEST_TIMEOUT_MS = 60_000;  // 60秒
export const MAX_DECOMPRESSED_IMAGE_SIZE = 50 * 1024 * 1024;  // 50MB
export const MAX_RESPONSE_SIZE = 200 * 1024 * 1024;           // 200MB
export const MAX_ZIP_ENTRIES = 10;
export const MAX_COMPRESSION_RATIO = 100;
export const MAX_VIBE_ENCODING_LENGTH = 5_000_000;            // ~3.5MB base64
