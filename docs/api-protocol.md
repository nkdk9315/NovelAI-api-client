# NovelAI API プロトコル仕様 (V4.5 / V5)

3言語のクライアントが従う HTTP プロトコルをまとめたもの。移植・互換実装の参考用。

情報源は次の3つ。どれに基づくかを必要に応じて【公式】【観察】で示す。
- 【公式】公式 OpenAPI 定義 `https://image.novelai.net/docs/doc.json` ([docs/official/doc.json](official/doc.json))
- 【観察】公式サイト (novelai.net/image) の通信と JS バンドル、および API キーでの実地検証 (2026-09-24)。詳細は [docs/v5-investigation/findings.md](v5-investigation/findings.md)

> 利用規約により、生成リクエストは人間の操作を起点とする必要があり、システムに過負荷をかける自動化は禁止されている【公式】。

---

## 認証と共通ヘッダ

```
Authorization: Bearer <persistent_api_token>
User-Agent: <クライアント名>/<バージョン>
```

- **User-Agent は明示する。** Cloudflare が一部の既定 UA (Python-urllib など) を弾き、HTML が返ることがある【観察】。
- 任意: `x-correlation-id` (英数字6文字。500 エラー時の問い合わせ用)【公式】、`x-initiated-at` (ISO8601。公式サイトが付けている)【観察】。

## エンドポイント一覧

すべて `image.novelai.net` にある。**`api.novelai.net` の画像系エンドポイントは廃止されている** (`/user/subscription` は 400、`/ai/upscale` は 404)【観察】。

| エンドポイント | メソッド | リクエスト | レスポンス | 用途 |
|---|---|---|---|---|
| `/ai/generate-image-stream` | POST | JSON | msgpack stream | 画像生成 (推奨) |
| `/ai/generate-image` | POST | JSON | ZIP / JSON | 画像生成 (非 stream) |
| `/ai/augment-image` | POST | JSON | ZIP | 画像加工 (Director Tools) |
| `/ai/upscale` | POST | JSON | ZIP | アップスケール (2倍) |
| `/ai/encode-vibe` | POST | JSON | バイナリ | Vibe エンコード |
| `/user/subscription` | GET | — | JSON | ティア・Anlas 残高・V5 使用量 |

ベース URL は `https://image.novelai.net`。環境変数 (`NOVELAI_API_URL`, `NOVELAI_STREAM_URL`, `NOVELAI_ENCODE_URL`, `NOVELAI_AUGMENT_URL`, `NOVELAI_UPSCALE_URL`, `NOVELAI_SUBSCRIPTION_URL`) で個別に上書きできる。

---

## リクエストボディの形式

### JSON ボディ (クライアントはこちらを使う)

```
Content-Type: application/json

{ ...ペイロード... }
```

画像はすべて **base64 文字列** としてペイロードに直接入れる。公式ドキュメントどおりの形式で、V4.5 / V5 の全機能 (txt2img / img2img / infill / vibe / charref / augment / upscale / encode-vibe) で動作を確認した【公式】【観察】。

### multipart/form-data (公式サイトの形式。参考)

公式サイトは次の形で送っている【観察】。
- `request` パート (`filename="blob"`, `Content-Type: application/json`) に JSON 全体
- 画像は別パート (`image/png` のバイナリ) にし、JSON 側の値には**パート名**を書く
  - `image` → パート `image`、`mask` → `mask`、`reference_image` → `reference_image`
  - `reference_image_multiple_cached[i].data` → `ref_multiple_{i}`
  - `director_reference_images_cached[i].data` → `director_ref_{i}`

