"""
NovelAI Client Dataclasses
後方互換性用のdataclass版モデル

novelai_client.py の既存dataclassをそのまま移行
"""

import base64
import hashlib
import json
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Optional, Union, List

from .constants import MODEL_KEY_MAP


@dataclass
class CharacterConfig:
    """
    キャラクター設定
    
    Attributes:
        prompt: キャラクターのプロンプト（例: "1girl, blonde hair, blue eyes"）
        center_x: キャラクターのX座標 (0.0-1.0, 左が0、右が1)
        center_y: キャラクターのY座標 (0.0-1.0, 上が0、下が1)
        negative_prompt: キャラクター固有のネガティブプロンプト（オプション）
    """
    prompt: str
    center_x: float = 0.5
    center_y: float = 0.5
    negative_prompt: str = ""
    
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


@dataclass
class CharacterReferenceConfig:
    """
    キャラクター参照設定
    
    Attributes:
        image: 参照画像（ファイルパス、バイトデータ、またはBase64文字列）
        fidelity: 忠実度 (0.0-1.0) - キャラクターの反映度
        include_style: 絵柄も参照するか (True: character&style, False: character only)
    
    Note:
        information_extractedはAPI仕様により常に1.0に固定されています
    """
    image: Union[str, Path, bytes]
    fidelity: float = 1.0
    include_style: bool = True


@dataclass
class VibeEncodeResult:
    """
    Vibeエンコード結果（DB保存対応）
    
    Attributes:
        encoding: Base64エンコードされたVibeデータ
        model: 使用モデル
        information_extracted: 情報抽出量
        strength: 推奨Vibe強度
        source_image_hash: 元画像のSHA256ハッシュ（重複検出用）
        created_at: 作成日時
        saved_path: 保存したファイルパス（保存した場合）
        anlas_remaining: 残りアンラス
        anlas_consumed: 今回消費したアンラス
    """
    encoding: str
    model: str
    information_extracted: float
    strength: float
    source_image_hash: str
    created_at: datetime
    saved_path: Optional[Path] = None
    anlas_remaining: Optional[int] = None
    anlas_consumed: Optional[int] = None
    
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


@dataclass
class GenerateResult:
    """
    画像生成結果
    
    Attributes:
        image_data: 生成された画像のバイトデータ
        seed: 使用されたシード値
        anlas_remaining: 残りアンラス
        anlas_consumed: 今回消費したアンラス
        saved_path: 保存先パス（保存した場合）
    """
    image_data: bytes
    seed: int
    anlas_remaining: Optional[int] = None
    anlas_consumed: Optional[int] = None
    saved_path: Optional[Path] = None
    
    def save(self, path: Union[str, Path]) -> Path:
        """画像を指定パスに保存"""
        path = Path(path)
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "wb") as f:
            f.write(self.image_data)
        self.saved_path = path
        return path


# =============================================================================
# エクスポート
# =============================================================================

__all__ = [
    "CharacterConfig",
    "CharacterReferenceConfig",
    "VibeEncodeResult",
    "GenerateResult",
]
