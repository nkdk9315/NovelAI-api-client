"""
NovelAI Client Pydantic Models
バリデーション付きデータモデル

validation_spec.yaml に基づいて実装
"""

import json
from datetime import datetime
from pathlib import Path
from typing import Optional, Union, List, Literal, Annotated, Any

from pydantic import (
    BaseModel,
    Field,
    field_validator,
    model_validator,
    ConfigDict,
)

from .constants import (
    VALID_SAMPLERS,
    VALID_MODELS,
    VALID_NOISE_SCHEDULES,
    MODEL_KEY_MAP,
    MAX_PROMPT_CHARS,
    MAX_PIXELS,
    MAX_CHARACTERS,
    MAX_VIBES,
    MAX_STEPS,
    MAX_SCALE,
    MAX_SEED,
    DEFAULT_MODEL,
    DEFAULT_WIDTH,
    DEFAULT_HEIGHT,
    DEFAULT_STEPS,
    DEFAULT_SCALE,
    DEFAULT_SAMPLER,
    DEFAULT_NOISE_SCHEDULE,
    DEFAULT_IMG2IMG_STRENGTH,
)


# =============================================================================
# CharacterConfig
# =============================================================================

class CharacterConfigModel(BaseModel):
    """
    キャラクター設定
    
    マルチキャラクター生成時に各キャラクターの位置とプロンプトを指定する
    """
    model_config = ConfigDict(validate_assignment=True)
    
    prompt: Annotated[str, Field(min_length=1, max_length=MAX_PROMPT_CHARS)]
    """キャラクターのプロンプト（例: "1girl, blonde hair, blue eyes"）"""
    
    center_x: Annotated[float, Field(ge=0.0, le=1.0)] = 0.5
    """キャラクターのX座標（左が0、右が1）"""
    
    center_y: Annotated[float, Field(ge=0.0, le=1.0)] = 0.5
    """キャラクターのY座標（上が0、下が1）"""
    
    negative_prompt: Annotated[str, Field(max_length=MAX_PROMPT_CHARS)] = ""
    """キャラクター固有のネガティブプロンプト"""
    
    def to_caption_dict(self) -> dict:
        """v4_prompt用のchar_caption辞書を生成"""
        return {
            "char_caption": self.prompt,
            "centers": [{"x": self.center_x, "y": self.center_y}]
        }
    
    def to_negative_caption_dict(self) -> dict:
        """v4_negative_prompt用のchar_caption辞書を生成"""
        return {
            "char_caption": self.negative_prompt,
            "centers": [{"x": self.center_x, "y": self.center_y}]
        }


# =============================================================================
# CharacterReferenceConfig
# =============================================================================

class CharacterReferenceConfigModel(BaseModel):
    """
    キャラクター参照設定（Director Reference）
    
    参照画像からキャラクターの特徴を抽出して生成に反映する
    """
    model_config = ConfigDict(validate_assignment=True, arbitrary_types_allowed=True)
    
    image: Union[str, Path, bytes]
    """参照画像（ファイルパス、バイトデータ、またはBase64文字列）"""
    
    fidelity: Annotated[float, Field(ge=0.0, le=1.0)] = 1.0
    """忠実度（キャラクターの反映度）"""
    
    include_style: bool = True
    """絵柄も参照するか（True: character&style, False: character only）"""
    
    @field_validator('image')
    @classmethod
    def validate_image(cls, v):
        """画像パスの存在チェック"""
        if isinstance(v, (str, Path)):
            path = Path(v)
            # パスとして存在する場合のみチェック
            if path.suffix.lower() in ('.png', '.jpg', '.jpeg', '.webp'):
                if not path.exists():
                    raise ValueError(f"画像ファイルが見つかりません: {path}")
        return v


# =============================================================================
# VibeEncodeResult
# =============================================================================

