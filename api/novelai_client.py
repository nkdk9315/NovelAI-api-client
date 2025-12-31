"""
NovelAI Unified Image Generation Client
統合されたVibe Transfer & Image2Image APIクライアント

使用方法:
- generate(): テキストからの画像生成、img2img、マルチキャラクター、Vibe Transferすべて対応
- encode_vibe(): 画像をVibeエンコード（オプションでファイル保存）
"""

import base64
import hashlib
import json
import os
import io
import zipfile
import random
from pathlib import Path
from datetime import datetime
from dataclasses import dataclass, field
from typing import Optional, Union, List, Literal

import httpx


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
        
        model_key_map = {
            "nai-diffusion-4-curated-preview": "v4curated",
            "nai-diffusion-4-full": "v4full",
            "nai-diffusion-4-5-curated": "v4-5curated",
            "nai-diffusion-4-5-full": "v4-5full",
        }
        model_key = model_key_map.get(self.model, "v4-5full")
        
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


class NovelAIClient:
    """NovelAI 統合画像生成クライアント"""
    
    API_URL = "https://image.novelai.net/ai/generate-image"
    STREAM_URL = "https://image.novelai.net/ai/generate-image-stream"
    ENCODE_URL = "https://image.novelai.net/ai/encode-vibe"
    SUBSCRIPTION_URL = "https://api.novelai.net/user/subscription"
    
    DEFAULT_NEGATIVE = (
        "nsfw, lowres, artistic error, film grain, scan artifacts, "
        "worst quality, bad quality, jpeg artifacts, very displeasing, "
        "chromatic aberration, dithering, halftone, screentone"
    )
    
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
            response = client.get(self.SUBSCRIPTION_URL, headers=headers)
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
    
    def _get_image_bytes(
        self,
        image: Union[str, Path, bytes],
    ) -> bytes:
        """画像データをバイトに変換"""
        if isinstance(image, bytes):
            return image
        elif isinstance(image, (str, Path)):
            path = Path(image)
            if path.exists():
                with open(path, "rb") as f:
                    return f.read()
            else:
                # Base64文字列として扱う
                return base64.b64decode(image)
        raise ValueError(f"Invalid image type: {type(image)}")
    
    def _get_image_base64(self, image: Union[str, Path, bytes]) -> str:
        """画像をBase64文字列に変換"""
        if isinstance(image, bytes):
            return base64.b64encode(image).decode('utf-8')
        elif isinstance(image, (str, Path)):
            path = Path(image)
            if path.exists():
                with open(path, "rb") as f:
                    return base64.b64encode(f.read()).decode('utf-8')
            else:
                # すでにBase64文字列
                return str(image)
        raise ValueError(f"Invalid image type: {type(image)}")
    
    def load_vibe_file(self, vibe_path: Union[str, Path]) -> dict:
        """
        .naiv4vibe ファイルを読み込む
        
        Args:
            vibe_path: .naiv4vibe ファイルのパス
            
        Returns:
            パースされたVibeデータの辞書
        """
        vibe_path = Path(vibe_path)
        with open(vibe_path, "r", encoding="utf-8") as f:
            return json.load(f)
    
    def _extract_encoding(
        self,
        vibe_data: dict,
        model: str = "nai-diffusion-4-5-full"
    ) -> tuple[str, float]:
        """Vibeデータからエンコード情報を抽出"""
        model_key_map = {
            "nai-diffusion-4-curated-preview": "v4curated",
            "nai-diffusion-4-full": "v4full",
            "nai-diffusion-4-5-curated": "v4-5curated",
            "nai-diffusion-4-5-full": "v4-5full",
        }
        model_key = model_key_map.get(model, "v4-5full")
        
        encodings = vibe_data.get("encodings", {})
        model_encodings = encodings.get(model_key, {})
        
        if not model_encodings:
            raise ValueError(f"No encoding found for model key: {model_key}")
        
        first_key = next(iter(model_encodings))
        encoding_data = model_encodings[first_key]
        
        encoding = encoding_data.get("encoding")
        params = encoding_data.get("params", {})
        information_extracted = params.get("information_extracted", 1.0)
        
        import_info = vibe_data.get("importInfo", {})
        if import_info:
            information_extracted = import_info.get("information_extracted", information_extracted)
        
        return encoding, information_extracted
    
    def _process_vibes(
        self,
        vibes: List[Union[str, Path, VibeEncodeResult]],
        model: str
    ) -> tuple[List[str], List[float]]:
        """Vibeリストをエンコードリストに変換"""
        encodings = []
        info_extracted_list = []
        
        for vibe in vibes:
            if isinstance(vibe, VibeEncodeResult):
                encodings.append(vibe.encoding)
                info_extracted_list.append(vibe.information_extracted)
            elif isinstance(vibe, (str, Path)):
                path = Path(vibe) if isinstance(vibe, str) else vibe
                if path.suffix == ".naiv4vibe" or (isinstance(vibe, str) and vibe.endswith(".naiv4vibe")):
                    # ファイルとして読み込み
                    data = self.load_vibe_file(path)
                    encoding, info_extracted = self._extract_encoding(data, model)
                    encodings.append(encoding)
                    info_extracted_list.append(info_extracted)
                else:
                    # エンコード済み文字列として使用
                    encodings.append(str(vibe))
                    info_extracted_list.append(1.0)
        
        return encodings, info_extracted_list
    
    def _prepare_character_reference_image(self, image_bytes: bytes) -> bytes:
        """
        キャラクター参照画像を適切なサイズに変換
        
        API仕様: 1024x1536 or 1536x1024 or 1472x1472 with black padding to fit
        """
        from PIL import Image
        import io
        
        img = Image.open(io.BytesIO(image_bytes))
        
        # RGBモードに変換（透過がある場合対応）
        if img.mode in ('RGBA', 'LA', 'P'):
            background = Image.new('RGB', img.size, (0, 0, 0))
            if img.mode == 'P':
                img = img.convert('RGBA')
            if img.mode in ('RGBA', 'LA'):
                background.paste(img, mask=img.split()[-1])
            img = background
        elif img.mode != 'RGB':
            img = img.convert('RGB')
        
        orig_width, orig_height = img.size
        aspect_ratio = orig_width / orig_height
        
        # 最適なターゲットサイズを選択
        # 縦長: 1024x1536 (aspect ~0.67)
        # 横長: 1536x1024 (aspect ~1.5)
        # 正方形に近い: 1472x1472 (aspect 1.0)
        if aspect_ratio < 0.8:
            # 縦長
            target_width, target_height = 1024, 1536
        elif aspect_ratio > 1.25:
            # 横長
            target_width, target_height = 1536, 1024
        else:
            # 正方形に近い
            target_width, target_height = 1472, 1472
        
        # 黒いキャンバスを作成
        result = Image.new('RGB', (target_width, target_height), (0, 0, 0))
        
        # 画像をアスペクト比を維持してリサイズ
        img_aspect = orig_width / orig_height
        target_aspect = target_width / target_height
        
        if img_aspect > target_aspect:
            # 画像が横長なので幅に合わせる
            new_width = target_width
            new_height = int(target_width / img_aspect)
        else:
            # 画像が縦長なので高さに合わせる
            new_height = target_height
            new_width = int(target_height * img_aspect)
        
        img_resized = img.resize((new_width, new_height), Image.Resampling.LANCZOS)
        
        # 中央に配置
        x_offset = (target_width - new_width) // 2
        y_offset = (target_height - new_height) // 2
        result.paste(img_resized, (x_offset, y_offset))
        
        # PNGとしてエクスポート
        output = io.BytesIO()
        result.save(output, format='PNG')
        return output.getvalue()
    
    def _process_character_references(
        self,
        refs: List[CharacterReferenceConfig],
    ) -> tuple[List[str], List[dict], List[float], List[float], List[float]]:
        """
        キャラクター参照リストをAPI用パラメータに変換
        
        API仕様: director_reference_images は
        1024x1536, 1536x1024, または 1472x1472 で黒いパディングが必要
        
        Returns:
            tuple containing:
            - images: [base64_image, ...] 直接Base64画像データのリスト
            - descriptions: [{"caption": {"base_caption": "character&style", ...}}, ...]
            - info_extracted: [1.0, ...]
            - strength_values: [1.0, ...] 常に1.0
            - secondary_strength_values: [1 - fidelity, ...] 
        """
        images = []
        descriptions = []
        info_extracted = []
        strength_values = []
        secondary_strength_values = []
        
        for ref in refs:
            # 画像を取得
            image_bytes = self._get_image_bytes(ref.image)
            
            # 正しいサイズにリサイズ・パディング
            processed_bytes = self._prepare_character_reference_image(image_bytes)
            b64_image = base64.b64encode(processed_bytes).decode('utf-8')
            
            # 直接Base64文字列をリストに追加
            images.append(b64_image)
            
            # 絵柄参照の設定
            ref_type = "character&style" if ref.include_style else "character"
            descriptions.append({
                "caption": {"base_caption": ref_type, "char_captions": []},
                "legacy_uc": False
            })
            
            # information_extractedはAPI仕様により常に1.0固定
            info_extracted.append(1.0)
            # 公式と同じ: strength_values = 1, secondary = 1 - fidelity
            strength_values.append(1.0)
            secondary_strength_values.append(1.0 - ref.fidelity)
        
        return images, descriptions, info_extracted, strength_values, secondary_strength_values
    
    def encode_vibe(
        self,
        image: Union[str, Path, bytes],
        *,
        model: str = "nai-diffusion-4-5-full",
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
        # 画像データ取得
        image_bytes = self._get_image_bytes(image)
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
            "information_extracted": information_extracted,
            "model": model
        }
        
        # エンコード前のアンラス残高を取得
        anlas_before = None
        try:
            balance = self.get_anlas_balance()
            anlas_before = balance["total"]
        except Exception:
            pass  # アンラス取得失敗は無視
        
        with httpx.Client(timeout=120.0) as client:
            response = client.post(self.ENCODE_URL, json=payload, headers=headers)
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
            model=model,
            information_extracted=information_extracted,
            strength=strength,
            source_image_hash=source_hash,
            created_at=datetime.now(),
            anlas_remaining=anlas_remaining,
            anlas_consumed=anlas_consumed,
        )
        
        # 保存処理
        if save_path:
            result.save(save_path)
        elif save_dir:
            save_dir = Path(save_dir)
            save_dir.mkdir(parents=True, exist_ok=True)
            filename = f"{source_hash[:12]}_{datetime.now().strftime('%Y%m%d_%H%M%S')}.naiv4vibe"
            result.save(save_dir / filename)
        
        return result
    
    def generate(
        self,
        prompt: str,
        *,
        # === Action & Image2Image ===
        action: Literal["generate", "img2img"] = "generate",
        source_image: Optional[Union[str, Path, bytes]] = None,
        img2img_strength: float = 0.62,
        img2img_noise: float = 0.0,
        
        # === キャラクター設定 ===
        characters: Optional[List[CharacterConfig]] = None,
        
        # === Vibe Transfer ===
        vibes: Optional[List[Union[str, Path, VibeEncodeResult]]] = None,
        vibe_strengths: Optional[List[float]] = None,
        vibe_info_extracted: Optional[List[float]] = None,
        
        # === Character Reference (Director Reference) ===
        character_reference: Optional[CharacterReferenceConfig] = None,
        
        # === プロンプト ===
        negative_prompt: Optional[str] = None,
        
        # === 出力オプション ===
        save_path: Optional[Union[str, Path]] = None,
        save_dir: Optional[Union[str, Path]] = None,
        
        # === 生成パラメータ ===
        model: str = "nai-diffusion-4-5-full",
        width: int = 832,
        height: int = 1216,
        steps: int = 23,
        scale: float = 5.0,
        seed: Optional[int] = None,
        sampler: str = "k_euler_ancestral",
        noise_schedule: str = "karras",
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
        if negative_prompt is None:
            negative_prompt = self.DEFAULT_NEGATIVE
        
        if seed is None:
            seed = random.randint(0, 2**32 - 1)
        
        # img2imgの場合、source_imageが必須
        if action == "img2img" and source_image is None:
            raise ValueError("source_image is required for img2img action")
        
        # vibesとcharacter_referenceは併用不可
        if vibes and character_reference:
            raise ValueError(
                "Cannot use vibes and character_reference together. "
                "Character reference does not support Vibe Transfer."
            )
        
        # Character Referenceの処理（単一オブジェクトをリストに変換して処理）
        char_ref_data = None
        if character_reference:
            char_ref_data = self._process_character_references([character_reference])
        
        # Vibeの処理
        vibe_encodings = []
        vibe_info_list = []
        if vibes:
            vibe_encodings, vibe_info_list = self._process_vibes(vibes, model)
            
            n_vibes = len(vibe_encodings)
            if vibe_strengths is None:
                vibe_strengths = [0.7] * n_vibes
            if vibe_info_extracted is not None:
                vibe_info_list = list(vibe_info_extracted)
        
        # キャラクタープロンプト構築
        char_captions = []
        char_negative_captions = []
        if characters:
            char_captions = [char.to_caption_dict() for char in characters]
            char_negative_captions = [char.to_negative_caption_dict() for char in characters]
        
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
                "seed": seed,
                "negative_prompt": negative_prompt,
                "deliberate_euler_ancestral_bug": False,
                "prefer_brownian": True,
            },
            "use_new_shared_trial": True
        }
        
        # img2imgパラメータ
        if action == "img2img":
            payload["parameters"]["image"] = self._get_image_base64(source_image)
            payload["parameters"]["strength"] = img2img_strength
            payload["parameters"]["noise"] = img2img_noise
            payload["parameters"]["extra_noise_seed"] = seed - 1
        
        # Vibeパラメータ
        if vibe_encodings:
            payload["parameters"]["reference_image_multiple"] = vibe_encodings
            payload["parameters"]["reference_strength_multiple"] = vibe_strengths
            payload["parameters"]["reference_information_extracted_multiple"] = vibe_info_list
            payload["parameters"]["normalize_reference_strength_multiple"] = True
        
        # Character Reference (Director Reference) パラメータ
        if char_ref_data:
            images, descriptions, info_extracted, strength_vals, secondary_strength = char_ref_data
            # Vibe Transferと同様に、director_reference_images で直接Base64リストを渡す
            payload["parameters"]["director_reference_images"] = images
            payload["parameters"]["director_reference_descriptions"] = descriptions
            payload["parameters"]["director_reference_information_extracted"] = info_extracted
            payload["parameters"]["director_reference_strength_values"] = strength_vals
            payload["parameters"]["director_reference_secondary_strength_values"] = secondary_strength
            payload["parameters"]["use_coords"] = True
            # ストリームエンドポイント用の必須パラメータ
            payload["parameters"]["stream"] = "msgpack"
            payload["parameters"]["image_format"] = "png"
            
            # characterPromptsも必要（キャラクターが指定されていない場合はデフォルトを作成）
            if not characters:
                # デフォルトのキャラクター設定を作成
                characters = [CharacterConfig(prompt=prompt, center_x=0.5, center_y=0.5)]
                char_captions = [char.to_caption_dict() for char in characters]
                char_negative_captions = [char.to_negative_caption_dict() for char in characters]
        
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
                "base_caption": negative_prompt,
                "char_captions": char_negative_captions
            },
            "legacy_uc": False
        }
        
        # use_coordsはキャラクターがある場合のみTrue
        if characters:
            payload["parameters"]["use_coords"] = True
            # characterPromptsを生成（キャラクターごとにprompt, uc, center, enabled）
            character_prompts = []
            for char in characters:
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
            pass  # アンラス取得失敗は無視
        
        # APIリクエスト
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json",
        }
        
        # キャラクター参照はストリームエンドポイントを使用
        use_stream = character_reference is not None
        api_url = self.STREAM_URL if use_stream else self.API_URL
        
        with httpx.Client(timeout=120.0) as client:
            response = client.post(api_url, json=payload, headers=headers)
            if response.status_code != 200:
                print(f"Error response: {response.text}")
            response.raise_for_status()
            
            if use_stream:
                # ストリームレスポンス（msgpack形式）を処理
                # msgpackのイベントストリームから画像データを抽出
                import msgpack
                
                content = response.content
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
                        # 拡張タイプを無視するハンドラ
                        def ext_hook(code, data):
                            return data  # rawデータをそのまま返す
                        
                        unpacker = msgpack.Unpacker(
                            raw=False, 
                            strict_map_key=False,
                            ext_hook=ext_hook
                        )
                        unpacker.feed(content)
                        
                        for event in unpacker:
                            if isinstance(event, dict):
                                # 最終的な画像データを探す
                                if 'data' in event:
                                    image_data = event['data']
                                elif 'image' in event:
                                    image_data = event['image']
                    except Exception as e:
                        # msgpackパース失敗時、バイナリデータを直接探す
                        # PNGマジックバイトを探す
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
            else:
                # 通常のZIPレスポンス
                with zipfile.ZipFile(io.BytesIO(response.content)) as zf:
                    for name in zf.namelist():
                        if name.endswith(('.png', '.webp', '.jpg', '.jpeg')):
                            image_data = zf.read(name)
                            break
                    else:
                        raise ValueError("No image found in response")
        
        # 生成後のアンラス残高を取得し、消費量を計算
        anlas_remaining = None
        anlas_consumed = None
        try:
            balance = self.get_anlas_balance()
            anlas_remaining = balance["total"]
            if anlas_before is not None:
                anlas_consumed = anlas_before - anlas_remaining
        except Exception:
            pass  # アンラス取得失敗は無視
        
        result = GenerateResult(
            image_data=image_data,
            seed=seed,
            anlas_remaining=anlas_remaining,
            anlas_consumed=anlas_consumed,
        )
        
        # 保存処理
        if save_path:
            result.save(save_path)
        elif save_dir:
            save_dir = Path(save_dir)
            save_dir.mkdir(parents=True, exist_ok=True)
            prefix = "img2img" if action == "img2img" else "gen"
            if characters:
                prefix += "_multi"
            filename = f"{prefix}_{datetime.now().strftime('%Y%m%d_%H%M%S')}_{seed}.png"
            result.save(save_dir / filename)
        
        return result


