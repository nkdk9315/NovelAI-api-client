# NovelAI API プロトコル仕様

非公式APIのため、コードから抽出した仕様をまとめたもの。移植・互換実装の参考用。

## 認証

全エンドポイントで Bearer トークン認証:

```
Authorization: Bearer <api_key>
```

## エンドポイント一覧

| エンドポイント | メソッド | Content-Type (リクエスト) | レスポンス形式 | 用途 |
|---------------|---------|--------------------------|---------------|------|
| `https://image.novelai.net/ai/generate-image` | POST | `application/json` | ZIP (PNGエントリ) | 画像生成 (通常) |
| `https://image.novelai.net/ai/generate-image-stream` | POST | `application/json` | msgpack stream | 画像生成 (img2img/charref/infill) |
| `https://image.novelai.net/ai/encode-vibe` | POST | `application/json` | バイナリ (encoding) | Vibeエンコード |
| `https://image.novelai.net/ai/augment-image` | POST | `application/json` | ZIP (PNGエントリ) | 画像加工 |
| `https://api.novelai.net/ai/upscale` | POST | `application/json` | ZIP or raw image | アップスケール |
| `https://api.novelai.net/user/subscription` | GET | — | JSON | サブスクリプション情報 |

環境変数でURL上書き可能 (`NOVELAI_API_URL`, `NOVELAI_STREAM_URL` 等)。

---

## generate ペイロード

### エンドポイント選択条件

- `character_reference` が指定されている → `STREAM_URL`
- `action === "infill"` → `STREAM_URL`
- `action === "img2img"` → `STREAM_URL`
- それ以外 → `API_URL`

### ペイロード構造 (GenerationPayload)

```json
{
  "input": "<prompt>",
  "model": "nai-diffusion-4-5-full",
  "action": "generate",
  "parameters": { ... },
  "use_new_shared_trial": true
}
```

### parameters オブジェクト (基本)

```json
{
  "params_version": 3,
  "width": 832,
  "height": 1216,
  "scale": 5.0,
  "sampler": "k_euler_ancestral",
  "steps": 23,
  "n_samples": 1,
  "ucPreset": 0,
  "qualityToggle": false,
  "autoSmea": false,
  "dynamic_thresholding": false,
  "controlnet_strength": 1,
  "legacy": false,
  "add_original_image": true,
  "cfg_rescale": 0,
  "noise_schedule": "karras",
  "legacy_v3_extend": false,
  "skip_cfg_above_sigma": null,
  "use_coords": false,
  "legacy_uc": false,
  "normalize_reference_strength_multiple": true,
  "inpaintImg2ImgStrength": 1,
  "seed": 1234567890,
  "negative_prompt": "<negative_prompt>",
  "deliberate_euler_ancestral_bug": false,
  "prefer_brownian": true
}
```

### v4_prompt 構造

全リクエストに付与される V4 プロンプト構造。`use_coords` はキャラクター設定の有無に応じて動的に設定される (`characters` が存在する場合は `true`、それ以外は `false`):

```json
{
  "v4_prompt": {
    "caption": {
      "base_caption": "<prompt>",
      "char_captions": [
        {
          "char_caption": "<character_prompt>",
          "centers": [{ "x": 0.5, "y": 0.5 }]
        }
      ]
    },
    "use_coords": false,
    "use_order": true
  },
  "v4_negative_prompt": {
    "caption": {
      "base_caption": "<negative_prompt>",
      "char_captions": [
        {
          "char_caption": "<character_negative_prompt>",
          "centers": [{ "x": 0.5, "y": 0.5 }]
        }
      ]
    },
    "legacy_uc": false
  }
}
```

### characterPrompts (use_coords)

キャラクター設定がある場合に追加:

```json
{
  "use_coords": true,
  "characterPrompts": [
    {
      "prompt": "<character_prompt>",
      "uc": "<character_negative_prompt>",
      "center": { "x": 0.5, "y": 0.5 },
      "enabled": true
    }
  ]
}
```

---

## stream vs 非stream の使い分け

| 条件 | エンドポイント | レスポンス形式 |
|------|--------------|---------------|
| `character_reference` あり | `generate-image-stream` | msgpack |
| `action === "infill"` | `generate-image-stream` | msgpack |
| `action === "img2img"` | `generate-image-stream` | msgpack |
| それ以外 | `generate-image` | ZIP |

stream エンドポイント使用時は以下が追加される:
```json
{
  "stream": "msgpack",
  "image_format": "png"
}
```

---

## img2img 固有パラメータ

`action: "img2img"` 時に parameters に追加:

```json
{
  "image": "<source_image_base64>",
  "strength": 0.62,
  "noise": 0.0,
  "extra_noise_seed": 1234567889,
  "stream": "msgpack",
  "image_format": "png"
}
```

