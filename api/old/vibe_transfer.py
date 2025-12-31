"""
NovelAI Vibe Transfer API Client
.naiv4vibe ファイル、エンコード済みデータ、生画像のエンコードに対応したVibe Transfer画像生成

使用方法:
1. .naiv4vibe ファイル: generate() に vibe_path を指定（Anlas消費なし）
2. エンコード済み文字列: generate_with_encoded() を使用（Anlas消費なし）
3. 生画像をエンコード: encode_vibe() でエンコード後に使用（エンコード時に2 Anlas消費）
"""

import base64
import json
import os
import io
import zipfile
from pathlib import Path
from datetime import datetime
from typing import Optional, Union, List

import httpx
from dataclasses import dataclass, field


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


class VibeTransferClient:
    """NovelAI Vibe Transfer APIクライアント"""
    
    API_URL = "https://image.novelai.net/ai/generate-image"
    ENCODE_URL = "https://image.novelai.net/ai/encode-vibe"
    
    def __init__(self, api_key: Optional[str] = None):
        """
        Args:
            api_key: NovelAI API Key。指定しない場合は環境変数 NOVELAI_API_KEY から取得
        """
        self.api_key = api_key or os.environ.get("NOVELAI_API_KEY")
        if not self.api_key:
            raise ValueError("API key is required. Set NOVELAI_API_KEY environment variable or pass api_key parameter.")
    
    def load_vibe_file(self, vibe_path: str | Path) -> dict:
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
    
    def extract_encoding(self, vibe_data: dict, model: str = "nai-diffusion-4-5-full") -> tuple[str, float, Optional[str]]:
        """
        Vibeデータからエンコード情報を抽出
        
        Args:
            vibe_data: .naiv4vibeファイルからロードしたデータ
            model: 使用するモデル名
            
        Returns:
            (encoding, information_extracted, mask) のタプル
        """
        # モデル名からエンコーディングキーを取得
        model_key_map = {
            "nai-diffusion-4-curated-preview": "v4curated",
            "nai-diffusion-4-full": "v4full",
            "nai-diffusion-4-5-curated": "v4-5curated",
            "nai-diffusion-4-5-full": "v4-5full",
        }
        model_key = model_key_map.get(model, "v4-5full")
        
        # エンコーディングを取得
        encodings = vibe_data.get("encodings", {})
        model_encodings = encodings.get(model_key, {})
        
        if not model_encodings:
            raise ValueError(f"No encoding found for model key: {model_key}")
        
        # 最初のエンコーディングを使用
        first_key = next(iter(model_encodings))
        encoding_data = model_encodings[first_key]
        
        encoding = encoding_data.get("encoding")
        params = encoding_data.get("params", {})
        information_extracted = params.get("information_extracted", 1.0)
        mask = params.get("mask")
        
        # importInfoからも取得を試みる
        import_info = vibe_data.get("importInfo", {})
        if import_info:
            information_extracted = import_info.get("information_extracted", information_extracted)
            mask = import_info.get("mask", mask)
        
        return encoding, information_extracted, mask
    
    def encode_vibe(
        self,
        image_path: Optional[str | Path] = None,
        image_data: Optional[bytes] = None,
        image_base64: Optional[str] = None,
        model: str = "nai-diffusion-4-5-full",
        information_extracted: float = 0.7,
    ) -> str:
        """
        生画像をVibe Transfer用にエンコード（2 Anlas消費）
        
        Args:
            image_path: 画像ファイルのパス
            image_data: 画像のバイトデータ
            image_base64: Base64エンコード済み画像データ
            model: 使用するモデル名
            information_extracted: 抽出する情報量 (0.0-1.0)
            
        Returns:
            エンコードされたVibeデータ（文字列）
            
        Note:
            image_path, image_data, image_base64 のいずれか1つを指定してください
        """
        # 画像データの取得
        if image_base64:
            b64_image = image_base64
        elif image_data:
            b64_image = base64.b64encode(image_data).decode('utf-8')
        elif image_path:
            with open(image_path, 'rb') as f:
                b64_image = base64.b64encode(f.read()).decode('utf-8')
        else:
            raise ValueError("image_path, image_data, or image_base64 must be provided")
        
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
        
        with httpx.Client(timeout=120.0) as client:
            response = client.post(self.ENCODE_URL, json=payload, headers=headers)
            response.raise_for_status()
            # レスポンスはバイナリデータなのでBase64エンコードする
            return base64.b64encode(response.content).decode('utf-8')
    
    def encode_vibe_and_save(
        self,
        image_path: str | Path,
        output_path: Optional[str | Path] = None,
        model: str = "nai-diffusion-4-5-full",
        information_extracted: float = 0.7,
        strength: float = 0.7,
    ) -> Path:
        """
        画像をエンコードして.naiv4vibe形式で保存（2 Anlas消費）
        
        Args:
            image_path: エンコードする画像ファイルのパス
            output_path: 出力ファイルパス（Noneの場合は自動生成）
            model: 使用するモデル名
            information_extracted: 抽出する情報量 (0.0-1.0)
            strength: Vibe Transfer の推奨強度
            
        Returns:
            保存されたファイルのパス
        """
        import hashlib
        
        # 画像をエンコード
        encoding = self.encode_vibe(
            image_path=image_path,
            model=model,
            information_extracted=information_extracted
        )
        
        # IDを生成（エンコーディングのハッシュ）
        vibe_id = hashlib.sha256(encoding.encode()).hexdigest()
        
        # モデルキーを取得
        model_key_map = {
            "nai-diffusion-4-curated-preview": "v4curated",
            "nai-diffusion-4-full": "v4full",
            "nai-diffusion-4-5-curated": "v4-5curated",
            "nai-diffusion-4-5-full": "v4-5full",
        }
        model_key = model_key_map.get(model, "v4-5full")
        
        # .naiv4vibe形式のデータを構築
        vibe_data = {
            "identifier": "novelai-vibe-transfer",
            "version": 1,
            "type": "encoding",
            "id": vibe_id,
            "encodings": {
                model_key: {
                    "unknown": {
                        "encoding": encoding,
                        "params": {
                            "information_extracted": information_extracted
                        }
                    }
                }
            },
            "name": f"{vibe_id[:6]}-{vibe_id[-6:]}",
            "createdAt": datetime.now().isoformat(),
            "importInfo": {
                "model": model,
                "information_extracted": information_extracted,
                "strength": strength
            }
        }
        
        # 出力パスの設定
        if output_path is None:
            output_path = Path(image_path).with_suffix(".naiv4vibe")
        else:
            output_path = Path(output_path)
        
        # ファイルに保存
        with open(output_path, "w", encoding="utf-8") as f:
            json.dump(vibe_data, f, indent=2)
        
        return output_path
    
    def generate_with_encoded(
        self,
        prompt: str,
        encoded_vibes: Union[str, List[str]],
        strengths: Optional[Union[float, List[float]]] = None,
        information_extracted_values: Optional[Union[float, List[float]]] = None,
        negative_prompt: str = "nsfw, lowres, artistic error, worst quality, bad quality, jpeg artifacts",
        model: str = "nai-diffusion-4-5-full",
        width: int = 832,
        height: int = 1216,
        steps: int = 23,
        scale: float = 5.0,
        seed: Optional[int] = None,
    ) -> bytes:
        """
        エンコード済みVibeデータを使用して画像を生成（Anlas消費なし）
        
        Args:
            prompt: 生成プロンプト
            encoded_vibes: エンコード済みVibeデータ（文字列または文字列リスト）
            strengths: 各Vibeの強度（単一値またはリスト、デフォルト0.7）
            information_extracted_values: 各Vibeのinformation_extracted値（デフォルト1.0）
            negative_prompt: ネガティブプロンプト
            model: モデル名
            width: 画像幅
            height: 画像高さ
            steps: 生成ステップ数
            scale: CFG Scale
            seed: シード値（Noneの場合はランダム）
            
        Returns:
            生成された画像のバイトデータ (PNG)
        """
        import random
        
        # リストに正規化
        if isinstance(encoded_vibes, str):
            encoded_vibes = [encoded_vibes]
        
        n_vibes = len(encoded_vibes)
        
        # 強度の正規化
        if strengths is None:
            strengths = [0.7] * n_vibes
        elif isinstance(strengths, (int, float)):
            strengths = [float(strengths)] * n_vibes
        
        # information_extractedの正規化
        if information_extracted_values is None:
            information_extracted_values = [1.0] * n_vibes
        elif isinstance(information_extracted_values, (int, float)):
            information_extracted_values = [float(information_extracted_values)] * n_vibes
        
        # シード生成
        if seed is None:
            seed = random.randint(0, 2**32 - 1)
        
        # リクエストペイロード構築
        payload = {
            "input": prompt,
            "model": model,
            "action": "generate",
            "parameters": {
                "params_version": 3,
                "width": width,
                "height": height,
                "scale": scale,
                "sampler": "k_euler_ancestral",
                "steps": steps,
                "n_samples": 1,
                "ucPreset": 0,
                "qualityToggle": True,
                "dynamic_thresholding": False,
                "controlnet_strength": 1,
                "legacy": False,
                "add_original_image": True,
                "cfg_rescale": 0,
                "noise_schedule": "karras",
                "seed": seed,
                "negative_prompt": negative_prompt,
                "reference_image_multiple": encoded_vibes,
                "reference_strength_multiple": strengths,
                "reference_information_extracted_multiple": information_extracted_values,
                "normalize_reference_strength_multiple": True,
                "deliberate_euler_ancestral_bug": False,
                "prefer_brownian": True,
            }
        }
        
        # V4プロンプト構造を追加
        payload["parameters"]["v4_prompt"] = {
            "caption": {
                "base_caption": prompt,
                "char_captions": []
            },
            "use_coords": True,
            "use_order": True
        }
        payload["parameters"]["v4_negative_prompt"] = {
            "caption": {
                "base_caption": negative_prompt,
                "char_captions": []
            },
            "legacy_uc": False
        }
        
        # APIリクエスト
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json",
        }
        
        with httpx.Client(timeout=120.0) as client:
            response = client.post(self.API_URL, json=payload, headers=headers)
            response.raise_for_status()
            
            # レスポンスはZIPファイル
            with zipfile.ZipFile(io.BytesIO(response.content)) as zf:
                # 最初の画像ファイルを取得
                for name in zf.namelist():
                    if name.endswith(('.png', '.webp', '.jpg', '.jpeg')):
                        return zf.read(name)
            
            raise ValueError("No image found in response")
    
    def generate_with_encoded_and_save(
        self,
        prompt: str,
        encoded_vibes: Union[str, List[str]],
        output_path: Optional[str | Path] = None,
        **kwargs
    ) -> Path:
        """
        エンコード済みVibeデータで画像を生成してファイルに保存
        
        Args:
            prompt: 生成プロンプト
            encoded_vibes: エンコード済みVibeデータ
            output_path: 出力パス（Noneの場合は自動生成）
            **kwargs: generate_with_encoded() に渡す追加パラメータ
            
        Returns:
            保存されたファイルのパス
        """
        image_data = self.generate_with_encoded(prompt, encoded_vibes, **kwargs)
        
        if output_path is None:
            output_dir = Path("output")
            output_dir.mkdir(parents=True, exist_ok=True)
            output_path = output_dir / f"vibe_{datetime.now().strftime('%Y%m%d_%H%M%S')}.png"
        else:
            output_path = Path(output_path)
            output_path.parent.mkdir(parents=True, exist_ok=True)
        
        with open(output_path, "wb") as f:
            f.write(image_data)
        
        return output_path
    
    def generate_multi_character(
        self,
        base_prompt: str,
        characters: List[CharacterConfig],
        encoded_vibes: Optional[Union[str, List[str]]] = None,
        vibe_paths: Optional[Union[str, Path, List[Union[str, Path]]]] = None,
        strengths: Optional[Union[float, List[float]]] = None,
        information_extracted_values: Optional[Union[float, List[float]]] = None,
        base_negative_prompt: str = "",
        model: str = "nai-diffusion-4-5-full",
        width: int = 832,
        height: int = 1216,
        steps: int = 23,
        scale: float = 5.0,
        seed: Optional[int] = None,
    ) -> bytes:
        """
        複数キャラクターを含む画像を生成（位置指定・個別プロンプト対応）
        
        Args:
            base_prompt: 背景・カメラワーク等の共通プロンプト（例: "school classroom, sunny day"）
            characters: キャラクター設定のリスト
            encoded_vibes: エンコード済みVibeデータ（グローバルに適用）
            vibe_paths: .naiv4vibe ファイルパスのリスト
            strengths: 各Vibeの強度
            information_extracted_values: 各Vibeのinformation_extracted値
            base_negative_prompt: 共通のネガティブプロンプト
            model: モデル名
            width: 画像幅
            height: 画像高さ
            steps: 生成ステップ数
            scale: CFG Scale
            seed: シード値（Noneの場合はランダム）
            
        Returns:
            生成された画像のバイトデータ (PNG)
            
        Example:
            characters = [
                CharacterConfig(
                    prompt="1girl, blonde hair, blue eyes, school uniform",
                    center_x=0.3, center_y=0.5,
                    negative_prompt="eyepatch"
                ),
                CharacterConfig(
                    prompt="1boy, black hair, glasses, school uniform", 
                    center_x=0.7, center_y=0.5
                ),
            ]
            image = client.generate_multi_character(
                base_prompt="school classroom, wide shot",
                characters=characters,
                vibe_paths=["style.naiv4vibe"]
            )
        """
        import random
        
        # Vibeデータの取得
        vibes = []
        info_extracted_list = []
        
        if encoded_vibes:
            if isinstance(encoded_vibes, str):
                vibes = [encoded_vibes]
            else:
                vibes = list(encoded_vibes)
            # デフォルトのinformation_extracted
            info_extracted_list = [1.0] * len(vibes)
        
        if vibe_paths:
            if isinstance(vibe_paths, (str, Path)):
                vibe_paths = [vibe_paths]
            for vp in vibe_paths:
                vibe_data = self.load_vibe_file(vp)
                encoding, info_extracted, _ = self.extract_encoding(vibe_data, model)
                vibes.append(encoding)
                info_extracted_list.append(info_extracted)
        
        # 強度の正規化
        n_vibes = len(vibes)
        if n_vibes > 0:
            if strengths is None:
                strengths = [0.7] * n_vibes
            elif isinstance(strengths, (int, float)):
                strengths = [float(strengths)] * n_vibes
            
            if information_extracted_values is None:
                information_extracted_values = info_extracted_list
            elif isinstance(information_extracted_values, (int, float)):
                information_extracted_values = [float(information_extracted_values)] * n_vibes
        
        # シード生成
        if seed is None:
            seed = random.randint(0, 2**32 - 1)
        
        # キャラクタープロンプト構築
        char_captions = [char.to_caption_dict() for char in characters]
        char_negative_captions = [char.to_negative_caption_dict() for char in characters]
        
        # リクエストペイロード構築
        payload = {
            "input": base_prompt,  # APIの入力はbase_prompt
            "model": model,
            "action": "generate",
            "parameters": {
                "params_version": 3,
                "width": width,
                "height": height,
                "scale": scale,
                "sampler": "k_euler_ancestral",
                "steps": steps,
                "n_samples": 1,
                "ucPreset": 0,
                "qualityToggle": True,
                "dynamic_thresholding": False,
                "controlnet_strength": 1,
                "legacy": False,
                "add_original_image": True,
                "cfg_rescale": 0,
                "noise_schedule": "karras",
                "seed": seed,
                "negative_prompt": base_negative_prompt,
                "deliberate_euler_ancestral_bug": False,
                "prefer_brownian": True,
            }
        }
        
        # Vibeパラメータを追加（存在する場合）
        if vibes:
            payload["parameters"]["reference_image_multiple"] = vibes
            payload["parameters"]["reference_strength_multiple"] = strengths
            payload["parameters"]["reference_information_extracted_multiple"] = information_extracted_values
            payload["parameters"]["normalize_reference_strength_multiple"] = True
        
        # V4プロンプト構造（キャラクター対応）
        payload["parameters"]["v4_prompt"] = {
            "caption": {
                "base_caption": base_prompt,
                "char_captions": char_captions
            },
            "use_coords": True,
            "use_order": True
        }
        payload["parameters"]["v4_negative_prompt"] = {
            "caption": {
                "base_caption": base_negative_prompt,
                "char_captions": char_negative_captions
            },
            "legacy_uc": False
        }
        
        # デバッグ: ペイロード確認
        # import json as json_module
        # print("Payload v4_prompt:", json_module.dumps(payload["parameters"]["v4_prompt"], indent=2, ensure_ascii=False))
        # print("Payload v4_negative_prompt:", json_module.dumps(payload["parameters"]["v4_negative_prompt"], indent=2, ensure_ascii=False))
        
        # APIリクエスト
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json",
        }
        
        with httpx.Client(timeout=120.0) as client:
            response = client.post(self.API_URL, json=payload, headers=headers)
            if response.status_code != 200:
                print(f"Error response: {response.text}")
            response.raise_for_status()
            
            # レスポンスはZIPファイル
            with zipfile.ZipFile(io.BytesIO(response.content)) as zf:
                for name in zf.namelist():
                    if name.endswith(('.png', '.webp', '.jpg', '.jpeg')):
                        return zf.read(name)
            
            raise ValueError("No image found in response")
    
    def generate_multi_character_and_save(
        self,
        base_prompt: str,
        characters: List[CharacterConfig],
        output_path: Optional[str | Path] = None,
        **kwargs
    ) -> Path:
        """
        複数キャラクター画像を生成してファイルに保存
        
        Args:
            base_prompt: 背景・カメラワーク等の共通プロンプト
            characters: キャラクター設定のリスト
            output_path: 出力パス（Noneの場合は自動生成）
            **kwargs: generate_multi_character() に渡す追加パラメータ
            
        Returns:
            保存されたファイルのパス
        """
        image_data = self.generate_multi_character(base_prompt, characters, **kwargs)
        
        if output_path is None:
            output_dir = Path("output")
            output_dir.mkdir(parents=True, exist_ok=True)
            output_path = output_dir / f"multi_char_{datetime.now().strftime('%Y%m%d_%H%M%S')}.png"
        else:
            output_path = Path(output_path)
            output_path.parent.mkdir(parents=True, exist_ok=True)
        
        with open(output_path, "wb") as f:
            f.write(image_data)
        
        return output_path

    def generate(
        self,
        prompt: str,
        vibe_path: str | Path,
        negative_prompt: str = "nsfw, lowres, artistic error, worst quality, bad quality, jpeg artifacts",
        model: str = "nai-diffusion-4-5-full",
        width: int = 832,
        height: int = 1216,
        steps: int = 23,
        scale: float = 5.0,
        seed: Optional[int] = None,
        strength: float = 0.7,
    ) -> bytes:
        """
        Vibe Transferを使用して画像を生成
        
        Args:
            prompt: 生成プロンプト
            vibe_path: .naiv4vibe ファイルのパス
            negative_prompt: ネガティブプロンプト
            model: モデル名
            width: 画像幅
            height: 画像高さ
            steps: 生成ステップ数
            scale: CFG Scale
            seed: シード値（Noneの場合はランダム）
            strength: Vibe Transfer の強度 (0.0-1.0)
            
        Returns:
            生成された画像のバイトデータ (PNG)
        """
        import random
        
        # Vibeファイルを読み込み
        vibe_data = self.load_vibe_file(vibe_path)
        encoding, information_extracted, mask = self.extract_encoding(vibe_data, model)
        
        # シード生成
        if seed is None:
            seed = random.randint(0, 2**32 - 1)
        
        # リクエストペイロード構築
        payload = {
            "input": prompt,
            "model": model,
            "action": "generate",
            "parameters": {
                "params_version": 3,
                "width": width,
                "height": height,
                "scale": scale,
                "sampler": "k_euler_ancestral",
                "steps": steps,
                "n_samples": 1,
                "ucPreset": 0,
                "qualityToggle": True,
                "dynamic_thresholding": False,
                "controlnet_strength": 1,
                "legacy": False,
                "add_original_image": True,
                "cfg_rescale": 0,
                "noise_schedule": "karras",
                "seed": seed,
                "negative_prompt": negative_prompt,
                "reference_image_multiple": [encoding],
                "reference_strength_multiple": [strength],
                "reference_information_extracted_multiple": [information_extracted],
                "normalize_reference_strength_multiple": True,
                "deliberate_euler_ancestral_bug": False,
                "prefer_brownian": True,
            }
        }
        
        # V4プロンプト構造を追加
        payload["parameters"]["v4_prompt"] = {
            "caption": {
                "base_caption": prompt,
                "char_captions": []
            },
            "use_coords": True,
            "use_order": True
        }
        payload["parameters"]["v4_negative_prompt"] = {
            "caption": {
                "base_caption": negative_prompt,
                "char_captions": []
            },
            "legacy_uc": False
        }
        
        # APIリクエスト
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json",
        }
        
        with httpx.Client(timeout=120.0) as client:
            response = client.post(self.API_URL, json=payload, headers=headers)
            response.raise_for_status()
            
            # レスポンスはZIPファイル
            with zipfile.ZipFile(io.BytesIO(response.content)) as zf:
                # 最初の画像ファイルを取得
                for name in zf.namelist():
                    if name.endswith(('.png', '.webp', '.jpg', '.jpeg')):
                        return zf.read(name)
            
            raise ValueError("No image found in response")
    
    def generate_and_save(
        self,
        prompt: str,
        vibe_path: str | Path,
        output_path: Optional[str | Path] = None,
        **kwargs
    ) -> Path:
        """
        画像を生成してファイルに保存
        
        Args:
            prompt: 生成プロンプト
            vibe_path: .naiv4vibe ファイルのパス
            output_path: 出力パス（Noneの場合は自動生成）
            **kwargs: generate() に渡す追加パラメータ
            
        Returns:
            保存されたファイルのパス
        """
        image_data = self.generate(prompt, vibe_path, **kwargs)
        
        if output_path is None:
            output_dir = Path("output")
            output_dir.mkdir(parents=True, exist_ok=True)
            output_path = output_dir / f"vibe_{datetime.now().strftime('%Y%m%d_%H%M%S')}.png"
        else:
            output_path = Path(output_path)
            output_path.parent.mkdir(parents=True, exist_ok=True)
        
        with open(output_path, "wb") as f:
            f.write(image_data)
        
        return output_path


def main():
    """使用例"""
    from dotenv import load_dotenv
    load_dotenv()
    
    # クライアント初期化
    client = VibeTransferClient()
    
    # Vibeファイルのパス（ユーザーが用意）
    vibe_path = "your_vibe.naiv4vibe"
    
    # プロンプト
    prompt = "1girl, beautiful anime girl, detailed eyes, masterpiece"
    
    # 生成と保存
    output = client.generate_and_save(
        prompt=prompt,
        vibe_path=vibe_path,
        strength=0.7,
    )
    print(f"Generated: {output}")


if __name__ == "__main__":
    main()
