"""
NovelAI Unified Client 使用例

すべての機能が統合された generate() と encode_vibe() の使用例

python example_unified_client.py
"""

from pathlib import Path
from dotenv import load_dotenv
from novelai import NovelAIClient, CharacterConfig, CharacterReferenceConfig

load_dotenv()


def example_simple_generate():
    """シンプルなテキストから画像生成"""
    print("\n=== シンプル生成 ===")
    
    client = NovelAIClient()
    
    result = client.generate(
        "1girl, beautiful anime girl, detailed eyes, masterpiece, best quality",
        save_dir="output/"
    )
    
    print(f"✓ Generated: {result.saved_path}")
    print(f"  Seed: {result.seed}")
    return result


def example_with_vibes():
    """Vibe Transferを使用した生成"""
    print("\n=== Vibe Transfer使用 ===")
    
    client = NovelAIClient()
    
    # .naiv4vibeファイルを指定
    vibe_files = [
        Path("えのきっぷ1.naiv4vibe"),
        Path("20251215_231647.naiv4vibe"),
        Path("漆黒の性王.naiv4vibe"),
    ]
    
    existing_vibes = [v for v in vibe_files if v.exists()]
    if not existing_vibes:
        print("Vibeファイルが見つかりません")
        return None
    
    result = client.generate(
        "1girl, beautiful anime girl, detailed eyes",
        vibes=existing_vibes,
        vibe_strengths=[0.5] * len(existing_vibes),
        save_dir="output/"
    )
    
    print(f"✓ Generated: {result.saved_path}")
    print(f"残りアンラス: {result.anlas_remaining}")
    print(f"今回消費: {result.anlas_consumed}")  # 通常は2
    return result


def example_img2img():
    """Image2Image生成"""
    print("\n=== Image2Image ===")
    
    client = NovelAIClient()
    
    input_image = Path("reference/input.png")
    if not input_image.exists():
        print(f"入力画像が見つかりません: {input_image}")
        return None
    
    result = client.generate(
        "1girl, beautiful anime girl, detailed eyes, masterpiece",
        action="img2img", 
        source_image=input_image,
        img2img_strength=0.6,  # 0に近いほど元画像に近い
        save_dir="output/"
    )
    
    print(f"✓ Generated: {result.saved_path}")
    print(f"残りアンラス: {result.anlas_remaining}")
    print(f"今回消費: {result.anlas_consumed}")  # 通常は2
    return result


def example_img2img_with_vibes():
    """Image2Image + Vibe Transfer"""
    print("\n=== Image2Image + Vibe Transfer ===")
    
    client = NovelAIClient()
    
    input_image = Path("reference/input.png")
    vibe_file = Path("えのきっぷ1.naiv4vibe")
    
    if not input_image.exists():
        print(f"入力画像が見つかりません: {input_image}")
        return None
    
    vibes = [vibe_file] if vibe_file.exists() else None
    
    result = client.generate(
        "",
        action="img2img",
        source_image=input_image,
        img2img_strength=0.7,
        img2img_noise=0,
        vibes=vibes,
        vibe_strengths=[0.7] if vibes else None,
        save_dir="output/"
    )
    
    print(f"✓ Generated: {result.saved_path}")
    print(f"残りアンラス: {result.anlas_remaining}")
    print(f"今回消費: {result.anlas_consumed}")  # 通常は2
    return result


def example_multi_character():
    """複数キャラクター生成"""
    print("\n=== 複数キャラクター ===")
    
    client = NovelAIClient()
    
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
    vibe_files = [
        Path("vibes/えのきっぷ1.naiv4vibe"),
        Path("vibes/20251215_231647.naiv4vibe"),
        Path("vibes/漆黒の性王.naiv4vibe"),
        Path("vibes/890bc110faa4_20251231_134734.naiv4vibe"),
    ]
    result = client.generate(
        "school classroom, sunny day, wide shot, detailed background, 2::face focus::, -3::multiple views::",
        characters=characters,
        vibes=vibe_files,
        vibe_strengths=[0.4, 0.3, 0.5, 0.2], 
        width=1024,  # 横長
        height=1024,
        save_dir="output/multi_character/"
    )
    print(f"✓ Generated: {result.saved_path}")
    print(f"残りアンラス: {result.anlas_remaining}")
    print(f"今回消費: {result.anlas_consumed}")  # 通常は2
    return result


def example_img2img_multi_character_with_vibes():
    """Image2Image + 複数キャラクター + Vibe Transfer（全部入り）"""
    print("\n=== 全部入り: img2img + マルチキャラ + Vibe ===")
    
    client = NovelAIClient()
    
    # 入力画像
    input_image = Path("reference/input.png")
    if not input_image.exists():
        print(f"入力画像が見つかりません: {input_image}")
        return None
    
    # Vibeファイル
    vibe_files = [
        Path("えのきっぷ1.naiv4vibe"),
        Path("20251215_231647.naiv4vibe"),
        Path("漆黒の性王.naiv4vibe"),
        Path("890bc110faa4_20251228_172917.naiv4vibe"),
    ]
    existing_vibes = [v for v in vibe_files if v.exists()]
    
    # キャラクター設定
    characters = [
        CharacterConfig(
            prompt="1girl, blonde hair, school uniform",
            center_x=0.3,
            center_y=0.5
        ),
        CharacterConfig(
            prompt="1boy, black hair, 2::fat, ugly::, 3::deep kiss::",
            center_x=0.7,
            center_y=0.5
        ),
    ]
    
    result = client.generate(
        "school classroom, wide shot",
        action="img2img",
        source_image=input_image,
        img2img_strength=0.7,
        img2img_noise=0,
        characters=characters,
        vibes=existing_vibes if existing_vibes else None,
        vibe_strengths=[0.5] * len(existing_vibes) if existing_vibes else None,
        width=1024,
        height=1024,
        save_dir="output/"
    )
    
    print(f"✓ Generated: {result.saved_path}")
    print(f"残りアンラス: {result.anlas_remaining}")
    print(f"今回消費: {result.anlas_consumed}") 
     # 通常は2  
    return result


