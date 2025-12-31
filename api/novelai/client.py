"""
NovelAI Unified Image Generation Client
統合されたVibe Transfer & Image2Image APIクライアント

Pydanticモデルによるバリデーション付き

使用方法:
- generate(): テキストからの画像生成、img2img、マルチキャラクター、Vibe Transferすべて対応
- encode_vibe(): 画像をVibeエンコード（オプションでファイル保存）
"""

import base64
import hashlib
import io
import logging
import os
import random
import zipfile
from datetime import datetime
from pathlib import Path
from typing import Optional, Union, List, Literal

import httpx

from .constants import (
    API_URL,
    STREAM_URL,
    ENCODE_URL,
    SUBSCRIPTION_URL,
    DEFAULT_NEGATIVE,
    DEFAULT_MODEL,
    DEFAULT_WIDTH,
    DEFAULT_HEIGHT,
    DEFAULT_STEPS,
    DEFAULT_SCALE,
    DEFAULT_SAMPLER,
    DEFAULT_NOISE_SCHEDULE,
    DEFAULT_VIBE_STRENGTH,
    DEFAULT_IMG2IMG_STRENGTH,
)
from .models import (
    CharacterConfigModel,
    CharacterReferenceConfigModel,
    VibeEncodeResultModel,
    GenerateResultModel,
    GenerateParamsModel,
    EncodeVibeParamsModel,
)
from .dataclasses import (
    CharacterConfig,
    CharacterReferenceConfig,
    VibeEncodeResult,
    GenerateResult,
)
from .utils import (
    get_image_bytes,
    get_image_base64,
    load_vibe_file,
    extract_encoding,
    process_vibes,
    process_character_references,
)

logger = logging.getLogger(__name__)


