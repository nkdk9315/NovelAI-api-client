"""
NovelAI トークナイザー調査

NovelAIの公式トークンカウントと一致するトークナイザーを探す
"""

import re

def parse_novelai_prompt(prompt: str) -> dict:
    """
    NovelAI重み構文をパースする
    
    構文例:
    - 2::tag::  → 重み2でtag
    - -1::tag:: → 重み-1（ネガティブ）でtag
    - :: separator → 区切り
    """
    # 重み構文のパターン: 数字::テキスト:: または ::テキスト::
    weight_pattern = r'(-?\d*\.?\d*)::(.*?)::'
    
    weighted_parts = re.findall(weight_pattern, prompt)
    
    # 重み構文を除去したクリーンなテキスト
    clean_text = re.sub(weight_pattern, r'\2', prompt)
    # 残りの :: を除去
    clean_text = clean_text.replace('::', ' ').strip()
    # 複数スペースを1つに
    clean_text = re.sub(r'\s+', ' ', clean_text)
    
    return {
        "original": prompt,
        "clean": clean_text,
        "weighted_parts": weighted_parts,
    }


def test_tokenizers(prompt: str, expected_tokens: int = None):
    """複数のトークナイザーを試す"""
    
    print(f"=" * 60)
    print(f"Prompt: {prompt}")
    if expected_tokens:
        print(f"Expected (NovelAI official): {expected_tokens} tokens")
    print()
    
    # パース
    parsed = parse_novelai_prompt(prompt)
    print(f"Clean text: {parsed['clean']}")
    print(f"Weighted parts: {parsed['weighted_parts']}")
    print()
    
    results = {}
    
    # 1. CLIP ViT-L/14 (Stable Diffusion 1.x, 2.x)
    try:
        from transformers import CLIPTokenizer
        tokenizer = CLIPTokenizer.from_pretrained("openai/clip-vit-large-patch14")
        
        tokens_original = tokenizer.encode(prompt)
        tokens_clean = tokenizer.encode(parsed['clean'])
        
        results["CLIP ViT-L/14 (original)"] = len(tokens_original)
        results["CLIP ViT-L/14 (clean)"] = len(tokens_clean)
        
        print(f"CLIP ViT-L/14 (original): {len(tokens_original)} tokens")
        print(f"  Token IDs: {tokens_original}")
        print(f"CLIP ViT-L/14 (clean): {len(tokens_clean)} tokens")
        print(f"  Token IDs: {tokens_clean}")
        
    except ImportError as e:
        print(f"CLIP ViT-L/14: Not available ({e})")
    
    print()
    
    # 2. CLIP ViT-H/14 (OpenCLIP, used in some newer models)
    try:
        from transformers import CLIPTokenizer
        tokenizer = CLIPTokenizer.from_pretrained("laion/CLIP-ViT-H-14-laion2B-s32B-b79K")
        
        tokens_original = tokenizer.encode(prompt)
        tokens_clean = tokenizer.encode(parsed['clean'])
        
        results["CLIP ViT-H/14 (original)"] = len(tokens_original)
        results["CLIP ViT-H/14 (clean)"] = len(tokens_clean)
        
        print(f"CLIP ViT-H/14 (original): {len(tokens_original)} tokens")
        print(f"CLIP ViT-H/14 (clean): {len(tokens_clean)} tokens")
        
    except Exception as e:
        print(f"CLIP ViT-H/14: Not available ({e})")
    
    print()
    
    # 3. T5 Tokenizer (SDXL uses T5 in addition to CLIP)
    try:
        from transformers import T5Tokenizer
        tokenizer = T5Tokenizer.from_pretrained("google/t5-v1_1-base")
        
        tokens_original = tokenizer.encode(prompt)
        tokens_clean = tokenizer.encode(parsed['clean'])
        
        results["T5 (original)"] = len(tokens_original)
        results["T5 (clean)"] = len(tokens_clean)
        
        print(f"T5 (original): {len(tokens_original)} tokens")
        print(f"T5 (clean): {len(tokens_clean)} tokens")
        
    except Exception as e:
        print(f"T5: Not available ({e})")
    
    print()
    
    # 4. GPT-2 / BPE (一部のモデルで使用)
    try:
        from transformers import GPT2Tokenizer
        tokenizer = GPT2Tokenizer.from_pretrained("gpt2")
        
        tokens_original = tokenizer.encode(prompt)
        tokens_clean = tokenizer.encode(parsed['clean'])
        
        results["GPT-2/BPE (original)"] = len(tokens_original)
        results["GPT-2/BPE (clean)"] = len(tokens_clean)
        
        print(f"GPT-2/BPE (original): {len(tokens_original)} tokens")
        print(f"GPT-2/BPE (clean): {len(tokens_clean)} tokens")
        
    except Exception as e:
        print(f"GPT-2: Not available ({e})")
    
    print()
    
    # 5. Simple word/comma count
    words = prompt.split()
    tags = [t.strip() for t in prompt.split(',') if t.strip()]
    
    print(f"Simple counts:")
    print(f"  Words: {len(words)}")
    print(f"  Tags (comma-separated): {len(tags)}")
    print(f"  Characters: {len(prompt)}")
    
    print()
    
    # 比較
    if expected_tokens:
        print(f"Comparison with expected ({expected_tokens}):")
        for name, count in results.items():
            diff = count - expected_tokens
            match = "✅" if diff == 0 else f"({'+' if diff > 0 else ''}{diff})"
            print(f"  {name}: {count} {match}")
    
    return results


if __name__ == "__main__":
    # ユーザーのテストケース
    test_cases = [
        # (プロンプト, 公式トークン数)
        ("girl, 2::liko (pokemon), ::  -1::hat::, -1::bag::, 2::cure ace (cosplay)::", 25),
    ]
    
    for prompt, expected in test_cases:
        test_tokenizers(prompt, expected)
        print("\n")
    
    print("=" * 60)
    print("結論:")
    print("- NovelAIが使用しているトークナイザーを特定する必要がある")
    print("- 重み構文（::）がトークンカウントに影響している可能性")
    print("- 公式ドキュメントまたはブラウザのJSコードを確認すると良い")
