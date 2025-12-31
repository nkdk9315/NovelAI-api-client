"""
NovelAI Token Count API テスト

トークンカウントAPIの動作確認用スクリプト
"""

import os
import httpx
import json

# .envファイルから環境変数を読み込む
from dotenv import load_dotenv
load_dotenv()

# 複数のベースURLを試す
BASE_URLS = [
    "https://api.novelai.net",
    "https://image.novelai.net",
]

ENDPOINT = "/oa/v1/internal/token-count"


def test_token_count(prompt: str, api_key: str | None = None):
    """トークンカウントAPIをテスト"""
    api_key = api_key or os.environ.get("NOVELAI_API_KEY")
    if not api_key:
        raise ValueError("NOVELAI_API_KEY environment variable is required")
    
    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
        "Accept": "application/json",
    }
    
    # Swagger仕様に基づくリクエストボディ
    payload = {
        "prompt": {"ofString": prompt},
        "model": "nai-diffusion-4-5-full"
    }
    
    print(f"Testing prompt: {prompt[:50]}...")
    print(f"Payload: {json.dumps(payload, indent=2)}")
    print()
    
    for base_url in BASE_URLS:
        url = f"{base_url}{ENDPOINT}"
        print(f"Trying: {url}")
        
        try:
            with httpx.Client(timeout=30.0) as client:
                response = client.post(url, json=payload, headers=headers)
                
                print(f"  Status: {response.status_code}")
                print(f"  Headers: {dict(response.headers)}")
                
                if response.status_code == 200:
                    print(f"  ✅ SUCCESS!")
                    print(f"  Response: {response.text}")
                    return response.json()
                else:
                    print(f"  ❌ Error: {response.text[:200]}")
                    
        except Exception as e:
            print(f"  ❌ Exception: {e}")
        
        print()
    
    # 別のリクエスト形式も試す
    print("=" * 50)
    print("Trying alternative payload format...")
    
    alternative_payloads = [
        # シンプルな形式
        {"prompt": prompt},
        # 直接文字列
        {"text": prompt},
        # model指定あり
        {"prompt": prompt, "model": "nai-diffusion-4-5-full"},
    ]
    
    for i, payload in enumerate(alternative_payloads):
        print(f"\nAlternative {i+1}: {json.dumps(payload)[:100]}")
        
        for base_url in BASE_URLS:
            url = f"{base_url}{ENDPOINT}"
            
            try:
                with httpx.Client(timeout=30.0) as client:
                    response = client.post(url, json=payload, headers=headers)
                    
                    if response.status_code == 200:
                        print(f"  ✅ SUCCESS at {base_url}!")
                        print(f"  Response: {response.text}")
                        return response.json()
                    else:
                        print(f"  {base_url}: {response.status_code}")
                        
            except Exception as e:
                print(f"  {base_url}: Exception - {e}")
    
    return None


if __name__ == "__main__":
    # テスト用プロンプト
    test_prompts = [
        "1girl, beautiful anime girl, detailed eyes",
        "masterpiece, best quality, 1girl, long blonde hair, blue eyes, school uniform, sitting in classroom, looking at viewer, smile",
    ]
    
    for prompt in test_prompts:
        print("=" * 60)
        result = test_token_count(prompt)
        print()
        
        if result:
            print(f"Final Result: {result}")
            break