- `extra_noise_seed`: `seed === 0` の場合は `MAX_SEED (4294967295)`、それ以外は `seed - 1`

### ソース画像リサイズ

ソース画像はターゲットサイズ (`width` x `height`) にリサイズしてから送信する:
```
入力画像: 任意サイズ → リサイズ後: width x height (fill フィット)
```
サーバーは `width`/`height` パラメータと画像サイズの不整合を許容しないため、クライアント側でリサイズが必要。

> **言語別実装**: ts-api は `sharp`、rust-api は `image` クレート (Lanczos3)、swift-api は `CoreGraphics` を使用。

---

## inpaint (infill) 固有仕様

### モデル名サフィックス

infill時、モデル名に `-inpainting` を自動付与:
```
nai-diffusion-4-5-full → nai-diffusion-4-5-full-inpainting
```
既に付与済みの場合はスキップ。

### マスクリサイズ

マスク画像は元画像の **1/8サイズ** にリサイズ:
```
元画像: 832x1216 → マスク: 104x152
```
リサイズ処理: 画像処理ライブラリで fill フィット + グレースケール化 + PNG出力

### ソース画像リサイズ

ソース画像はターゲットサイズ (`width` x `height`) にリサイズしてから送信する（img2imgと同様）:
```
入力画像: 任意サイズ → リサイズ後: width x height (fill フィット)
```
サーバーは `width`/`height` パラメータと画像サイズの不整合を許容しないため。

> **言語別実装**: ts-api は `sharp`、rust-api は `image` クレート (Lanczos3)、swift-api は `CoreGraphics` を使用。

### cache_secret_key

元画像・マスクそれぞれの SHA256 ハッシュを送信:

```json
{
  "image": "<source_image_base64>",
  "mask": "<resized_mask_base64>",
  "strength": 0.45,
  "noise": 0,
  "add_original_image": false,
  "extra_noise_seed": 1234567889,
  "inpaintImg2ImgStrength": 0.68,
  "img2img": {
    "strength": 0.68,
    "color_correct": true
  },
  "image_cache_secret_key": "<sha256_of_source_image>",
  "mask_cache_secret_key": "<sha256_of_resized_mask>",
  "image_format": "png",
  "stream": "msgpack"
}
```

### inpaint パラメータの関係

| パラメータ | ペイロードフィールド | 説明 |
|-----------|---------------------|------|
| `mask_strength` | `inpaintImg2ImgStrength`, `img2img.strength` | マスク反映度 |
| `hybrid_img2img_strength` | `strength` | 元画像維持率 (未指定時は mask_strength) |
| `hybrid_img2img_noise` | `noise` | ノイズ (未指定時は 0) |
| `inpaint_color_correct` | `img2img.color_correct` | 色補正 |

---

## Character Reference 固有仕様

### 画像リサイズルール

参照画像はアスペクト比に応じて以下のサイズにリサイズ:

| 条件 | ターゲットサイズ |
|------|-----------------|
| アスペクト比 < 0.8 (縦長) | 1024 x 1536 |
| アスペクト比 > 1.25 (横長) | 1536 x 1024 |
| その他 (正方形付近) | 1472 x 1472 |

リサイズ方法: contain フィット (アスペクト比維持 + 黒パディング)

> **言語別実装**: ts-api は `sharp` の contain、rust-api は `image` クレートの resize + 黒キャンバス、swift-api は `CoreGraphics` を使用。

### director_reference パラメータ

```json
{
  "director_reference_images": ["<resized_image_base64>"],
  "director_reference_descriptions": [
    {
      "caption": {
        "base_caption": "character&style",
        "char_captions": []
      },
      "legacy_uc": false
    }
  ],
  "director_reference_information_extracted": [1.0],
  "director_reference_strength_values": [0.6],
  "director_reference_secondary_strength_values": [0.0],
  "stream": "msgpack",
  "image_format": "png"
}
```

- `base_caption`: mode の値 (`"character"` / `"character&style"` / `"style"`)
- `information_extracted`: 常に `1.0`
- `strength_values`: ユーザー指定の `strength`
- `secondary_strength_values`: `1.0 - fidelity`

---

## Vibe Transfer パラメータ

```json
{
  "reference_image_multiple": ["<vibe_encoding_base64>", ...],
  "reference_strength_multiple": [0.7, ...],
  "reference_information_extracted_multiple": [0.7, ...],
  "normalize_reference_strength_multiple": true
}
```

---

## augment ペイロード

```json
{
  "req_type": "colorize",
  "use_new_shared_trial": true,
  "width": 832,
  "height": 1216,
  "image": "<image_base64>"
}
```

`colorize` の場合:
```json
{
  "prompt": "vibrant colors",
  "defry": 3
}
```

