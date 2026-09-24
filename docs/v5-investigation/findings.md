# NovelAI V5 調査結果 (2026-09-24)

ローカル環境で以下の3つの情報源から V5 時代の画像生成 API を調べた結果。

1. 公式 OpenAPI 定義 `https://image.novelai.net/docs/doc.json` → [docs/official/doc.json](../official/doc.json)
2. novelai.net/image での実際の通信 (Claude in Chrome で `window.fetch` をフックして取得) → [web-captures-2026-09-24.json](web-captures-2026-09-24.json)
3. 公式サイトの JS バンドル (`_app-8f8e25e0af037617.js`) の該当ロジック
4. API キー (永続トークン) で `/ai/generate-image` に **公式ドキュメント通りの JSON ボディ** を直接投げた検証 → [probe_v5.py](probe_v5.py)

> HAR ファイルは取っていない。代わりに fetch フックでリクエスト (ヘッダ・multipart 各パート・JSON) とストリームのフレーム構造を記録した。認証ヘッダ・プロンプト本文は除去済み。

---

## 結論

| 機能 | 公式ドキュメント (doc.json) | 公式サイトの挙動 | API キー + JSON で直接実行 |
|---|---|---|---|
| txt2img (V5) | ✅ `action`, `parameters` | multipart + stream | ✅ 動作 (0 Anlas, 512x768) |
| img2img (V5) | ✅ `image`, `strength`, `noise`, `color_correct` はスキーマに記載 | `action:"img2img"` | ✅ 動作 |
| inpaint (V5) | ✅ `mask`, `img2img` ("used by inpaint") | `action:"infill"`, モデル `*-inpainting` | ✅ 動作 (マスク部分のみ変化を確認) |
| 透過 (V5) | ✅ `straight_alpha`, `tag_hint_transparent_background` | 透過トグル → プロンプトに `transparent background` を追加 | ✅ RGBA PNG (約71%のピクセルが alpha=0) |
| Vibe Transfer (V5) | スキーマ上は存在 | **V5 では UI 無効** (`vibetransfer:false`) | ❌ 500 Internal Server Error |
| Character Reference (V5) | `director_reference_*` はスキーマに記載 | **V5 では UI 無効** (`characterReferences:false`) | ❌ 400 (encode-director の接続先が無効) |
| Augment | ✅ `req_type` 等 | multipart + ZIP | ✅ `declutter-keep-bubbles` も動作 |
| Upscale | ✅ `/ai/upscale` (`model` 必須) | `{image, model:"nai-diffusion-5-curated", declared_blur_sigma:0}` | ✅ 2倍, 1 Anlas (`api.novelai.net` は 404) |

> 2回目の調査 (§7〜§12) で、**既存の3言語クライアントは V4.5 でも img2img / infill / vibe / charref / upscale が壊れている**ことが分かった。原因は PR #30 の multipart 送信 (§7)。

**img2img・inpaint・透過は、いずれも公式ドキュメントに載っているフィールドと JSON ボディだけで V5 でも動く。リバースエンジニアリングは必須ではない。**

公式ドキュメントに書かれていないのは次の点で、ここは通信の観察で補う必要がある。
- `action` の値 (`generate` / `img2img` / `infill`)。スキーマは `string` としか書いていない
- inpaint 用モデル名 (`nai-diffusion-5-full-inpainting` など)
- V5 の品質タグ・UC プリセットの中身
- ストリーム応答の形式 (doc.json は SSE と書いているが、`stream:"msgpack"` にすると長さ付き msgpack が返る)

---

## 1. 公式 OpenAPI (doc.json) の要点

