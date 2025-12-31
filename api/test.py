from dotenv import load_dotenv
load_dotenv()
from pathlib import Path
from novelai import NovelAIClient, CharacterConfig

client = NovelAIClient()

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
            prompt="",
            center_x=0.8,
            center_y=0.5
         ),
    ]
    vibe_files = [
        Path("えのきっぷ1.naiv4vibe"),
        Path("20251215_231647.naiv4vibe"),
        Path("漆黒の性王.naiv4vibe"),
        Path("vibes/890bc110faa4_20251228_172917.naiv4vibe"),
    ]
    result = client.generate(
        prompt="",
        characters=characters,
        vibes=vibe_files,
        vibe_strengths=[0.4, 0.3, 0.5, 0.2], 
        width=1024,  # 横長
        height=1024,
        save_dir="output2/multi_character/"
    )
    print(f"✓ Generated: {result.saved_path}")
    print(f"残りアンラス: {result.anlas_remaining}")
    print(f"今回消費: {result.anlas_consumed}")  # 通常は2
    return result

example_multi_character()