def example_encode_vibe():
    """画像をVibeエンコード"""
    print("\n=== Vibeエンコード ===")
    
    client = NovelAIClient()
    
    image_path = Path("reference/input.png")
    if not image_path.exists():
        print(f"参照画像が見つかりません: {image_path}")
        return None
    
    # エンコードのみ（保存なし）
    result = client.encode_vibe(
        image_path,
        information_extracted=0.5,
        strength=0.7,
    )
    print(f"✓ Encoded (hash: {result.source_image_hash[:12]}...)")
    
    # エンコード + 自動保存
    result_saved = client.encode_vibe(
        image_path,
        save_dir="vibes/"
    )
    print(f"✓ Saved: {result_saved.saved_path}")
    
    # DB保存用データ
    db_data = result_saved.to_dict()
    print(f"  DB data keys: {list(db_data.keys())}")

    print(f"残りアンラス: {result.anlas_remaining}")
    print(f"今回消費: {result.anlas_consumed}")  # 通常は2
    
    return result_saved


def example_encode_and_generate():
    """エンコードしたVibeで生成"""
    print("\n=== エンコード → 生成 ===")
    
    client = NovelAIClient()
    
    image_path = Path("reference.png")
    if not image_path.exists():
        print(f"参照画像が見つかりません: {image_path}")
        return None
    
    # 1. エンコード
    vibe_result = client.encode_vibe(
        image_path,
        information_extracted=0.7,
        save_dir="vibes/"
    )
    print(f"✓ Encoded: {vibe_result.saved_path}")
    
    # 2. 生成に使用（VibeEncodeResultをそのまま渡せる）
    gen_result = client.generate(
        "1girl, beautiful anime girl",
        vibes=[vibe_result],  # VibeEncodeResultを直接使用
        vibe_strengths=[0.7],
        save_dir="output/"
    )
    print(f"✓ Generated: {gen_result.saved_path}")
    print(f"残りアンラス: {gen_result.anlas_remaining}")
    print(f"今回消費: {gen_result.anlas_consumed}")  # 通常は2

    return gen_result


def example_character_reference():
    """キャラクター参照を使用した生成（5 Anlas消費）"""
    print("\n=== キャラクター参照 ===")
    
    client = NovelAIClient()
    
    # 参照画像のパス
    reference_image = Path("reference/input.png")
    if not reference_image.exists():
        print(f"参照画像が見つかりません: {reference_image}")
        return None
    
    # キャラクター参照設定
    char_ref = CharacterReferenceConfig(
        image=reference_image,
        fidelity=0.8,           # キャラクターの反映度 (0.0-1.0)
        include_style=True,     # 絵柄も参照する
    )
    
    # キャラクター位置設定
    characters = [
        CharacterConfig(
            prompt="3::peeing::",
            center_x=0.5,
            center_y=0.5
        ),
    ]
    
    result = client.generate(
        "school classroom, sunny day, detailed background",
        characters=characters,
        character_reference=char_ref,
        save_dir="output/charref/"
    )
    
    print(f"✓ Generated: {result.saved_path}")
    print(f"  Seed: {result.seed}")
    if result.anlas_remaining is not None:
        print(f"  残りアンラス: {result.anlas_remaining}")
    
    return result


def example_character_reference_style_off():
    """キャラクター参照（絵柄参照OFF）"""
    print("\n=== キャラクター参照（絵柄OFF） ===")
    
    client = NovelAIClient()
    
    reference_image = Path("reference/input.png")
    if not reference_image.exists():
        print(f"参照画像が見つかりません: {reference_image}")
        return None
    
    char_ref = CharacterReferenceConfig(
        image=reference_image,
        fidelity=1.0,
        include_style=False,  # 絵柄は参照しない（キャラクターのみ）
    )
    
    result = client.generate(
        "1girl, standing, masterpiece",
        character_reference=char_ref,
        save_dir="output/charref/"
    )
    
    print(f"✓ Generated: {result.saved_path}")
    if result.anlas_remaining is not None:
        print(f"  残りアンラス: {result.anlas_remaining}")
    
    return result


def main():
    load_dotenv()
    
    print("=" * 50)
    print("NovelAI Unified Client 使用例")
    print("=" * 50)
    
    # 実行したい例のコメントを外してください
    
    # シンプル生成
    # example_simple_generate()
    
    # Vibe Transfer
    # example_with_vibes()
    
    # Image2Image
    # example_img2img()
    
    # Image2Image + Vibe
    # example_img2img_with_vibes()
    
    # 複数キャラクター
    
    # example_multi_character()
    
    
    # 全部入り
    # example_img2img_multi_character_with_vibes()
    
    # Vibeエンコード
    # example_encode_vibe()
    
    # エンコード → 生成
    # example_encode_and_generate()
    
    # キャラクター参照（5 Anlas消費）
    # example_character_reference()
    
    # キャラクター参照（絵柄OFF）
    # example_character_reference_style_off()
    
    print("\n使用したい例のコメントを外して実行してください。")


if __name__ == "__main__":
    main()