class VibeEncodeResultModel(BaseModel):
    """
    Vibeエンコード結果（DB保存対応）
    """
    model_config = ConfigDict(validate_assignment=True, arbitrary_types_allowed=True)
    
    encoding: Annotated[str, Field(min_length=1)]
    """Base64エンコードされたVibeデータ"""
    
    model: str
    """使用モデル"""
    
    information_extracted: Annotated[float, Field(ge=0.0, le=1.0)]
    """情報抽出量"""
    
    strength: Annotated[float, Field(ge=0.0, le=1.0)]
    """推奨Vibe強度"""
    
    source_image_hash: Annotated[str, Field(pattern=r'^[a-f0-9]{64}$')]
    """元画像のSHA256ハッシュ（重複検出用）"""
    
    created_at: datetime
    """作成日時"""
    
    saved_path: Optional[Path] = None
    """保存したファイルパス（保存した場合）"""
    
    anlas_remaining: Optional[Annotated[int, Field(ge=0)]] = None
    """残りアンラス"""
    
    anlas_consumed: Optional[Annotated[int, Field(ge=0)]] = None
    """今回消費したアンラス"""
    
    @field_validator('model')
    @classmethod
    def validate_model(cls, v):
        if v not in VALID_MODELS:
            raise ValueError(f"無効なモデル: {v}。有効なモデル: {VALID_MODELS}")
        return v
    
    def __str__(self) -> str:
        """後方互換: 文字列として使用するとエンコードを返す"""
        return self.encoding
    
    def to_dict(self) -> dict:
        """DB保存用の辞書に変換"""
        return {
            "encoding": self.encoding,
            "model": self.model,
            "information_extracted": self.information_extracted,
            "strength": self.strength,
            "source_image_hash": self.source_image_hash,
            "created_at": self.created_at.isoformat(),
            "saved_path": str(self.saved_path) if self.saved_path else None,
            "anlas_remaining": self.anlas_remaining,
            "anlas_consumed": self.anlas_consumed,
        }
    
    def save(self, path: Union[str, Path]) -> Path:
        """naiv4vibe形式で保存"""
        path = Path(path)
        path.parent.mkdir(parents=True, exist_ok=True)
        
        model_key = MODEL_KEY_MAP.get(self.model, "v4-5full")
        
        vibe_data = {
            "identifier": "novelai-vibe-transfer",
            "version": 1,
            "type": "encoding",
            "id": self.source_image_hash,
            "encodings": {
                model_key: {
                    "unknown": {
                        "encoding": self.encoding,
                        "params": {
                            "information_extracted": self.information_extracted
                        }
                    }
                }
            },
            "name": f"{self.source_image_hash[:6]}-{self.source_image_hash[-6:]}",
            "createdAt": self.created_at.isoformat(),
            "importInfo": {
                "model": self.model,
                "information_extracted": self.information_extracted,
                "strength": self.strength
            }
        }
        
        with open(path, "w", encoding="utf-8") as f:
            json.dump(vibe_data, f, indent=2)
        
        self.saved_path = path
        return path


# =============================================================================
# GenerateResult
# =============================================================================

class GenerateResultModel(BaseModel):
    """
    画像生成結果
    """
    model_config = ConfigDict(validate_assignment=True, arbitrary_types_allowed=True)
    
    image_data: bytes
    """生成された画像のバイトデータ"""
    
    seed: Annotated[int, Field(ge=0, le=MAX_SEED)]
    """使用されたシード値"""
    
    anlas_remaining: Optional[Annotated[int, Field(ge=0)]] = None
    """残りアンラス"""
    
    anlas_consumed: Optional[Annotated[int, Field(ge=0)]] = None
    """今回消費したアンラス"""
    
    saved_path: Optional[Path] = None
    """保存先パス（保存した場合）"""
    
    def save(self, path: Union[str, Path]) -> Path:
        """画像を指定パスに保存"""
        path = Path(path)
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "wb") as f:
            f.write(self.image_data)
        self.saved_path = path
        return path


# =============================================================================
# GenerateParams（generate()メソッドの引数）
# =============================================================================