- 前回保存していたものと中身は同じ (整形のみ違う)。Swagger 2.0 で、タイトルは "Omegalaser API"。
- 画像関連のエンドポイントは `/ai/generate-image`, `/ai/generate-image-stream`, `/ai/augment-image`, `/ai/upscale`, `/ai/encode-vibe`, `/ai/generate-image/suggest-tags`, `/user/subscription`。
- `image.RequestParameters` に次が**公式に**載っている。
  - img2img/inpaint: `image`, `mask`, `strength`, `noise`, `color_correct`, `extra_noise_seed`, `img2img` (`Img2ImgParams`: strength/noise/color_correct/extra_noise_seed, "used by inpaint"), `add_original_image`
  - 透過: `straight_alpha`, `tag_hint_transparent_background` (説明は "Pure pass-through hint for the model … Omegalaser does not interpret it")
  - プリセット: `tag_hint_qt`, `tag_hint_uc_preset`, `ucPreset`, `qualityToggle`
  - 出力: `image_format` (`png` / `webp`), `stream` (`msgpack` / `sse`)
  - CharRef: `director_reference_images` / `_descriptions` / `_information_extracted` / `_strength_values` / `_secondary_strength_values`
  - Vibe: `reference_image_multiple`, `reference_information_extracted_multiple`, `reference_strength_multiple`
  - `upscale.declared_blur_sigma`
- `/ai/generate-image` は `Accept: application/json` を付けると `{"images":[{"image":base64,"index","seed"}]}` が返る (201 と書かれているが実際は 200)。
- `UpscaleRequest.image` は "can be given as base64 or as multipart pointer"。multipart は公式に認められた送り方。
- 利用規約上の注意: 「生成リクエストは必ず人間の操作で開始すること。過負荷になる自動化は禁止」と明記されている。

## 2. 公式サイトの通信 (V5)

詳しくは [web-captures-2026-09-24.json](web-captures-2026-09-24.json)。

### 共通
- エンドポイントは常に `POST https://image.novelai.net/ai/generate-image-stream`
- ヘッダ: `Authorization: Bearer <token>`, `x-correlation-id` (英数字6文字), `x-initiated-at` (ISO8601)
- ボディ: `multipart/form-data`
  - `request` パート (filename `blob`, `application/json`) に JSON 全体
  - 画像類は別パートの PNG バイナリで送り、JSON 側の値にはパート名を入れる (`"image":"image"`, `"mask":"mask"`, `ref_multiple_N`, `director_ref_N`)
- トップレベル: `input`, `model`, `action`, `parameters`, `use_new_shared_trial: true`
- V5 のパラメータ:
  - `params_version: 4` (4.5 時代は 3)
  - `ucPresetId` / `qualityPresetId` (文字列 ID)。旧 `ucPreset` / `qualityToggle` は送っていない
  - `tag_hint_qt` (品質プリセットの数値ID: light=3), `tag_hint_uc_preset` (none=0)
  - `straight_alpha: true`, `tag_hint_transparent_background: true` (透過トグル ON のとき)
  - `image_format: "webp"` (サイトは常に webp。設定で png にするとクライアント側で変換)
  - `noise_schedule` は V5 だと常に `karras` に固定される
  - `stream: "msgpack"`
  - `characterPrompts` / `v4_prompt` / `v4_negative_prompt` の形は 4.5 と同じ

### action ごとの違い
| 操作 | model | action | 追加パラメータ |
|---|---|---|---|
| txt2img | `nai-diffusion-5-full` | `generate` | — |
| img2img | `nai-diffusion-5-full` | `img2img` | `image`, `strength`(0.7), `noise`(0), `extra_noise_seed`, `color_correct:false` |
| inpaint | `nai-diffusion-5-full-inpainting` | `infill` | `image`, `mask`, `strength`, `noise`, `extra_noise_seed`, `add_original_image:false`, `inpaintImg2ImgStrength` |
| augment | — | (`/ai/augment-image`) | `req_type`, `width`, `height`, `image`, `use_new_shared_trial` |

