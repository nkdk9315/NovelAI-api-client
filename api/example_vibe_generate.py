"""
Vibe Transfer 画像生成サンプル

使用方法:
1. .naiv4vibe ファイルを使用（Anlas消費なし）
2. 生画像をエンコードして使用（エンコード時に2 Anlas消費）
3. エンコード済みデータを直接使用（Anlas消費なし）

python example_vibe_generate.py
"""

from pathlib import Path
from dotenv import load_dotenv
from vibe_transfer import VibeTransferClient, CharacterConfig


def example_with_vibe_file():
    """
    方法1: .naiv4vibe ファイルを使用（Anlas消費なし）
    NovelAI公式サイトからエクスポートしたファイルを使用
    """
    print("\n=== 方法1: .naiv4vibe ファイルを使用 ===")
    
    client = VibeTransferClient()
    
    # .naiv4vibe ファイルのパス
    vibe_path = Path("jjjj.naiv4vibe")
    
    if not vibe_path.exists():
        print(f"Vibe file not found: {vibe_path}")
        print("Please export a .naiv4vibe file from NovelAI official site.")
        return None
    
    prompt = "1girl, beautiful anime girl, detailed eyes, masterpiece, best quality"
    
    print(f"Vibe file: {vibe_path}")
    print(f"Prompt: {prompt}")
    print("Generating...")
    
    output = client.generate_and_save(
        prompt=prompt,
        vibe_path=vibe_path,
        strength=0.7,
        width=832,
        height=1216,
    )
    print(f"✓ Generated: {output}")
    return output


def example_encode_and_use():
    """
    方法2: 生画像をエンコードして使用（エンコード時に2 Anlas消費）
    任意の画像をVibeエンコードし、そのまま使用または.naiv4vibeファイルに保存
    """
    print("\n=== 方法2: 生画像をエンコードして使用 ===")
    
    client = VibeTransferClient()
    
    # エンコードする画像ファイル
    image_path = Path("20251215_231647.png")
    
    if not image_path.exists():
        print(f"Reference image not found: {image_path}")
        print("Please provide a reference image file.")
        return None
    
    # 方法2a: エンコードして.naiv4vibeファイルに保存
    print(f"Encoding image: {image_path}")
    vibe_file = client.encode_vibe_and_save(
        image_path=image_path,
        information_extracted=0.7,
        strength=0.7,
    )
    print(f"✓ Saved vibe file: {vibe_file}")
    
    # 保存したファイルを使用して画像生成
    prompt = "1girl, beautiful anime girl, detailed eyes, masterpiece"
    print(f"Generating with prompt: {prompt}")
    
    output = client.generate_and_save(
        prompt=prompt,
        vibe_path=vibe_file,
        strength=0.7,
    )
    print(f"✓ Generated: {output}")
    return output


def example_encode_directly():
    """
    方法2b: エンコードを直接取得して使用（ファイル保存なし）
    """
    print("\n=== 方法2b: エンコードを直接取得して使用 ===")
    
    client = VibeTransferClient()
    
    image_path = Path("reference_image.png")
    
    if not image_path.exists():
        print(f"Reference image not found: {image_path}")
        return None
    
    # エンコードを直接取得（2 Anlas消費）
    print(f"Encoding image: {image_path}")
    encoding = client.encode_vibe(
        image_path=image_path,
        information_extracted=0.7,
    )
    print(f"✓ Got encoding (length: {len(encoding)} chars)")
    
    # エンコードを使用して画像生成
    prompt = "1girl, beautiful anime girl, detailed eyes"
    output = client.generate_with_encoded_and_save(
        prompt=prompt,
        encoded_vibes=encoding,
        strengths=0.7,
        information_extracted_values=0.7,
    )
    print(f"✓ Generated: {output}")
    return output


def example_with_encoded_string():
    """
    方法3: エンコード済み文字列を直接使用（Anlas消費なし）
    事前に取得したエンコーディングを再利用
    """
    print("\n=== 方法3: エンコード済み文字列を直接使用 ===")
    
    client = VibeTransferClient()
    
    # 事前に取得したエンコーディング（例）
    # 実際には encode_vibe() で取得した文字列を使用
    encoded_vibe = None  # ここにエンコード済み文字列をペースト
    
    if encoded_vibe is None:
        print("No encoded vibe string provided.")
        print("First, use encode_vibe() to get an encoding, then paste it here.")
        return None
    
    prompt = "1girl, beautiful anime girl, detailed eyes, masterpiece"
    
    output = client.generate_with_encoded_and_save(
        prompt=prompt,
        encoded_vibes=encoded_vibe,
        strengths=0.7,
    )
    print(f"✓ Generated: {output}")
    return output


def example_multiple_vibes():
    """
    複数のVibeを同時に使用
    """
    print("\n=== 複数のVibeを同時に使用 ===")
    
    client = VibeTransferClient()
    
    # 複数の画像をエンコード
    image_paths = [
        Path("reference1.png"),
        Path("reference2.png"),
    ]
    
    encodings = []
    for img_path in image_paths:
        if img_path.exists():
            print(f"Encoding: {img_path}")
            enc = client.encode_vibe(image_path=img_path, information_extracted=0.7)
            encodings.append(enc)
        else:
            print(f"Image not found: {img_path}")
    
    if len(encodings) < 2:
        print("Need at least 2 images for this example.")
        return None
    
    # 複数のVibeを使用して生成
    prompt = "1girl, beautiful anime girl, detailed eyes"
    output = client.generate_with_encoded_and_save(
        prompt=prompt,
        encoded_vibes=encodings,
        strengths=[0.5, 0.5],  # 各Vibeの強度
        information_extracted_values=[0.7, 0.7],
    )
    print(f"✓ Generated: {output}")
    return output