> **注意:** multipart で送ると、サーバーは画像系フィールドの文字列を「パート名への参照」として解釈する。JSON に base64 本体を入れたまま multipart で送ると `400 image field references unknown form part "iVBOR..."` になる。**以前のクライアント (PR #30) はこの形で送っていて、img2img / infill / vibe / charref が失敗していた。**

---

## generate

### エンドポイント

`POST /ai/generate-image-stream` に `parameters.stream = "msgpack"` を付けて送る。

- 非 stream の `/ai/generate-image` でも同じペイロードで動く (ZIP 応答)。`Accept: application/json` を付けると `{"images":[{"image":<base64>,"index":0,"seed":N}]}` が返る【公式】。
- 以前「非 stream だとノイズ状の画像が返る」事例があったため、クライアントは stream を使う。

### トップレベル

```json
{
  "input": "<prompt>",
  "model": "nai-diffusion-5-full",
  "action": "generate",
  "parameters": { ... },
  "use_new_shared_trial": true
}
```

| action | 用途 | 必要なもの |
|---|---|---|
| `generate` | txt2img | — |
| `img2img` | img2img | `parameters.image` |
| `infill` | inpaint | `parameters.image`, `parameters.mask`、inpainting モデル |

`action` の値は公式ドキュメントに列挙されていない (型は string のみ)【観察】。

### モデル

| モデル | 世代 | inpaint (`infill`) 用モデル |
|---|---|---|
| `nai-diffusion-5-full` | V5 | `nai-diffusion-5-full-inpainting` |
| `nai-diffusion-5-curated` | V5 | **`nai-diffusion-4-5-curated-inpainting`** (V5 curated の inpaint モデルは存在しない) |
| `nai-diffusion-4-5-full` | V4.5 | `nai-diffusion-4-5-full-inpainting` |
| `nai-diffusion-4-5-curated` | V4.5 | `nai-diffusion-4-5-curated-inpainting` |
| `nai-diffusion-4-full` | V4 | `nai-diffusion-4-full-inpainting` |
| `nai-diffusion-4-curated-preview` | V4 | `nai-diffusion-4-curated-inpainting` |

- `nai-diffusion-5-curated-inpainting` は 400 `model ... doesn't exist`、`nai-diffusion-5-curated` + `infill` は 400 `doesn't support action infill`【観察】。
- 公式サイトも V5 curated の inpaint を `nai-diffusion-4-5-curated-inpainting` に置き換えている【観察】。

### V4.5 と V5 の違い

| 項目 | V4.5 | V5 |
|---|---|---|
| `params_version` | 3 (サイトは現在 4 を送る。どちらも通る) | 4 |
| scale の既定 | 5 | 7 |
| `noise_schedule` | karras / exponential / polyexponential | 常に `karras` (サイトが強制) |
| キャラクタープロンプト上限 | 6 | 32 |
| Vibe Transfer | ✅ | ❌ (サーバーが 500) |
| Character Reference | ✅ | ❌ (サーバーが 400) |
| 透過 | ❌ | ✅ |
| トークナイザー / 上限 | T5 / 512 | Qwen / full 1471, curated 703 |
| コスト | 基本式 | 基本式 × 1.5 |
| Opus 無料の上限 | なし | 使用量 (`usage`) の枠内のみ |

### parameters (共通)

```json
{
  "params_version": 4,
  "width": 832,
  "height": 1216,
  "scale": 7,
  "sampler": "k_euler_ancestral",
  "steps": 23,
  "n_samples": 1,
  "seed": 1234567890,
  "extra_noise_seed": 1234567889,
  "noise_schedule": "karras",
  "cfg_rescale": 0,
  "dynamic_thresholding": false,
  "controlnet_strength": 1,
  "legacy": false,
  "add_original_image": true,
  "legacy_v3_extend": false,
  "skip_cfg_above_sigma": null,
  "use_coords": false,
  "legacy_uc": false,
  "normalize_reference_strength_multiple": true,
  "inpaintImg2ImgStrength": 1,
  "deliberate_euler_ancestral_bug": false,
  "prefer_brownian": true,
  "characterPrompts": [],
  "negative_prompt": "<negative_prompt>",
  "v4_prompt": { ... },
  "v4_negative_prompt": { ... },
  "image_format": "png",
  "stream": "msgpack"
}
```

- `extra_noise_seed`: `seed === 0` なら `4294967295`、それ以外は `seed - 1`。
- `image_format`: `png` か `webp`【公式】。公式サイトは常に `webp` (ロスレス、アルファ・EXIF 付き)。クライアントは既定 `png` で、`image_format` / `imageFormat` で選べる。結果の形式は返ってきたバイト列の先頭 (`\x89PNG` / `RIFF....WEBP`) で判定する。
- `k_euler_ancestral` で `noise_schedule` が `native` 以外のとき、サイトは `deliberate_euler_ancestral_bug:false`, `prefer_brownian:true` にする【観察】。
- 公式サイトは V4.5 / V5 とも `ucPresetId` / `qualityPresetId` (文字列) と、`tag_hint_qt` / `tag_hint_uc_preset` (数値) を付ける。サーバーにとっては任意 (付けなくても動く)。旧形式の `ucPreset` / `qualityToggle` も受け付けられる。

### v4_prompt / v4_negative_prompt

```json
{
  "v4_prompt": {
    "caption": {
      "base_caption": "<prompt>",
      "char_captions": [
        { "char_caption": "<character_prompt>", "centers": [{ "x": 0.5, "y": 0.5 }] }
      ]
    },
    "use_coords": true,
    "use_order": true
  },
  "v4_negative_prompt": {
    "caption": {
      "base_caption": "<negative_prompt>",
      "char_captions": [
        { "char_caption": "<character_negative_prompt>", "centers": [{ "x": 0.5, "y": 0.5 }] }
      ]
    },
    "legacy_uc": false
  }
}
```

キャラクターがいるときは `characterPrompts` も付ける (`use_coords` はキャラクター位置を指定するとき `true`)。

```json
{
  "characterPrompts": [
    { "prompt": "<character_prompt>", "uc": "<character_negative_prompt>", "center": { "x": 0.5, "y": 0.5 }, "enabled": true }
  ]
}
```

### 品質タグ・UC プリセット (プロンプトへの追記)

プリセットの中身は公式サイトがプロンプト文字列に直接追記する。サーバーはプロンプトの文字列だけを見る【観察】。

品質タグ (プロンプト末尾に `, ` 区切りで追加):

| モデル | `standard` | `light` |
|---|---|---|
| V5 (full / curated) | `very aesthetic, masterpiece, no text` | `very aesthetic, amazing quality, no text` |
| V4.5 full | `very aesthetic, masterpiece, no text` | — |
| V4.5 curated | `very aesthetic, masterpiece, no text, -0.8::feet::, rating:general` | — |

UC プリセット (V5。ネガティブの先頭に追加):

| ID | 内容 |
|---|---|
| `heavy` | `lowres, artistic error, film grain, scan artifacts, worst quality, bad quality, jpeg artifacts, very displeasing, chromatic aberration, dithering, halftone, screentone, multiple views, logo, too many watermarks, negative space, blank page` |
| `light` | `lowres, bad hands, bad anatomy, artistic error, sepia, white haze, worst quality, very displeasing, jpeg artifacts, 0::ai-generated::` |
| `humanFocus` | `heavy` + `, @_@, mismatched pupils, glowing eyes, bad anatomy` |
| `none` | (なし) |

`tag_hint_qt` / `tag_hint_uc_preset` の数値: `0 none, 1 standard, 2 heavy, 3 light, 4 humanFocus, 5 furryFocus, 6 lowQualityPlusBadAnatomy, 7 lowQuality, 8 badAnatomy`

### 透過背景 (V5 のみ)

**透過はプロンプトに `transparent background` を含めることで決まる。** V5 は常に RGBA で返し、プロンプトがあれば背景のアルファが 0 になる (512x768 で約 71% のピクセルが alpha=0)【観察】。

公式サイトは透過 ON のとき次のようにしている。クライアントも合わせる。
1. 品質タグの前に `transparent background` を足す (例: `..., transparent background, very aesthetic, amazing quality, no text`)
2. `"straight_alpha": true` (透過部分の RGB を straight alpha で持つ。無しだと premultiplied になる)
3. `"tag_hint_transparent_background": true` (サーバーは解釈しない「モデルへのヒント」【公式】)

`straight_alpha` / `tag_hint_transparent_background` だけでは透過にならない (プロンプトが必要)。V4.5 以前ではサイトはこの2つを送らない。

### トークン上限

| モデル | トークナイザー | 上限 |
|---|---|---|
| `nai-diffusion-5-full` (+ inpainting) | Qwen | 1471 |
| `nai-diffusion-5-curated` | Qwen | 703 |
| V4 / V4.5 | T5 | 512 |

- 上限はポジティブ・ネガティブそれぞれに適用される。
- **サーバーは長いプロンプトを拒否しない** (切り詰める)。公式サイトも「切り詰められます」と通知するだけ。
  - このクライアントは、黙って切り詰められるのを防ぐため ts / rust では検証エラーにする。swift は `generate` では数えず、`GenerateParams.validateTokenCounts()` で明示的に確認する。
- 数え方: V5 (Qwen) は生のプロンプトをそのまま数える (重み記法や括弧も含む、EOS なし)。V4.x (T5) は括弧と重み記法を除いてから数え、EOS を含む。
- 定義ファイル: `https://novelai.net/tokenizer/compressed/{name}?v=2&static=true` (raw deflate 圧縮の JSON)
  - `t5_tokenizer.def`: HuggingFace tokenizers 形式の Unigram
  - `qwen35_tokenizer.def`: `{config.splitRegex, specialTokens, vocab, merges}` のバイトレベル BPE (語彙 248,070)
  - `clip_tokenizer.def`: CLIP BPE (augment のプロンプト用)

---

## img2img

`action: "img2img"` のとき parameters に追加する。

```json
{
  "image": "<source_image_base64>",
  "strength": 0.7,
  "noise": 0,
  "color_correct": false
}
```

- 元画像は `width` × `height` にリサイズしてから送る (サイズが合わないとサーバーが受け付けない)。
- 公式サイトの既定は `strength: 0.7`, `noise: 0`。
- ストリームの intermediate は `ceil(steps × strength)` 回分だけ来る。

---

## inpaint (infill)

`action: "infill"` と inpainting モデル (上のモデル表) を使う。

```json
{
  "image": "<source_image_base64>",
  "mask": "<mask_png_base64>",
  "strength": 0.7,
  "noise": 0,
  "add_original_image": false,
  "inpaintImg2ImgStrength": 0.7,
  "img2img": { "strength": 0.7, "color_correct": true }
}
```

- マスクは白 = 塗り直す、黒 = 残す。
- 元画像は `width` × `height` にリサイズ。マスクは元画像の 1/8 (例 832x1216 → 104x152、グレースケール PNG) にリサイズして送る。画像と同じサイズの白黒 PNG でも動作する【観察】。
- `img2img` オブジェクト (`Img2ImgParams`) は公式ドキュメントに "used by inpaint" と書かれている【公式】。

| クライアントのパラメータ | ペイロード |
|---|---|
| `mask_strength` | `inpaintImg2ImgStrength`, `img2img.strength` |
| `hybrid_img2img_strength` | `strength` (未指定時は `mask_strength`) |
| `hybrid_img2img_noise` | `noise` (未指定時は 0) |
| `inpaint_color_correct` | `img2img.color_correct` |

### 画像キャッシュキー (任意・クライアントは使わない)

公式サイトは帯域節約のため `image_cache_secret_key` / `mask_cache_secret_key` (および `reference_image_multiple_cached` / `director_reference_images_cached`) を使う【観察】。
- キーはセッションごとの乱数鍵で base64 文字列を HMAC-SHA256 した値 (画像のハッシュではない)。
- 1回目は本体 + キー、2回目以降はキーだけを送る。サーバーが `400 {"message":"INVALID_CACHE_KEYS","details":{"invalidKeys":[...]}}` を返したら本体付きで再送する。

**付けなくても動くので、クライアントは付けずに毎回本体を送る。**

---

## Vibe Transfer (V4 / V4.5 のみ)

```json
{
  "reference_image_multiple": ["<vibe_encoding_base64>"],
  "reference_strength_multiple": [0.7],
  "reference_information_extracted_multiple": [0.7],
  "normalize_reference_strength_multiple": true
}
```

- エンコード値は `.naiv4vibe` の `encodings[<model_key>][*].encoding` (model_key は `v4full`, `v4curated`, `v4-5full`, `v4-5curated`) か、`/ai/encode-vibe` の結果。
- **V5 では使えない** (500 Internal Server Error)【観察】。
- Character Reference とは同時に使えない。

## Character Reference (V4.5 のみ)

```json
{
  "director_reference_images": ["<resized_image_base64>"],
  "director_reference_descriptions": [
    { "caption": { "base_caption": "character&style", "char_captions": [] }, "legacy_uc": false }
  ],
  "director_reference_information_extracted": [1.0],
  "director_reference_strength_values": [0.6],
  "director_reference_secondary_strength_values": [0.0]
}
```

- 参照画像は縦横比に応じて 1024x1536 (比 < 0.8) / 1536x1024 (比 > 1.25) / 1472x1472 (それ以外) に、比率を保ったまま黒パディングで収める【公式】。
- `base_caption`: `"character"` / `"character&style"` / `"style"`。
- `secondary_strength_values` = `1.0 - fidelity`。
- **V5 では使えない** (400 `Error encoding v4 director references`)【観察】。

---

## レスポンス

### stream (`/ai/generate-image-stream`, `stream: "msgpack"`)

`Content-Type: application/msgpack`。長さ付き msgpack フレームが続く【観察】。

```
[uint32 BE 長さ][msgpack map] [uint32 BE 長さ][msgpack map] ...
```

| `event_type` | フィールド | 内容 |
|---|---|---|
| `intermediate` | `samp_ix`, `step_ix`, `gen_id`, `sigma`, `image` | 途中経過のプレビュー (JPEG) |
| `final` | `samp_ix`, `gen_id`, `image` | 完成画像 (`image_format` に応じて PNG / WEBP) |
| `error` | `message`, `samp_ix` | 生成エラー |

- `final` フレームの `image` を使う。`error` フレームがあれば例外にする。
- 公式ドキュメントは SSE と書いているが、`stream: "sse"` は未検証。
- 使用量などの情報はレスポンスに含まれない。

### 非 stream (`/ai/generate-image`) と augment / upscale

ZIP (`PK` で始まる)。エントリ名は `image_0.png` (webp なら `image_0.webp`)。

### パースの手順 (3言語共通)

1. 先頭が `PK` → ZIP を展開
   - エントリ数 ≤ 10、展開後サイズ ≤ 50MB、圧縮比 ≤ 100 をチェック (ZIP ボム対策)
   - 拡張子 `.png` / `.webp` / `.jpg` / `.jpeg` のエントリを返す
2. 先頭が PNG シグネチャ → そのまま返す
3. 長さ付き msgpack フレームとしてパース → `final` の `image` (なければ最後のフレームの `image` / `data`)
4. フォールバック: 生の msgpack パース、埋め込み PNG の検索 (PNG シグネチャから IEND + CRC まで)

> 実装: ts-api は `msgpackr`、rust-api は `rmpv`、swift-api は `msgpack-swift`。

### エラー

JSON `{"statusCode": N, "message": "..."}`。主なもの:

| ステータス | 例 |
|---|---|
| 400 | `Validation error: ...`, `image field references unknown form part ...`, `INVALID_CACHE_KEYS` |
| 401 | 認証失敗 |
| 402 | Anlas 不足 |
| 429 | 同時生成の制限 |
| 500 | サーバーエラー (V5 に Vibe を送ったときなど) |

---

## augment (`/ai/augment-image`)

```json
{
  "req_type": "colorize",
  "use_new_shared_trial": true,
  "width": 832,
  "height": 1216,
  "image": "<image_base64>",
  "prompt": "vibrant colors",
  "defry": 3
}
```

| `req_type` | prompt | defry | 備考 |
|---|---|---|---|
| `colorize` | 任意 | 必須 (0〜5) | |
| `emotion` | 必須 (`<emotion>;;<追加プロンプト>`) | 必須 (0〜5) | `;;` はクライアントが付ける |
| `declutter` | — | — | |
| `declutter-keep-bubbles` | — | — | 吹き出しを残すデクラッター (V5 世代で追加) |
| `sketch` | — | — | |
| `lineart` | — | — | |
| `bg-removal` | — | — | Opus でも有料 |

- `width` / `height` は入力画像のサイズ。公式サイトは送る前に、3,145,728 − 2000 px を超える画像は縮小し、1,011,712 px 未満なら 1,048,576 px 近くまで拡大している。
- 入力画像のピクセル数が上限 (`MAX_PIXELS` = 3,145,728) を超えるときはクライアントでエラーにする。
- サイトの「ドット絵お直し」(`pixel-snap`) はブラウザ内の処理で、API は呼ばない。

## upscale (`/ai/upscale`)

```json
{
  "image": "<image_base64>",
  "model": "nai-diffusion-5-curated",
  "declared_blur_sigma": 0
}
```

- `model` は必須 (既存の画像モデル名)【公式】。公式サイトは常に `nai-diffusion-5-curated`、`declared_blur_sigma: 0`。
- **倍率の指定はなく、常に 2 倍** (512x768 → 1024x1536)【観察】。以前の `{image, width, height, scale}` は使えない。
- 応答は ZIP (`image_0.png`)。`Accept: application/json` なら base64 の JSON【公式】。
- 入力は 1,048,576 px (1024x1024 相当) 以下にする (公式サイトの制限)。
- `declared_blur_sigma` は `[0.0, 0.30, 0.35, 0.40, 0.45, 0.50]` のどれかに丸める【公式】。

## encode-vibe (`/ai/encode-vibe`)

```json
{
  "image": "<image_base64>",
  "information_extracted": 0.7,
  "model": "nai-diffusion-4-5-full"
}
```

- 応答はバイナリ (エンコード値)。base64 にして `.naiv4vibe` に保存する。
- 任意のフィールド: `mask` (base64), `crop_to_mask`, `focus_seed`, `info_extract_seed`【公式】。
- V5 は Vibe に対応していないので、V4 / V4.5 のモデルを指定する。

---

## subscription (`GET /user/subscription`)

```json
{
  "tier": 3,
  "active": true,
  "trainingStepsLeft": { "fixedTrainingStepsLeft": 0, "purchasedTrainingSteps": 9990 },
  "usage": { "percent": 100, "isNegative": false, "timeUntilNextPercent": 7888 },
  "perks": { ... }
}
```

- Anlas 残高 = `fixedTrainingStepsLeft + purchasedTrainingSteps`。
- tier: `0` Free / `1` Tablet / `2` Scroll / `3` Opus。

### V5 の使用量 (`usage`)

Opus (tier 3) の V5 無料生成には上限があり、時間で回復する【観察】。

| フィールド | 意味 |
|---|---|
| `percent` | 残り (%)。表示は 100 で頭打ち |
| `isNegative` | 使い切った状態。`true` なら残り 0% として扱い、V5 の Opus 無料は無効 (Anlas を消費する) |
| `timeUntilNextPercent` | 1% 回復するのにかかる秒数。0 以下なら回復しない |

公式サイトが出している値の計算式:
- 残り% = `isNegative ? 0 : max(0, percent)`
- 回復速度 (%/日) = `timeUntilNextPercent <= 0 ? 0 : round(86400 / timeUntilNextPercent × 10) / 10`
- 残り枚数の目安 = `round(17.3 × 残り%)` (100% ≈ 1730 枚)
- 残りが少ない判定 = `isNegative || percent < 5`
- 100% のときは回復しない

使用量はこのエンドポイントからしか取れない (生成レスポンスには含まれない)。

---

## コスト

詳細は [anlas-cost-calculation.md](anlas-cost-calculation.md)。このドキュメントに関係する変更点だけ書く。

- **V5 のコスト = V4.5 の式 × 1.5** (切り上げ、最低 2)。例: 1088x1024・steps 1 で V5 = 6、V4.5 = 4 Anlas (実測)。
- **V5 の Opus 無料は `usage.isNegative` が `false` のときだけ**。条件 (`W×H ≤ 1,048,576`, `steps ≤ 28`, CharRef なし, tier 3, サブスク有効) は V4.5 と同じ。
- **upscale**: 入力の画素数で `≤1,048,576 → 1`、`≤1,747,627 → 2`、`≤2,446,678 → 3`、`≤3,145,728 → 4` Anlas。Opus 無料はない。
- CharRef: 1枚あたり 5 Anlas。encode-vibe: 2 Anlas (実測)。

---

## リトライ

| 状況 | リトライ |
|---|---|
| 429 (Too Many Requests / 同時生成の制限) | する |
| ネットワークエラー (タイムアウト、接続不可、DNS) | する |
| その他 (400, 401, 402, 500 など) | しない |

> rust-api のみ 502 / 503 もリトライする。

待ち時間: `round(1000ms × 2^attempt × (1 + random() × 0.3))`、最大 3 回 (1秒 → 2秒 → 4秒 + 0〜30% の揺らぎ)。

リクエストのタイムアウトは 60 秒 (公式サイトは 120 秒)。タイムアウトはネットワークエラーとしてリトライする。
