"""
NovelAI Client Utilities
画像処理・ユーティリティ関数

novelai_client.py から抽出した共通処理
"""

import base64
import io
import json
from pathlib import Path
from typing import Union, List, Tuple

from .constants import (
    MODEL_KEY_MAP,
    CHARREF_PORTRAIT_SIZE,
    CHARREF_LANDSCAPE_SIZE,
    CHARREF_SQUARE_SIZE,
)
from .dataclasses import (
    CharacterConfig,
    CharacterReferenceConfig,
    VibeEncodeResult,
)


def get_image_bytes(image: Union[str, Path, bytes]) -> bytes:
    """
    画像データをバイトに変換
    
    Args:
        image: 画像ファイルパス、バイトデータ、またはBase64文字列
        
    Returns:
        画像のバイトデータ
    """
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


def get_image_base64(image: Union[str, Path, bytes]) -> str:
    """
    画像をBase64文字列に変換
    
    Args:
        image: 画像ファイルパス、バイトデータ、またはBase64文字列
        
    Returns:
        Base64エンコードされた文字列
    """
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


def load_vibe_file(vibe_path: Union[str, Path]) -> dict:
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


def extract_encoding(
    vibe_data: dict,
    model: str = "nai-diffusion-4-5-full"
) -> Tuple[str, float]:
    """
    Vibeデータからエンコード情報を抽出
    
    Args:
        vibe_data: パースされたVibeデータ辞書
        model: 使用するモデル名
        
    Returns:
        (encoding, information_extracted) のタプル
    """
    model_key = MODEL_KEY_MAP.get(model, "v4-5full")
    
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


def process_vibes(
    vibes: List[Union[str, Path, VibeEncodeResult]],
    model: str
) -> Tuple[List[str], List[float]]:
    """
    Vibeリストをエンコードリストに変換
    
    Args:
        vibes: Vibeリスト（パス、エンコード文字列、VibeEncodeResult）
        model: 使用するモデル名
        
    Returns:
        (encodings, info_extracted_list) のタプル
    """
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
                data = load_vibe_file(path)
                encoding, info_extracted = extract_encoding(data, model)
                encodings.append(encoding)
                info_extracted_list.append(info_extracted)
            else:
                # エンコード済み文字列として使用
                encodings.append(str(vibe))
                info_extracted_list.append(1.0)
    
    return encodings, info_extracted_list


def prepare_character_reference_image(image_bytes: bytes) -> bytes:
    """
    キャラクター参照画像を適切なサイズに変換
    
    API仕様: 1024x1536 or 1536x1024 or 1472x1472 with black padding to fit
    
    Args:
        image_bytes: 元画像のバイトデータ
        
    Returns:
        リサイズ・パディングされた画像のバイトデータ（PNG形式）
    """
    from PIL import Image
    
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
        target_width, target_height = CHARREF_PORTRAIT_SIZE
    elif aspect_ratio > 1.25:
        # 横長
        target_width, target_height = CHARREF_LANDSCAPE_SIZE
    else:
        # 正方形に近い
        target_width, target_height = CHARREF_SQUARE_SIZE
    
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


def process_character_references(
    refs: List[CharacterReferenceConfig],
) -> Tuple[List[str], List[dict], List[float], List[float], List[float]]:
    """
    キャラクター参照リストをAPI用パラメータに変換
    
    API仕様: director_reference_images は
    1024x1536, 1536x1024, または 1472x1472 で黒いパディングが必要
    
    Args:
        refs: キャラクター参照設定リスト
        
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
        image_bytes = get_image_bytes(ref.image)
        
        # 正しいサイズにリサイズ・パディング
        processed_bytes = prepare_character_reference_image(image_bytes)
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


# =============================================================================
# エクスポート
# =============================================================================

__all__ = [
    "get_image_bytes",
    "get_image_base64",
    "load_vibe_file",
    "extract_encoding",
    "process_vibes",
    "prepare_character_reference_image",
    "process_character_references",
]