def main():
    """使用例"""
    from dotenv import load_dotenv
    load_dotenv()
    
    client = NovelAIClient()
    
    # 例1: シンプルな生成
    # result = client.generate(
    #     "1girl, beautiful anime girl, detailed eyes, masterpiece",
    #     save_dir="output/"
    # )
    # print(f"Generated: {result.saved_path}")
    
    # 例2: Vibeを使用した生成
    # result = client.generate(
    #     "1girl, beautiful anime girl",
    #     vibes=["style.naiv4vibe"],
    #     vibe_strengths=[0.7],
    #     save_dir="output/"
    # )
    
    # 例3: img2img
    # result = client.generate(
    #     "1girl, beautiful anime girl",
    #     action="img2img",
    #     source_image="input.png",
    #     img2img_strength=0.6,
    #     save_dir="output/"
    # )
    
    # 例4: マルチキャラクター + Vibe + img2img
    # characters = [
    #     CharacterConfig("1girl, blonde hair", center_x=0.3, center_y=0.5),
    #     CharacterConfig("1boy, black hair", center_x=0.7, center_y=0.5),
    # ]
    # result = client.generate(
    #     "school classroom, wide shot",
    #     action="img2img",
    #     source_image="input.png",
    #     characters=characters,
    #     vibes=["style.naiv4vibe"],
    #     save_dir="output/"
    # )
    
    print("NovelAIClient ready!")


if __name__ == "__main__":
    main()