class GenerateParamsModel(BaseModel):
    """
    画像生成パラメータ
    
    generate()メソッドの引数をバリデーション
    """
    model_config = ConfigDict(validate_assignment=True, arbitrary_types_allowed=True)
    
    # === 基本プロンプト ===
    prompt: Annotated[str, Field(min_length=0, max_length=MAX_PROMPT_CHARS)]
    """生成プロンプト（キャラクター指定時は空文字可）"""
    
    # === Action & Image2Image ===
    action: Literal["generate", "img2img"] = "generate"
    """生成アクション"""
    
    source_image: Optional[Union[str, Path, bytes]] = None
    """img2img用の入力画像"""
    
    img2img_strength: Annotated[float, Field(ge=0.0, le=1.0)] = DEFAULT_IMG2IMG_STRENGTH
    """img2img変換強度（低いほど元画像に近い）"""
    
    img2img_noise: Annotated[float, Field(ge=0.0, le=1.0)] = 0.0
    """img2imgノイズ量"""
    
    # === キャラクター設定 ===
    characters: Optional[Annotated[List[CharacterConfigModel], Field(max_length=MAX_CHARACTERS)]] = None
    """キャラクター設定のリスト（最大6）"""
    
    # === Vibe Transfer ===
    vibes: Optional[Annotated[List[Any], Field(max_length=MAX_VIBES)]] = None
    """Vibeリスト（最大10、5以上は1Vibeあたり2Anlas消費）"""
    
    vibe_strengths: Optional[List[Annotated[float, Field(ge=0.0, le=1.0)]]] = None
    """各Vibeの強度"""
    
    vibe_info_extracted: Optional[List[Annotated[float, Field(ge=0.0, le=1.0)]]] = None
    """各Vibeのinformation_extracted値"""
    
    # === Character Reference ===
    character_reference: Optional[CharacterReferenceConfigModel] = None
    """キャラクター参照設定（vibesと併用不可）"""
    
    # === プロンプト ===
    negative_prompt: Optional[Annotated[str, Field(max_length=MAX_PROMPT_CHARS)]] = None
    """ネガティブプロンプト"""
    
    # === 出力オプション ===
    save_path: Optional[Union[str, Path]] = None
    """保存先ファイルパス"""
    
    save_dir: Optional[Union[str, Path]] = None
    """保存先ディレクトリ（自動ファイル名）"""
    
    # === 生成パラメータ ===
    model: str = DEFAULT_MODEL
    """モデル名"""
    
    width: Annotated[int, Field(ge=64)] = DEFAULT_WIDTH
    """画像幅（ピクセル、64の倍数、width*height <= 1,048,576）"""
    
    height: Annotated[int, Field(ge=64)] = DEFAULT_HEIGHT
    """画像高さ（ピクセル、64の倍数、width*height <= 1,048,576）"""
    
    steps: Annotated[int, Field(ge=1, le=MAX_STEPS)] = DEFAULT_STEPS
    """生成ステップ数"""
    
    scale: Annotated[float, Field(ge=0.0, le=MAX_SCALE)] = DEFAULT_SCALE
    """CFG Scale"""
    
    seed: Optional[Annotated[int, Field(ge=0, le=MAX_SEED)]] = None
    """シード値（Noneでランダム）"""
    
    sampler: str = DEFAULT_SAMPLER
    """サンプラー"""
    
    noise_schedule: str = DEFAULT_NOISE_SCHEDULE
    """ノイズスケジュール"""
    
    # === Field Validators ===
    
    @field_validator('model')
    @classmethod
    def validate_model(cls, v):
        if v not in VALID_MODELS:
            raise ValueError(f"無効なモデル: {v}。有効なモデル: {VALID_MODELS}")
        return v
    
    @field_validator('sampler')
    @classmethod
    def validate_sampler(cls, v):
        if v not in VALID_SAMPLERS:
            raise ValueError(f"無効なサンプラー: {v}。有効なサンプラー: {VALID_SAMPLERS}")
        return v
    
    @field_validator('noise_schedule')
    @classmethod
    def validate_noise_schedule(cls, v):
        if v not in VALID_NOISE_SCHEDULES:
            raise ValueError(f"無効なノイズスケジュール: {v}。有効な値: {VALID_NOISE_SCHEDULES}")
        return v
    
    @field_validator('width', 'height')
    @classmethod
    def validate_multiple_of_64(cls, v, info):
        if v % 64 != 0:
            raise ValueError(f"{info.field_name}は64の倍数である必要があります（現在: {v}）")
        return v
    
    # === Cross-Field Validators ===
    
    @model_validator(mode='after')
    def validate_cross_fields(self):
        errors = []
        
        # 1. vibes と character_reference は同時使用不可
        if self.vibes and self.character_reference:
            errors.append(
                "vibesとcharacter_referenceは同時に使用できません。"
                "キャラクター参照機能ではVibe Transferは利用できません。"
            )
        
        # 2. action="img2img" の場合は source_image が必須
        if self.action == "img2img" and not self.source_image:
            errors.append("img2imgアクションにはsource_imageが必須です")
        
        # 3. vibes なしで vibe_strengths が指定されている
        if self.vibe_strengths and not self.vibes:
            errors.append("vibe_strengthsはvibesなしでは指定できません")
        
        # 4. vibes なしで vibe_info_extracted が指定されている
        if self.vibe_info_extracted and not self.vibes:
            errors.append("vibe_info_extractedはvibesなしでは指定できません")
        
        # 5. vibes と vibe_strengths の長さが一致しない
        if self.vibes and self.vibe_strengths:
            if len(self.vibes) != len(self.vibe_strengths):
                errors.append(
                    f"vibesの数({len(self.vibes)})と"
                    f"vibe_strengthsの数({len(self.vibe_strengths)})が一致しません"
                )
        
        # 6. vibes と vibe_info_extracted の長さが一致しない
        if self.vibes and self.vibe_info_extracted:
            if len(self.vibes) != len(self.vibe_info_extracted):
                errors.append(
                    f"vibesの数({len(self.vibes)})と"
                    f"vibe_info_extractedの数({len(self.vibe_info_extracted)})が一致しません"
                )
        
        # 7. width * height が MAX_PIXELS を超える
        total_pixels = self.width * self.height
        if total_pixels > MAX_PIXELS:
            errors.append(
                f"ピクセル数({total_pixels:,})が上限({MAX_PIXELS:,})を超えています。"
                f"Opusプランで無料なのは1024×1024以下です。"
                f"現在: {self.width}×{self.height}"
            )
        
        if errors:
            raise ValueError("\n".join(errors))
        
        return self