`emotion` の場合:
```json
{
  "prompt": "happy;;",
  "defry": 0
}
```

注意: emotion の prompt にはコード内で `;;` が自動付与される。

### 入力画像の解像度チェック

Augment 入力画像はピクセル数上限 (`MAX_PIXELS`) を超えてはならない。超過した場合はバリデーションエラー。

---

## upscale ペイロード

```json
{
  "image": "<image_base64>",
  "width": 832,
  "height": 1216,
  "scale": 4
}
```

レスポンスは ZIP または raw image バイナリ。ZIP シグネチャ (`PK`) の有無で判別。

### 入力画像の解像度チェック

Upscale 入力画像はピクセル数上限 (`UPSCALE_MAX_PIXELS = 1,048,576`、1024x1024相当) を超えてはならない。超過した場合は API が 400 エラーを返すため、クライアント側でバリデーションする。

---

## encode-vibe ペイロード

```json
{
  "image": "<image_base64>",
  "information_extracted": 0.7,
  "model": "nai-diffusion-4-5-full"
}
```

リクエストヘッダ:
```
Accept: */*
```

レスポンス: バイナリデータ (encoding)。Base64エンコードして `.naiv4vibe` ファイルに保存。

---

## subscription レスポンス

```json
{
  "trainingStepsLeft": {
    "fixedTrainingStepsLeft": 10000,
    "purchasedTrainingSteps": 5000
  },
  "tier": 3
}
```

| tier値 | プラン |
|--------|--------|
| 0 | Free |
| 1 | Tablet |
| 2 | Scroll |
| 3 | Opus |

---

## レスポンスパース詳細

### ZIP形式の判定とパース

```
バイト[0..1] === 0x50 0x4b ("PK") → ZIP
```

処理:
1. ZIP ライブラリで展開
2. エントリ数チェック (<= 10)
3. 拡張子 `.png` / `.webp` / `.jpg` / `.jpeg` のエントリを検索
4. 展開前にサイズチェック: `header.size <= 50MB`
5. 圧縮比チェック: `size / compressedSize <= 100`
6. エントリデータを返却

### stream レスポンスのパース

ZIP でも先頭 PNG でもない場合、以下の順でパースを試行する:

#### フレーム化 msgpack (4バイト BE 長プレフィックス)

stream エンドポイントは長さプレフィックス付きバイナリフレームの連続として応答する場合がある:

```
[4バイト BE 長] [msgpack データ] [4バイト BE 長] [msgpack データ] ...
```

処理:
1. 先頭4バイトをビッグエンディアンの `uint32` として読み取り、フレーム長とする
2. フレーム長分の msgpack データをデコード
3. `event_type === "error"` のフレームがあればエラーとして即座に例外
4. 各フレームの `data` または `image` フィールドからバイナリを取得
5. **最終フレームを優先** (最終フレームがフル解像度画像、途中フレームはプレビュー)

#### 埋め込み PNG 検索

PNGマジックバイト (`89 50 4E 47 0D 0A 1A 0A`) を検索し、IENDチャンク (`49 45 4E 44`) + 4バイト CRC まで切り出す。

> Swift/Rust では末尾から検索 (`extractLastPNG` / `rfind_subsequence`) してフル解像度画像を優先する。

#### Raw msgpack パース (後方互換フォールバック)

フレーム処理なしで msgpack ライブラリで直接パースを試行する。`data` または `image` フィールドからバイナリを取得。

> **言語別実装**: ts-api は `msgpackr`、rust-api は `rmpv`、swift-api は `msgpack-swift` を使用。

> **注意**: TS版では raw msgpack → 埋め込みPNG の順、Swift/Rust版では 埋め込みPNG → raw msgpack の順。いずれもフレーム化msgpackが最優先。

---

## レートリミット対策

### リトライ条件

| HTTPステータス | リトライ | 説明 |
|---------------|---------|------|
| 429 | する | Too Many Requests / Concurrent generation locked |
| ネットワークエラー | する | タイムアウト, 接続不可, DNS解決失敗 |
| その他 (400, 401, 500等) | しない | 即座にエラー |

> **注意**: rust-api のみ 502/503 もリトライ対象に含む。

### Exponential Backoff 計算式

```
retryDelay = round(baseRetryDelay * 2^attempt * (1 + random() * 0.3))
```

| パラメータ | 値 |
|-----------|-----|
| `baseRetryDelay` | 1000ms |
| `maxRetries` | 3 |
| jitter | 0〜30% |

実際の待機時間 (jitter除く):
- 1回目: 1000ms
- 2回目: 2000ms
- 3回目: 4000ms

### リクエストタイムアウト

- 60秒のタイムアウト (`DEFAULT_REQUEST_TIMEOUT_MS`)
- タイムアウトした場合はネットワークエラーとしてリトライ対象