### 画像キャッシュ (`*_cache_secret_key`)
- JS では `e6()` がセッションごとに `crypto.getRandomValues(32)` で作った鍵を使い、**base64 画像文字列を HMAC-SHA256** したものをキーにしている。**画像のハッシュではない。**
- 初めて送る画像は「本体 + キー」、同じセッションで2回目以降は「キーだけ」を送る。
- サーバーが `400 {"message":"INVALID_CACHE_KEYS","details":{"invalidKeys":[...]}}` を返したら、本体を付けて再送する。
- **省略可能な最適化**。API キーから JSON で送るときはキーを付けずに本体だけ送れば動く (検証済み)。
  - 既存実装 (`ts-api/src/utils.ts` の「SHA256 で cache_secret_key」) は仕組みを取り違えている。害はないが、不要なら外してよい。

### ストリーム応答 (msgpack)
- `Content-Type: application/msgpack`
- `[uint32 BE 長さ][msgpack map]` の繰り返し
  - `event_type:"intermediate"`: `samp_ix`, `step_ix`, `gen_id`, `sigma`, `image` (JPEG プレビュー)
  - `event_type:"final"`: `samp_ix`, `gen_id`, `image` (webp / png 本体)
  - `event_type:"error"`: `message`, `samp_ix`
- img2img は `steps × strength` 回分しか intermediate が来ない (23×0.7 → 16 フレーム)
- 最終画像 (webp) は `VP8X` (alpha + EXIF フラグ) + `VP8L` (ロスレス)。EXIF にメタデータが入っている

### Augment (Director Tools)
- `POST /ai/augment-image`、multipart (`image` パート + `request` パート)、応答は ZIP
- サイトのツール一覧: `bg-removal`, `declutter`, **`declutter-keep-bubbles`** (新, API 経由), `lineart`, `sketch`, `colorize`, `emotion`, `upscale`, **`pixel-snap`** (新, **ブラウザ内処理で API は呼ばない**。§10 参照)

## 3. JS バンドルから分かったこと

### モデル一覧 (V5 関連)
`nai-diffusion-5-curated`, `nai-diffusion-5-full`, `nai-diffusion-5-full-inpainting` (4.5 系・4 系も引き続き存在)。`nai-diffusion-5-curated-inpainting` という名前は JS にあるが**サーバーには存在しない** (§11)

### モデル別の機能フラグ (V5)
```
vibetransfer:false  encodedVibes:false  characterReferences:false  charRefInpainting:false
transparency:true   img2imgInpainting:true  inpainting:true  streamedResponses:true
characterPrompts:true  maxCharacters:32  canPositionOneCharacter:true  freeformCharacterPosition:true
noiseSchedule:false (送信時は karras 固定)  cfgRescale:true  cfgDelay:false  e2eUpscale:false
text:true  autoText:true  maxEnhance:true  opusUsageLimit:true  scaleMax:10
```
4.5 と比べると、`transparency` が true になり、`vibetransfer` / `characterReferences` が false になった。キャラクター数の上限は 6 → 32。

### V5 のデフォルトパラメータ
`params_version:4, width:832, height:1216, scale:7, sampler:k_euler_ancestral, steps:23, strength:0.7, noise:0, noise_schedule:karras, cfg_rescale:0, inpaintImg2ImgStrength:1`

### 品質タグ (V5, プロンプト末尾に追加)
- `standard`: `very aesthetic, masterpiece, no text`
- `light`: `very aesthetic, amazing quality, no text`
- `none`
- 透過 ON のときは先頭に `transparent background` を足す (例: `transparent background, very aesthetic, amazing quality, no text`)

### UC プリセット (V5, ネガティブ先頭に追加)
- `heavy`: `lowres, artistic error, film grain, scan artifacts, worst quality, bad quality, jpeg artifacts, very displeasing, chromatic aberration, dithering, halftone, screentone, multiple views, logo, too many watermarks, negative space, blank page`
- `light`: `lowres, bad hands, bad anatomy, artistic error, sepia, white haze, worst quality, very displeasing, jpeg artifacts, 0::ai-generated::`
- `humanFocus`: heavy + `, @_@, mismatched pupils, glowing eyes, bad anatomy`
- `furryFocus`: (4.5 と同じ furry 用)
- `none`