class NovelAIClient:
    """NovelAI 統合画像生成クライアント（Pydanticバリデーション付き）"""

    def __init__(self, api_key: Optional[str] = None):
        """
        Args:
            api_key: NovelAI API Key。指定しない場合は環境変数 NOVELAI_API_KEY から取得
        """
        self.api_key = api_key or os.environ.get("NOVELAI_API_KEY")
        if not self.api_key:
            raise ValueError(
                "API key is required. Set NOVELAI_API_KEY environment variable "
                "or pass api_key parameter."
            )

    def get_anlas_balance(self) -> dict:
        """
        残りアンラス（Training Steps）を取得

        Returns:
            dict: アンラス情報
                - fixed: サブスクリプション付随のアンラス残量
                - purchased: 追加購入したアンラス残量
                - total: 合計アンラス残量
                - tier: サブスクリプションプラン (0:なし, 1:Tablet, 2:Scroll, 3:Opus)
        """
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Accept": "application/json",
        }

        with httpx.Client(timeout=30.0) as client:
            response = client.get(SUBSCRIPTION_URL, headers=headers)
            response.raise_for_status()
            data = response.json()

        training_steps = data.get("trainingStepsLeft", {})
        fixed = training_steps.get("fixedTrainingStepsLeft", 0)
        purchased = training_steps.get("purchasedTrainingSteps", 0)

        return {
            "fixed": fixed,
            "purchased": purchased,
            "total": fixed + purchased,
            "tier": data.get("tier", 0),
        }

    def encode_vibe(
        self,
        image: Union[str, Path, bytes],
        *,
        model: str = DEFAULT_MODEL,
        information_extracted: float = 0.7,
        strength: float = 0.7,
        save_path: Optional[Union[str, Path]] = None,
        save_dir: Optional[Union[str, Path]] = None,
    ) -> VibeEncodeResult:
        """
        画像をVibe Transfer用にエンコード（2 Anlas消費）

        Args:
            image: 画像ファイルパス、バイトデータ、またはBase64文字列
            model: 使用するモデル名
            information_extracted: 抽出する情報量 (0.0-1.0)
            strength: Vibe Transfer の推奨強度
            save_path: 保存先ファイルパス（指定時のみ保存）
            save_dir: 保存先ディレクトリ（自動ファイル名で保存）

        Returns:
            VibeEncodeResult: エンコード結果（DB保存対応）
        """
        # パラメータをバリデーション
        params = EncodeVibeParamsModel(
            image=image,
            model=model,
            information_extracted=information_extracted,
            strength=strength,
            save_path=save_path,
            save_dir=save_dir,
        )

        # 画像データ取得
        image_bytes = get_image_bytes(params.image)
        b64_image = base64.b64encode(image_bytes).decode('utf-8')

        # ハッシュ計算
        source_hash = hashlib.sha256(image_bytes).hexdigest()

        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json",
            "Accept": "*/*"
        }

        payload = {
            "image": b64_image,
            "information_extracted": params.information_extracted,
            "model": params.model
        }

        # エンコード前のアンラス残高を取得
        anlas_before = None
        try:
            balance = self.get_anlas_balance()
            anlas_before = balance["total"]
        except Exception:
            pass  # アンラス取得失敗は無視

        with httpx.Client(timeout=120.0) as client:
            response = client.post(ENCODE_URL, json=payload, headers=headers)
            response.raise_for_status()
            encoding = base64.b64encode(response.content).decode('utf-8')

        # エンコード後のアンラス残高を取得し、消費量を計算
        anlas_remaining = None
        anlas_consumed = None
        try:
            balance = self.get_anlas_balance()
            anlas_remaining = balance["total"]
            if anlas_before is not None:
                anlas_consumed = anlas_before - anlas_remaining
        except Exception:
            pass  # アンラス取得失敗は無視

        result = VibeEncodeResult(
            encoding=encoding,
            model=params.model,
            information_extracted=params.information_extracted,
            strength=params.strength,
            source_image_hash=source_hash,
            created_at=datetime.now(),
            anlas_remaining=anlas_remaining,
            anlas_consumed=anlas_consumed,
        )

        # 保存処理
        if params.save_path:
            result.save(params.save_path)
        elif params.save_dir:
            save_dir_path = Path(params.save_dir)
            save_dir_path.mkdir(parents=True, exist_ok=True)
            filename = f"{source_hash[:12]}_{datetime.now().strftime('%Y%m%d_%H%M%S')}.naiv4vibe"
            result.save(save_dir_path / filename)

        return result

    def generate(
        self,
        prompt: str,
        *,
        # === Action & Image2Image ===
        action: Literal["generate", "img2img"] = "generate",
        source_image: Optional[Union[str, Path, bytes]] = None,
        img2img_strength: float = DEFAULT_IMG2IMG_STRENGTH,
        img2img_noise: float = 0.0,

        # === キャラクター設定 ===
        characters: Optional[List[Union[CharacterConfig, CharacterConfigModel]]] = None,

        # === Vibe Transfer ===
        vibes: Optional[List[Union[str, Path, VibeEncodeResult]]] = None,
        vibe_strengths: Optional[List[float]] = None,
        vibe_info_extracted: Optional[List[float]] = None,

        # === Character Reference (Director Reference) ===
        character_reference: Optional[Union[CharacterReferenceConfig, CharacterReferenceConfigModel]] = None,

        # === プロンプト ===
        negative_prompt: Optional[str] = None,

        # === 出力オプション ===
        save_path: Optional[Union[str, Path]] = None,
        save_dir: Optional[Union[str, Path]] = None,

        # === 生成パラメータ ===
        model: str = DEFAULT_MODEL,
        width: int = DEFAULT_WIDTH,
        height: int = DEFAULT_HEIGHT,
        steps: int = DEFAULT_STEPS,
        scale: float = DEFAULT_SCALE,
        seed: Optional[int] = None,
        sampler: str = DEFAULT_SAMPLER,
        noise_schedule: str = DEFAULT_NOISE_SCHEDULE,
    ) -> GenerateResult:
        """
        統合画像生成メソッド

        Args:
            prompt: 生成プロンプト（キャラクターなしの場合はメインプロンプト、
                   キャラクターありの場合は背景・シーンプロンプト）
            action: "generate"（テキストから生成）or "img2img"（画像から生成）
            source_image: img2img用の入力画像（パス、バイト、Base64）
            img2img_strength: img2img変換強度 (0.0-1.0)
            img2img_noise: img2imgノイズ量 (0.0-1.0)
            characters: キャラクター設定のリスト（位置・個別プロンプト）
            vibes: Vibeリスト（.naiv4vibeパス、エンコード文字列、VibeEncodeResult）
            vibe_strengths: 各Vibeの強度（デフォルト0.7）
            vibe_info_extracted: 各Vibeのinformation_extracted値
            character_reference: キャラクター参照設定（Vibeと併用不可）
            negative_prompt: ネガティブプロンプト
            save_path: 保存先ファイルパス
            save_dir: 保存先ディレクトリ（自動ファイル名）
            model: モデル名
            width: 画像幅
            height: 画像高さ
            steps: 生成ステップ数
            scale: CFG Scale
            seed: シード値（Noneでランダム）
            sampler: サンプラー
            noise_schedule: ノイズスケジュール

        Returns:
            GenerateResult: 生成結果
        """
        # パラメータをバリデーション
        params = GenerateParamsModel(
            prompt=prompt,
            action=action,
            source_image=source_image,
            img2img_strength=img2img_strength,
            img2img_noise=img2img_noise,
            vibes=vibes,
            vibe_strengths=vibe_strengths,
            vibe_info_extracted=vibe_info_extracted,
            negative_prompt=negative_prompt,
            save_path=save_path,
            save_dir=save_dir,
            model=model,
            width=width,
            height=height,
            steps=steps,
            scale=scale,
            seed=seed,
            sampler=sampler,
            noise_schedule=noise_schedule,
        )

        # デフォルト値の設定
        if params.negative_prompt is None:
            negative_prompt_value = DEFAULT_NEGATIVE
        else:
            negative_prompt_value = params.negative_prompt

        if params.seed is None:
            seed_value = random.randint(0, 2**32 - 1)
        else:
            seed_value = params.seed

        # Character Referenceの変換（PydanticモデルからDataclassへ）
        char_ref_config = None
        if character_reference:
            if isinstance(character_reference, CharacterReferenceConfigModel):
                char_ref_config = CharacterReferenceConfig(
                    image=character_reference.image,
                    fidelity=character_reference.fidelity,
                    include_style=character_reference.include_style,
                )
            else:
                char_ref_config = character_reference

        # Charactersの変換（PydanticモデルからDataclassへ）
        char_configs: List[CharacterConfig] = []
        if characters:
            for char in characters:
                if isinstance(char, CharacterConfigModel):
                    char_configs.append(CharacterConfig(
                        prompt=char.prompt,
                        center_x=char.center_x,
                        center_y=char.center_y,
                        negative_prompt=char.negative_prompt,
                    ))
                else:
                    char_configs.append(char)

        # Character Referenceの処理
        char_ref_data = None
        if char_ref_config:
            char_ref_data = process_character_references([char_ref_config])

        # Vibeの処理
        vibe_encodings = []
        vibe_info_list = []
        if vibes:
            vibe_encodings, vibe_info_list = process_vibes(vibes, model)

            n_vibes = len(vibe_encodings)
            if vibe_strengths is None:
                vibe_strengths = [DEFAULT_VIBE_STRENGTH] * n_vibes
            if vibe_info_extracted is not None:
                vibe_info_list = list(vibe_info_extracted)

        # キャラクタープロンプト構築
        char_captions = []
        char_negative_captions = []
        if char_configs:
            char_captions = [char.to_caption_dict() for char in char_configs]
            char_negative_captions = [char.to_negative_caption_dict() for char in char_configs]

        # ペイロード構築
        payload = {
            "input": prompt,
            "model": model,
            "action": action,
            "parameters": {
                "params_version": 3,
                "width": width,
                "height": height,
                "scale": scale,
                "sampler": sampler,
                "steps": steps,
                "n_samples": 1,
                "ucPreset": 0,
                "qualityToggle": True,
                "autoSmea": False,
                "dynamic_thresholding": False,
                "controlnet_strength": 1,
                "legacy": False,
                "add_original_image": True,
                "cfg_rescale": 0,
                "noise_schedule": noise_schedule,
                "legacy_v3_extend": False,
                "skip_cfg_above_sigma": None,
                "use_coords": True,
                "legacy_uc": False,
                "normalize_reference_strength_multiple": True,
                "inpaintImg2ImgStrength": 1,
                "seed": seed_value,
                "negative_prompt": negative_prompt_value,
                "deliberate_euler_ancestral_bug": False,
                "prefer_brownian": True,
            },
            "use_new_shared_trial": True
        }

        # img2imgパラメータ
        if action == "img2img":
            payload["parameters"]["image"] = get_image_base64(source_image)
            payload["parameters"]["strength"] = img2img_strength
            payload["parameters"]["noise"] = img2img_noise
            payload["parameters"]["extra_noise_seed"] = seed_value - 1

        # Vibeパラメータ
        if vibe_encodings:
            payload["parameters"]["reference_image_multiple"] = vibe_encodings
            payload["parameters"]["reference_strength_multiple"] = vibe_strengths
            payload["parameters"]["reference_information_extracted_multiple"] = vibe_info_list
            payload["parameters"]["normalize_reference_strength_multiple"] = True

        # Character Reference (Director Reference) パラメータ
        if char_ref_data:
            images, descriptions, info_extracted, strength_vals, secondary_strength = char_ref_data
            payload["parameters"]["director_reference_images"] = images
            payload["parameters"]["director_reference_descriptions"] = descriptions
            payload["parameters"]["director_reference_information_extracted"] = info_extracted
            payload["parameters"]["director_reference_strength_values"] = strength_vals
            payload["parameters"]["director_reference_secondary_strength_values"] = secondary_strength
            payload["parameters"]["use_coords"] = True
            payload["parameters"]["stream"] = "msgpack"
            payload["parameters"]["image_format"] = "png"

            # characterPromptsも必要
            if not char_configs:
                char_configs = [CharacterConfig(prompt=prompt, center_x=0.5, center_y=0.5)]
                char_captions = [char.to_caption_dict() for char in char_configs]
                char_negative_captions = [char.to_negative_caption_dict() for char in char_configs]

        # V4プロンプト構造
        payload["parameters"]["v4_prompt"] = {
            "caption": {
                "base_caption": prompt,
                "char_captions": char_captions
            },
            "use_coords": True,
            "use_order": True
        }
        payload["parameters"]["v4_negative_prompt"] = {
            "caption": {
                "base_caption": negative_prompt_value,
                "char_captions": char_negative_captions
            },
            "legacy_uc": False
        }

        # use_coordsはキャラクターがある場合のみTrue
        if char_configs:
            payload["parameters"]["use_coords"] = True
            character_prompts = []
            for char in char_configs:
                character_prompts.append({
                    "prompt": char.prompt,
                    "uc": char.negative_prompt,
                    "center": {"x": char.center_x, "y": char.center_y},
                    "enabled": True
                })
            payload["parameters"]["characterPrompts"] = character_prompts

        # 生成前のアンラス残高を取得
        anlas_before = None
        try:
            balance = self.get_anlas_balance()
            anlas_before = balance["total"]
        except Exception:
            pass

        # APIリクエスト
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json",
        }

        use_stream = char_ref_config is not None
        api_url = STREAM_URL if use_stream else API_URL

        with httpx.Client(timeout=120.0) as client:
            response = client.post(api_url, json=payload, headers=headers)
            if response.status_code != 200:
                logger.error(f"Error response: {response.text}")
            response.raise_for_status()

            if use_stream:
                image_data = self._parse_stream_response(response.content)
            else:
                image_data = self._parse_zip_response(response.content)

        # 生成後のアンラス残高を取得
        anlas_remaining = None
        anlas_consumed = None
        try:
            balance = self.get_anlas_balance()
            anlas_remaining = balance["total"]
            if anlas_before is not None:
                anlas_consumed = anlas_before - anlas_remaining
        except Exception:
            pass

        result = GenerateResult(
            image_data=image_data,
            seed=seed_value,
            anlas_remaining=anlas_remaining,
            anlas_consumed=anlas_consumed,
        )

        # 保存処理
        if save_path:
            result.save(save_path)
        elif save_dir:
            save_dir_path = Path(save_dir)
            save_dir_path.mkdir(parents=True, exist_ok=True)
            prefix = "img2img" if action == "img2img" else "gen"
            if char_configs:
                prefix += "_multi"
            filename = f"{prefix}_{datetime.now().strftime('%Y%m%d_%H%M%S')}_{seed_value}.png"
            result.save(save_dir_path / filename)

        return result

    def _parse_stream_response(self, content: bytes) -> bytes:
        """ストリームレスポンスから画像データを抽出"""
        image_data = None

        # まずZIPファイルかどうかチェック
        if content[:2] == b'PK':
            with zipfile.ZipFile(io.BytesIO(content)) as zf:
                for name in zf.namelist():
                    if name.endswith(('.png', '.webp', '.jpg', '.jpeg')):
                        image_data = zf.read(name)
                        break
        # 直接PNGデータかチェック
        elif content[:8] == b'\x89PNG\r\n\x1a\n':
            image_data = content
        else:
            # msgpackとしてパースを試みる
            try:
                import msgpack

                def ext_hook(code, data):
                    return data

                unpacker = msgpack.Unpacker(
                    raw=False,
                    strict_map_key=False,
                    ext_hook=ext_hook
                )
                unpacker.feed(content)

                for event in unpacker:
                    if isinstance(event, dict):
                        if 'data' in event:
                            image_data = event['data']
                        elif 'image' in event:
                            image_data = event['image']
            except Exception as e:
                # msgpackパース失敗時、バイナリデータを直接探す
                png_magic = b'\x89PNG\r\n\x1a\n'
                png_start = content.find(png_magic)
                if png_start >= 0:
                    image_data = content[png_start:]
                else:
                    raise ValueError(f"Cannot parse msgpack response: {e}")

        if image_data is None:
            raise ValueError(f"No image found in stream response (length: {len(content)})")

        # image_dataがbytesでない場合（base64の場合）
        if isinstance(image_data, str):
            image_data = base64.b64decode(image_data)

        return image_data

    def _parse_zip_response(self, content: bytes) -> bytes:
        """ZIPレスポンスから画像データを抽出"""
        with zipfile.ZipFile(io.BytesIO(content)) as zf:
            for name in zf.namelist():
                if name.endswith(('.png', '.webp', '.jpg', '.jpeg')):
                    return zf.read(name)
        raise ValueError("No image found in response")


# =============================================================================
# エクスポート
# =============================================================================

__all__ = [
    "NovelAIClient",
]