def example_multi_character():
    """
    マルチキャラクター生成
    キャラクターごとの位置指定と個別プロンプト対応
    """
    print("\n=== マルチキャラクター生成 ===")
    
    client = VibeTransferClient()
    
    # キャラクター設定
    characters = [
        CharacterConfig(
            prompt="1girl, blonde hair, blue eyes, school uniform, smile",
            center_x=0.3,  # 左寄り
            center_y=0.5,
            negative_prompt="eyepatch, scar"
        ),
        CharacterConfig(
            prompt="1boy, black hair, glasses, school uniform, serious expression",
            center_x=0.7,  # 右寄り
            center_y=0.5,
            negative_prompt="beard, mustache"
        ),
    ]
    
    # 背景・シーンのプロンプト
    base_prompt = "school classroom, sunny day, wide shot, detailed background"
    
    print(f"Base prompt: {base_prompt}")
    print(f"Characters: {len(characters)}")
    for i, char in enumerate(characters):
        print(f"  Character {i+1}: pos=({char.center_x}, {char.center_y})")
        print(f"    Prompt: {char.prompt[:50]}...")
    
    # Vibeファイルを使用する場合（オプション）
    vibe_path = Path("style.naiv4vibe")
    vibe_paths = [vibe_path] if vibe_path.exists() else None
    
    if vibe_paths:
        print(f"Using vibe: {vibe_path}")
    else:
        print("No vibe file (generating without vibe)")
    
    print("Generating...")
    
    output = client.generate_multi_character_and_save(
        base_prompt=base_prompt,
        characters=characters,
        vibe_paths=vibe_paths,
        strengths=0.7,
        width=1216,  # 横長（複数キャラ向け）
        height=832,
    )
    print(f"✓ Generated: {output}")
    return output


def example_multi_character_with_vibes():
    """
    複数のVibeを使用したマルチキャラクター生成
    """
    print("\n=== 複数Vibe + マルチキャラクター生成 ===")
    
    client = VibeTransferClient()
    
    # 複数のVibeファイル
    vibe_files = [
        Path("20251215_231647.naiv4vibe"),
        Path("えのきっぷ1.naiv4vibe"),
        Path("漆黒の性王.naiv4vibe"),
    ]
    
    existing_vibes = [v for v in vibe_files if v.exists()]
    if not existing_vibes:
        print("No vibe files found. Skipping...")
        return None
    
    # キャラクター設定
    characters = [
        CharacterConfig(
            prompt="3::cynthia (pokemon) school uniform::,  3::saliva drip::, 2::embarrassed::, large areolae, 3::nude::, -2::loli::,2::deep kiss::, 3::saliva on breasts and areolae::",
            center_x=0.2,
            center_y=0.5
        ),
         CharacterConfig(
            prompt="2::fat man::, 2::ugly::, 3::deep kiss, 3::saliva drip::,  ",
            center_x=0.8,
            center_y=0.5
         ),
    ]
    
    output = client.generate_multi_character_and_save(
        base_prompt="school classroom, detailed background, -3::multiple views::, 3::face focus:: ",
        characters=characters,
        vibe_paths=existing_vibes,
        # strengths=[0.5] * len(existing_vibes),
        strengths=[0.2, 0.2, 0.7],
        width=1024,
        height=1024,
    )
    print(f"✓ Generated: {output}")
    return output


def main():
    # 環境変数をロード
    load_dotenv()
    
    print("=" * 50)
    print("Vibe Transfer 使用例")
    print("=" * 50)
    
    # 使用する方法を選択してコメントアウトを外してください
    
    # 方法1: .naiv4vibe ファイルを使用
    # try:
    #     example_with_vibe_file()
    # except Exception as e:
    #     print(f"✗ Error: {e}")
    
    # 方法2: 生画像をエンコードして使用
    # try:
    #     example_encode_and_use()
    # except Exception as e:
    #     print(f"✗ Error: {e}")
    
    # 方法2b: エンコードを直接取得して使用
    # try:
    #     example_encode_directly()
    # except Exception as e:
    #     print(f"✗ Error: {e}")
    
    # 方法3: エンコード済み文字列を直接使用
    # try:
    #     example_with_encoded_string()
    # except Exception as e:
    #     print(f"✗ Error: {e}")
    
    # 複数Vibeの使用例
    # try:
    #     example_multiple_vibes()
    # except Exception as e:
    #     print(f"✗ Error: {e}")
    
    # マルチキャラクター生成
    # try:
    #     example_multi_character()
    # except Exception as e:
    #     print(f"✗ Error: {e}")
    
    #i複数Vibe + マルチキャラクター生成
    try:
        example_multi_character_with_vibes()
    except Exception as e:
        print(f"✗ Error: {e}")


if __name__ == "__main__":
    main()