`tag_hint_qt` / `tag_hint_uc_preset` の数値表: `0 none, 1 standard, 2 heavy, 3 light, 4 humanFocus, 5 furryFocus, 6 lowQualityPlusBadAnatomy, 7 lowQuality, 8 badAnatomy`

### その他
- サイトには `debugLegacyImageGenRequest` 設定があり、ON だと multipart ではなく `Content-Type: application/json` で送る。JSON ボディは現役でサーバーに受け付けられる。
- `altImageEndpoint` 設定で `image-<name>.novelai.net` に切り替えられる。
- 非 stream 版 (`/ai/generate-image`) の応答は ZIP で、エントリ名は `image_N.png` / `image_N.webp`。

## 4. API キーでの直接検証

[probe_v5.py](probe_v5.py) (標準ライブラリのみ) を使い、`POST https://image.novelai.net/ai/generate-image` に JSON ボディで送った。サイズは 512x768、steps 23。

| テスト | 結果 | Anlas |
|---|---|---|
| txt2img, `straight_alpha` + `tag_hint_transparent_background` | 200, ZIP `image_0.png`, RGBA | 9996→9996 |
| txt2img, `Accept: application/json`, `image_format:webp` | 200, `{"images":[{image,index,seed}]}`, webp (alpha あり) | 変化なし |
| img2img (`action:img2img`, base64 `image`, strength 0.7) | 200, RGBA PNG | 変化なし |
| infill (`nai-diffusion-5-full-inpainting`, base64 `image`+`mask`) | 200, RGBA PNG。マスク範囲 (上 1/3) だけ変化 | 変化なし |
| 透過フラグの組み合わせ (なし / hint のみ / straight_alpha のみ) | どれも RGBA で alpha=0 は 71.0% | 変化なし |

分かったこと:
- **透過はプロンプトの `transparent background` で決まる。** V5 は常に RGBA で返してくる。`straight_alpha` は透過領域の RGB の持ち方 (straight か premultiplied か) に効いているだけのようで、出力サイズが少し変わる。`tag_hint_transparent_background` はサーバーでは解釈されない (doc.json の説明どおり)。公式サイトの設定に合わせて3つ全部入れておくのが安全。
- Cloudflare が Python-urllib の既定 User-Agent を弾く (subscription が HTML を返す)。クライアントでは UA を付けておくのが無難。
- マスクは画像と同じサイズの RGBA PNG (白=塗り直す, 黒=残す) で動いた。
- V5 の Opus 無料枠は API キー経由でも効いていた (1024² 以内, steps ≤ 28, 1枚)。


---

# 2回目の調査 (V4.5 の動作確認・使用制限・コスト・augment・トークナイザー)

## 7. 既存クライアントの V4.5 での動作確認

[ts-api/examples/smoke_v45.ts](../../ts-api/examples/smoke_v45.ts) で実 API に対して確認した (512x768, steps 23)。rust / swift は既存の `example_infill` で確認した。

| 操作 | 現状のまま | JSON ボディ + URL 修正 |
|---|---|---|
| getAnlasBalance | ❌ 400 (`api.novelai.net/user/subscription`) | ✅ |
| txt2img | ✅ (残高だけ取れない) | ✅ 0 Anlas |
| img2img | ❌ 400 | ✅ 0 Anlas |
| infill | ❌ 400 | ✅ 0 Anlas |
| vibe (エンコード済み .naiv4vibe) | ❌ 400 | ✅ 0 Anlas |
| charref | ❌ 400 | ✅ 5 Anlas |
| augment (sketch) | ✅ | ✅ 0 Anlas |
| upscale | ❌ 404 (`api.novelai.net/ai/upscale`) | ❌ 400 `model doesn't exist` → §9 の新形式で ✅ |
| encodeVibe | — | ✅ 2 Anlas |

