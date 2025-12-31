"""
NovelAI Client Constants
定数・デフォルト値

validation_spec.yaml に基づいて定義
"""

# =============================================================================
# API URLs
# =============================================================================

API_URL = "https://image.novelai.net/ai/generate-image"
STREAM_URL = "https://image.novelai.net/ai/generate-image-stream"
ENCODE_URL = "https://image.novelai.net/ai/encode-vibe"
SUBSCRIPTION_URL = "https://api.novelai.net/user/subscription"


# =============================================================================
# デフォルト値
# =============================================================================

DEFAULT_NEGATIVE = (
    "nsfw, lowres, artistic error, film grain, scan artifacts, "
    "worst quality, bad quality, jpeg artifacts, very displeasing, "
    "chromatic aberration, dithering, halftone, screentone"
)

DEFAULT_MODEL = "nai-diffusion-4-5-full"
DEFAULT_WIDTH = 832
DEFAULT_HEIGHT = 1216
DEFAULT_STEPS = 23
DEFAULT_SCALE = 5.0
DEFAULT_SAMPLER = "k_euler_ancestral"
DEFAULT_NOISE_SCHEDULE = "karras"
DEFAULT_VIBE_STRENGTH = 0.7
DEFAULT_VIBE_INFO_EXTRACTED = 0.7
DEFAULT_IMG2IMG_STRENGTH = 0.62


# =============================================================================
# バリデーション定数
# =============================================================================

# サンプラー
VALID_SAMPLERS = [
    "k_euler",
    "k_euler_ancestral",
    "k_dpmpp_2s_ancestral",
    "k_dpmpp_2m_sde",
    "k_dpmpp_2m",
    "k_dpmpp_sde",
]

# モデル
VALID_MODELS = [
    "nai-diffusion-4-curated-preview",
    "nai-diffusion-4-full",
    "nai-diffusion-4-5-curated",
    "nai-diffusion-4-5-full",
]

# ノイズスケジュール
VALID_NOISE_SCHEDULES = [
    "native",
    "karras",
    "exponential",
    "polyexponential",
]

# モデルキーマップ（Vibeファイル用）
MODEL_KEY_MAP = {
    "nai-diffusion-4-curated-preview": "v4curated",
    "nai-diffusion-4-full": "v4full",
    "nai-diffusion-4-5-curated": "v4-5curated",
    "nai-diffusion-4-5-full": "v4-5full",
}


# =============================================================================
# 制限値
# =============================================================================

# プロンプト
MAX_PROMPT_CHARS = 2000  # 文字数制限（512トークン×4文字の目安）

# ピクセル
MAX_PIXELS = 1_048_576  # 1024 * 1024 (Opusプラン無料枠)
MIN_DIMENSION = 64
MAX_DIMENSION = 1024

# キャラクター
MAX_CHARACTERS = 6

# Vibe
MAX_VIBES = 10  # 5以上は1Vibeあたり2Anlas消費

# 生成パラメータ
MIN_STEPS = 1
MAX_STEPS = 50
MIN_SCALE = 0.0
MAX_SCALE = 10.0
MAX_SEED = 4294967295  # 2^32 - 1

# 出力画像サイズ制限 (Zip Bomb Protection)
MAX_IMAGE_SIZE_BYTES = 100 * 1024 * 1024  # 100MB

# 参照画像
MAX_REF_IMAGE_SIZE_MB = 10
MAX_REF_IMAGE_DIMENSION = 4096

# キャラクター参照画像サイズ
CHARREF_PORTRAIT_SIZE = (1024, 1536)  # 縦長
CHARREF_LANDSCAPE_SIZE = (1536, 1024)  # 横長
CHARREF_SQUARE_SIZE = (1472, 1472)  # 正方形


# =============================================================================
# エクスポート
# =============================================================================

__all__ = [
    # URLs
    "API_URL",
    "STREAM_URL",
    "ENCODE_URL",
    "SUBSCRIPTION_URL",
    # Defaults
    "DEFAULT_NEGATIVE",
    "DEFAULT_MODEL",
    "DEFAULT_WIDTH",
    "DEFAULT_HEIGHT",
    "DEFAULT_STEPS",
    "DEFAULT_SCALE",
    "DEFAULT_SAMPLER",
    "DEFAULT_NOISE_SCHEDULE",
    "DEFAULT_VIBE_STRENGTH",
    "DEFAULT_VIBE_INFO_EXTRACTED",
    "DEFAULT_IMG2IMG_STRENGTH",
    # Valid values
    "VALID_SAMPLERS",
    "VALID_MODELS",
    "VALID_NOISE_SCHEDULES",
    "MODEL_KEY_MAP",
    # Limits
    "MAX_PROMPT_CHARS",
    "MAX_PIXELS",
    "MIN_DIMENSION",
    "MAX_DIMENSION",
    "MAX_CHARACTERS",
    "MAX_VIBES",
    "MIN_STEPS",
    "MAX_STEPS",
    "MIN_SCALE",
    "MAX_SCALE",
    "MAX_SEED",
    "MAX_IMAGE_SIZE_BYTES",
    "MAX_REF_IMAGE_SIZE_MB",
    "MAX_REF_IMAGE_DIMENSION",
    "CHARREF_PORTRAIT_SIZE",
    "CHARREF_LANDSCAPE_SIZE",
    "CHARREF_SQUARE_SIZE",
]
