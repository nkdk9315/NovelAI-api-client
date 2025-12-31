# NovelAI Client パッケージ

NovelAI の画像生成 API を簡単に使えるPythonクライアントです。

## 📦 インストール

```bash
cd api
uv add httpx python-dotenv pydantic pillow msgpack
```

## 🔑 セットアップ

`.env` ファイルにAPIキーを設定:

```env
NOVELAI_API_KEY=pst-xxxxxxxxxxxxxxxxx
```

## 📖 基本的な使い方

```python
from dotenv import load_dotenv
load_dotenv()

from novelai import NovelAIClient

client = NovelAIClient()
```

---

## 🎨 画像生成

### シンプルな生成

```python
result = client.generate(
    prompt="1girl, beautiful anime girl, detailed eyes, masterpiece",
    width=832,
    height=1216,
)

# 画像を保存
result.save("output.png")
print(f"シード値: {result.seed}")
print(f"残りアンラス: {result.anlas_remaining}")
```

### 自動保存

```python
# ファイル名を指定
result = client.generate(
    prompt="1girl, beautiful",
    save_path="my_image.png",  
)

# フォルダを指定（自動ファイル名）
result = client.generate(
    prompt="1girl, beautiful",
    save_dir="output/",  # gen_20231229_161500_12345678.png のようなファイル名
)
```

---

## 🎭 マルチキャラクター

```python
from novelai import NovelAIClient, CharacterConfig

client = NovelAIClient()

characters = [
    CharacterConfig(
        prompt="1girl, blonde hair, blue eyes, school uniform",
        center_x=0.3,  # 左側に配置
        center_y=0.5,
    ),
    CharacterConfig(
        prompt="1boy, black hair, brown eyes, school uniform", 
        center_x=0.7,  # 右側に配置
        center_y=0.5,
    ),
]

result = client.generate(
    prompt="school classroom, wide shot, sunny day",  # 背景プロンプト
    characters=characters,
    save_path="multi_character.png",
)
```

---

## 🖼️ Image2Image (img2img)

```python
result = client.generate(
    prompt="1girl, anime style, detailed",
    action="img2img",
    source_image="input.png",  # 入力画像
    img2img_strength=0.6,      # 変換強度 (0.0-1.0, 低いほど元画像に近い)
    save_path="img2img_output.png",
)
```

---

## 🎵 Vibe Transfer

### .naiv4vibe ファイルを使用

```python
result = client.generate(
    prompt="1girl, beautiful",
    vibes=["style.naiv4vibe"],
    vibe_strengths=[0.7],
    save_path="vibe_output.png",
)
```

### 複数のVibe

```python
result = client.generate(
    prompt="1girl, beautiful",
    vibes=["style1.naiv4vibe", "style2.naiv4vibe"],
    vibe_strengths=[0.5, 0.3],  # 各Vibeの強度
    save_path="multi_vibe_output.png",
)
```

### 画像からVibeをエンコード（2 Anlas消費）

```python
# 画像からVibeを作成
vibe_result = client.encode_vibe(
    "reference_image.png",
    information_extracted=0.7,
    strength=0.7,
    save_path="my_vibe.naiv4vibe",  # ファイルとして保存
)

# 作成したVibeを使用
result = client.generate(
    prompt="1girl, beautiful",
    vibes=[vibe_result],
    vibe_strengths=[0.7],
)
```

---

## 👤 キャラクター参照 (Character Reference)

参照画像からキャラクターの特徴を抽出:

```python
from novelai import NovelAIClient, CharacterReferenceConfig

client = NovelAIClient()

result = client.generate(
    prompt="1girl, standing, full body",
    character_reference=CharacterReferenceConfig(
        image="reference.png",
        fidelity=1.0,        # 忠実度 (0.0-1.0)
        include_style=True,  # 絵柄も参照するか
    ),
    save_path="charref_output.png",
)
```

> ⚠️ **注意**: キャラクター参照とVibe Transferは同時に使用できません

---

## 💰 アンラス残高確認

```python
balance = client.get_anlas_balance()
print(f"固定アンラス: {balance['fixed']}")
print(f"購入アンラス: {balance['purchased']}")
print(f"合計: {balance['total']}")
print(f"プラン: {['なし', 'Tablet', 'Scroll', 'Opus'][balance['tier']]}")
```

---

## ⚙️ 生成パラメータ

| パラメータ | デフォルト | 説明 |
|-----------|-----------|------|
| `model` | `"nai-diffusion-4-5-full"` | モデル名 |
| `width` | `832` | 画像幅（64の倍数） |
| `height` | `1216` | 画像高さ（64の倍数） |
| `steps` | `23` | 生成ステップ数 (1-50) |
| `scale` | `5.0` | CFG Scale (0.0-10.0) |
| `seed` | `None` | シード値（Noneでランダム） |
| `sampler` | `"k_euler_ancestral"` | サンプラー |
| `noise_schedule` | `"karras"` | ノイズスケジュール |
| `negative_prompt` | デフォルトネガティブ | ネガティブプロンプト |

### 有効なモデル

- `nai-diffusion-4-curated-preview`
- `nai-diffusion-4-full`
- `nai-diffusion-4-5-curated`
- `nai-diffusion-4-5-full` ← 推奨

### 有効なサンプラー

- `k_euler`
- `k_euler_ancestral` ← デフォルト
- `k_dpmpp_2s_ancestral`
- `k_dpmpp_2m_sde`
- `k_dpmpp_2m`
- `k_dpmpp_sde`

---

## ⚠️ 制限事項

- **ピクセル数**: `width × height ≤ 1,048,576`（例: 832×1216 = 1,011,712 ✓）
- **サイズ**: 64の倍数
- **プロンプト**: 最大2000文字
- **キャラクター**: 最大6体
- **Vibe**: 最大10個（5個以上は1Vibeあたり2Anlas消費）
- **ステップ数**: 1-50
- **CFG Scale**: 0.0-10.0

---

## 📁 GenerateResult の使い方

```python
result = client.generate(prompt="1girl")

# 属性
result.image_data      # bytes: 画像のバイトデータ
result.seed            # int: 使用されたシード値
result.anlas_remaining # int: 残りアンラス
result.anlas_consumed  # int: 消費したアンラス
result.saved_path      # Path: 保存先パス（保存済みの場合）

# メソッド
result.save("output.png")  # 画像を保存
```

---

## 🔧 高度な使い方

### フルカスタマイズ

```python
result = client.generate(
    prompt="1girl, beautiful anime girl, detailed eyes",
    negative_prompt="lowres, bad quality, blurry",
    model="nai-diffusion-4-5-full",
    width=1024,
    height=1024,
    steps=28,
    scale=6.0,
    seed=12345678,
    sampler="k_dpmpp_2m_sde",
    noise_schedule="karras",
    save_dir="output/",
)
```

### バリデーションエラーのキャッチ

```python
from pydantic import ValidationError

try:
    result = client.generate(
        prompt="1girl",
        width=100,  # 64の倍数でない → エラー
    )
except ValidationError as e:
    print(f"バリデーションエラー: {e}")
```