**壊れている原因 (3言語共通):** PR #30 で、JSON を丸ごと multipart の `request` パート1つに入れて送るようにした。multipart で送ると、サーバーは `image` / `mask` / `reference_image_multiple` / `director_reference_images` などの文字列を**別パートの名前への参照**として読む。そのため base64 本体が入っていると次のエラーになる。
```
400 image field references unknown form part "iVBORw0KGgo..."
```
直し方はどちらか。
1. **JSON ボディで送る** (`Content-Type: application/json`)。公式ドキュメントどおりで、stream エンドポイントでも受け付けられる。今回 V4.5 / V5 の全機能で動作を確認した。**おすすめ。**
2. 公式サイトと同じく、画像を別パートにして JSON にはパート名を入れる (`tY()` の実装、§2 参照)。

PR #30 のコメントにある「非 stream 経路だとノイズ状の画像が返る」問題は、JSON ボディで **stream エンドポイント** (`/ai/generate-image-stream`, `stream:"msgpack"`) に送れば起きない。出力画像はどれも正常だった。

その他:
- rust-api のユニットテストが 8 件失敗している (`tests/client_test.rs`)。PR #30 の stream 化・ペイロード変更にモックが追従していない (501 が返る、`add_original_image` の期待値が違う)。ts / swift のテストは全件通る。
- Cloudflare が一部の User-Agent (Python-urllib) を弾く。3言語とも UA を明示しておくと安全。

## 8. V5 の使用制限 (Opus 生成の残量)

- 取得先は `GET https://image.novelai.net/user/subscription` の `usage` だけ。生成レスポンス (ヘッダ・msgpack) には含まれない。
  ```json
  "usage": { "percent": 100, "isNegative": false, "timeUntilNextPercent": 7888 }
  ```
- サイト (chunk `1950-*.js` のモジュール 86321) での解釈:
  - 表示されるのは `tier >= 3` で有効なサブスクのときだけ
  - 残り% = `isNegative ? 0 : max(0, percent)` (バーの表示は 100 で頭打ち)
  - 残りが少ない判定: `isNegative || percent < 5`
  - `timeUntilNextPercent` は **1% 回復するのにかかる秒数** (デバッグ UI での表示名は "SecondsPerPct")
  - 回復速度 (%/日) = `round(86400 / timeUntilNextPercent × 10) / 10` (7888 秒なら 11.0%/日)
  - 残り枚数の目安 = `round(17.3 × percent)` (100% ≈ 1730 枚)
  - 100% のときは回復が止まる ("最大100%まで上限が自動回復します")
- 枠を使い切ると (`isNegative: true`)、V5 の Opus 無料が無効になり、通常どおり Anlas を消費する (§9)。
- 対象は `opusUsageLimit: true` のモデル = V5 だけ。4.5 以前の Opus 無料には上限がない。
- 今回の生成 (V5 を 20 回ほど) では percent は 100 のまま変わらなかった (1% ≈ 17 枚なので妥当)。
- 他に見た値: `perks` から画像生成関連のフラグは消えている (`maxPriorityActions`, `startPriority`, `contextTokens`, `unlimitedMaxPriority`, `moduleTrainingSteps` のみ)。`GET /user/priority` → `{maxPriorityActions, nextRefillAt, taskPriority}`。

## 9. Anlas コスト (現行 JS と実測)

生成コストの関数 (chunk `1601-*.js` の `GI`) は既存の [anlas-cost-calculation.md](../anlas-cost-calculation.md) とほぼ同じで、違いは次の2点。
1. **V5 はコストが ×1.5**: `ceil(2.951823e-6·px + 5.753298e-7·px·steps) × SMEA倍率` の後に `×1.5` し、`max(ceil(w × strength), 2)`
   - 実測 (1088x1024, steps 1): V5 = **6** Anlas (予測 6)、V4.5 = **4** Anlas (予測 4)
