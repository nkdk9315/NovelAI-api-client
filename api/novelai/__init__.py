"""
NovelAI Unified Image Generation Client Package
統合されたVibe Transfer & Image2Image APIクライアント

使用例:
    from api.novelai import NovelAIClient, CharacterConfig
    
    client = NovelAIClient()
    result = client.generate(
        prompt="1girl, beautiful anime girl",
        width=832,
        height=1216,
    )
    result.save("output.png")
"""

from .client import NovelAIClient

# Pydanticモデル（バリデーション付き）
from .models import (
    CharacterConfigModel,
    CharacterReferenceConfigModel,
    VibeEncodeResultModel,
    GenerateResultModel,
    GenerateParamsModel,
    EncodeVibeParamsModel,
    APIKeyModel,
)

# Dataclass版（後方互換性用）
from .dataclasses import (
    CharacterConfig,
    CharacterReferenceConfig,
    VibeEncodeResult,
    GenerateResult,
)

# 定数
from .constants import (
    # URLs
    API_URL,
    STREAM_URL,
    ENCODE_URL,
    SUBSCRIPTION_URL,
    # Defaults
    DEFAULT_NEGATIVE,
    DEFAULT_MODEL,
    DEFAULT_WIDTH,
    DEFAULT_HEIGHT,
    DEFAULT_STEPS,
    DEFAULT_SCALE,
    DEFAULT_SAMPLER,
    DEFAULT_NOISE_SCHEDULE,
    # Valid values
    VALID_SAMPLERS,
    VALID_MODELS,
    VALID_NOISE_SCHEDULES,
    # Limits
    MAX_PROMPT_CHARS,
    MAX_PIXELS,
    MAX_CHARACTERS,
    MAX_VIBES,
    MAX_STEPS,
    MAX_SCALE,
)

# ユーティリティ関数
from .utils import (
    get_image_bytes,
    get_image_base64,
    load_vibe_file,
)


__version__ = "1.0.0"

__all__ = [
    # Client
    "NovelAIClient",
    
    # Pydantic Models
    "CharacterConfigModel",
    "CharacterReferenceConfigModel",
    "VibeEncodeResultModel",
    "GenerateResultModel",
    "GenerateParamsModel",
    "EncodeVibeParamsModel",
    "APIKeyModel",
    
    # Dataclasses (backward compatibility)
    "CharacterConfig",
    "CharacterReferenceConfig",
    "VibeEncodeResult",
    "GenerateResult",
    
    # Constants
    "API_URL",
    "STREAM_URL",
    "ENCODE_URL",
    "SUBSCRIPTION_URL",
    "DEFAULT_NEGATIVE",
    "DEFAULT_MODEL",
    "DEFAULT_WIDTH",
    "DEFAULT_HEIGHT",
    "DEFAULT_STEPS",
    "DEFAULT_SCALE",
    "DEFAULT_SAMPLER",
    "DEFAULT_NOISE_SCHEDULE",
    "VALID_SAMPLERS",
    "VALID_MODELS",
    "VALID_NOISE_SCHEDULES",
    "MAX_PROMPT_CHARS",
    "MAX_PIXELS",
    "MAX_CHARACTERS",
    "MAX_VIBES",
    "MAX_STEPS",
    "MAX_SCALE",
    
    # Utils
    "get_image_bytes",
    "get_image_base64",
    "load_vibe_file",
]