# =============================================================================
# EncodeVibeParams（encode_vibe()メソッドの引数）
# =============================================================================

class EncodeVibeParamsModel(BaseModel):
    """
    Vibeエンコードパラメータ
    """
    model_config = ConfigDict(validate_assignment=True, arbitrary_types_allowed=True)
    
    image: Union[str, Path, bytes]
    """画像ファイルパス、バイトデータ、またはBase64文字列"""
    
    model: str = DEFAULT_MODEL
    """使用するモデル名"""
    
    information_extracted: Annotated[float, Field(ge=0.0, le=1.0)] = 0.7
    """抽出する情報量"""
    
    strength: Annotated[float, Field(ge=0.0, le=1.0)] = 0.7
    """Vibe Transferの推奨強度"""
    
    save_path: Optional[Union[str, Path]] = None
    """保存先ファイルパス（指定時のみ保存）"""
    
    save_dir: Optional[Union[str, Path]] = None
    """保存先ディレクトリ（自動ファイル名で保存）"""
    
    @field_validator('model')
    @classmethod
    def validate_model(cls, v):
        if v not in VALID_MODELS:
            raise ValueError(f"無効なモデル: {v}。有効なモデル: {VALID_MODELS}")
        return v


# =============================================================================
# API Key バリデーション
# =============================================================================

class APIKeyModel(BaseModel):
    """
    NovelAI API Key バリデーション
    """
    api_key: Annotated[str, Field(pattern=r'^pst-.*', min_length=10)]
    """API Key（"pst-"で始まる）"""


# =============================================================================
# エクスポート
# =============================================================================

__all__ = [
    "CharacterConfigModel",
    "CharacterReferenceConfigModel",
    "VibeEncodeResultModel",
    "GenerateResultModel",
    "GenerateParamsModel",
    "EncodeVibeParamsModel",
    "APIKeyModel",
]