2. **V5 の Opus 無料は使用枠が残っているときだけ**: `opusUsageLimit && usage.isNegative` なら、課金枚数から 1 を引く処理をしない
   - Opus 無料の条件そのもの (`!characterRef && W×H ≤ 1048576 && steps ≤ 28`, tier ≥ 3, サブスク有効) は変わらない
- 1枚あたりの上限は 140 Anlas で変わらない。Vibe のバッチコスト `max(0, n-4) × 2` も同じ。

**Upscale のコストは変わっていた** (関数 `tY`、既存ドキュメントの表は古い):
```
入力画素数 ≤ 1,048,576 → 1 / ≤ 1,747,627 → 2 / ≤ 2,446,678 → 3 / ≤ 3,145,728 → 4
```
- Opus 無料はない (512x768 で実測 1 Anlas)。サイトの UI は 1,048,576 px を超える画像のアップスケールを禁止しているので、実質いつも 1 Anlas。
- リクエストは `{image, model:"nai-diffusion-5-curated", declared_blur_sigma:0}`。倍率の指定はなく **常に 2 倍** (512x768 → 1024x1536)。応答は ZIP (`image_0.png`)。
- 既存クライアントの `scale` (2/4) パラメータと `{image,width,height,scale}` のペイロードは使えなくなった (400 `model doesn't exist`)。

Augment のコスト: サイトは V3 モデルとして `GI` を計算している (×1.5 なし)。画像を 3,145,728 − 2000 px 以下に縮小し、1,011,712 px 未満なら 1,048,576 px 近くまで拡大してから送る。既存ドキュメントの内容で問題ない。

JS にある `POST /ai/generate-image/request-price` (サーバー側の見積もり) は、`api.novelai.net` / `image.novelai.net` のどちらでも 404 で使えない。

## 10. Augment に増えた2種類

| ツール | 送り方 | 結果 |
|---|---|---|
| `declutter-keep-bubbles` (デクラッター・吹き出しを残す) | 通常の augment と同じ。`{req_type:"declutter-keep-bubbles", width, height, image}` | ✅ API で動作、ZIP 応答、0 Anlas (Opus) |
| `pixel-snap` (ドット絵お直し) | **API を呼ばない。** ブラウザの Web Worker (chunk 8748) で処理している。コストは常に 0 | API クライアントでは対応不要 (対応するなら自前実装) |

`pixel-snap` の UI の既定値は `{paletteMode:"auto", colors:64, avoidOverRefining:false, upscale:false}`。Worker には `{colors:"auto", autoTol:6.5}` または `{colors: 16〜256 / 0}`、`maxDetail`、`upscale` を渡している。

## 11. トークナイザーとプロンプト上限

- V5 のトークナイザーは **Qwen** (`qwen35_tokenizer.def`)。4.x は T5 で変わらない。
  - URL: `https://novelai.net/tokenizer/compressed/qwen35_tokenizer.def?v=2&static=true` (3.7 MB、raw deflate、展開後 17.5 MB)
  - 中身は JSON: `config.splitRegex` (Qwen 系の pre-tokenize 正規表現)、`specialTokens` (26 個)、`vocab` (248,070)、`merges` (247,587)。**バイトレベル BPE** (GPT-2 系) で、既存の T5 Unigram 実装はそのまま使えない
- サイト UI のトークン上限 (関数 `d$`):
  | モデル | 上限 |
  |---|---|
  | `nai-diffusion-5-full` / `-inpainting` | **1471** |
  | `nai-diffusion-5-curated` | **703** |
  | V4 / V4.5 系 | 512 |
  | それ以前 | 225 |
- 上限を超えてもサイトは「切り詰められます」とトーストを出すだけ。**サーバーは長いプロンプトを拒否しない** (V5 で 17,000 文字、V4.5 で 7,500 文字でも 200)。クライアントの 512 トークン検証は、エラーではなく警告にするか、V5 用の上限にするのがよい。

## 12. V5 でのサーバー挙動 (追加確認)

| 確認内容 | 結果 |
|---|---|
| V5 + Vibe (`reference_image_multiple` に 4.5 のエンコード) | 500 Internal Server Error |
| V5 + CharRef (`director_reference_*`) | 400 `Error encoding v4 director references: ... no such host` |
| `nai-diffusion-5-curated-inpainting` | 400 `model ... doesn't exist` |
| `nai-diffusion-5-curated` で `action:infill` | 400 `doesn't support action infill` (なぜかエラー JSON の後ろに ZIP が続く) |
| V5 生成レスポンスのヘッダ | 使用量の情報はなし |

- V5 curated の inpaint は、サイトでも **`nai-diffusion-4-5-curated-inpainting` に切り替えている** (関数 `u()` のマッピング)。V5 full の inpaint は `nai-diffusion-5-full-inpainting`。
- JSON ボディ + stream エンドポイントの組み合わせも V5 で動作した。

---

## 13. リポジトリの改修 TODO

**優先度 高 (V4.5 でも壊れているもの):**
- generate の送信を **JSON ボディ** (`Content-Type: application/json`) に戻す。エンドポイントは stream のまま (`stream:"msgpack"`)。3言語共通
- `SUBSCRIPTION_URL` → `https://image.novelai.net/user/subscription`
- `UPSCALE_URL` → `https://image.novelai.net/ai/upscale`、ペイロードを `{image, model, declared_blur_sigma}` にする (常に 2 倍)。`scale` パラメータは廃止か 2 固定
- rust-api のテスト 8 件を直す
- User-Agent を明示する

**V5 対応:**
- モデル: `nai-diffusion-5-full`, `nai-diffusion-5-full-inpainting`, `nai-diffusion-5-curated` (curated の inpaint は `nai-diffusion-4-5-curated-inpainting`)
- `params_version: 4`、品質タグ・UC プリセット (§3)、`tag_hint_qt` / `tag_hint_uc_preset`
- 透過: プロンプトに `transparent background` を入れる + `straight_alpha` + `tag_hint_transparent_background`
- `image_format` (png / webp) の選択と webp 応答への対応
- V5 の検証: Vibe / CharRef は使えない (サーバーもエラー)、キャラクター上限 32、noise_schedule は karras 固定
- トークナイザー: V5 用に Qwen (バイトレベル BPE) を追加し、上限を 1471 (full) / 703 (curated) にする。超過は警告にする
- コスト計算: V5 は ×1.5、V5 の Opus 無料は `usage.isNegative` のとき無効、upscale の新しい表
- 使用制限: `getAnlasBalance` (または新メソッド) で `usage` (percent / isNegative / timeUntilNextPercent) を返す。回復速度・残り枚数の目安は §8 の式で計算できる
- Augment: `declutter-keep-bubbles` を追加。`pixel-snap` は API ではないので対象外
- `*_cache_secret_key` は付けない (JSON ボディなら不要)

## 14. まだ分かっていないこと
- 画像アップロード時の「NovelAIポーション」「精密参照」メニューを V5 で選んだときの挙動 (4.5 に切り替える関数はあるが、UI の流れは未確認)
- サイトが作るマスク PNG の正確な形式 (自前の画像と同じサイズの白黒 RGBA では動作を確認済み)
- `usage.percent` が 1 回の生成でどれだけ減るか (サイズ・ステップ数に比例するのか)。1% ≈ 17 枚なので、測るには 20 枚以上の連続生成が必要
- 1 リクエストで複数枚 (`n_samples` > 1) 生成したときの V5 の挙動とコスト
