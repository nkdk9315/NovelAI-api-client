# API リファレンス

## NovelAIClient

### コンストラクタ

```typescript
new NovelAIClient(apiKey?: string, options?: { logger?: Logger })
```

| 引数 | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `apiKey` | `string` | `process.env.NOVELAI_API_KEY` | API認証キー |
| `options.logger` | `Logger` | `console` | ロガー (`warn`, `error` メソッドを持つオブジェクト) |

APIキーが未指定かつ環境変数にもない場合は `Error` をスローする。

---

## generate()

```typescript
async generate(params: GenerateParams): Promise<GenerateResult>
```

画像生成の統合メソッド。txt2img / img2img / inpaint を `action` パラメータで切り替える。

### GenerateParams

#### 基本パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `prompt` | `string` | — (必須) | プロンプト (空文字列可) |
| `negative_prompt` | `string?` | デフォルトネガティブ※ | ネガティブプロンプト |
| `model` | `string` | `"nai-diffusion-4-5-full"` | モデル名 |
| `width` | `number` | `832` | 画像幅 (64の倍数, 64〜2048) |
| `height` | `number` | `1216` | 画像高さ (64の倍数, 64〜2048) |
| `steps` | `number` | `23` | ステップ数 (1〜50) |
| `scale` | `number` | `5.0` | CFGスケール (0.0〜10.0) |
| `cfg_rescale` | `number` | `0` | CFGリスケール (0〜1) |
| `seed` | `number?` | ランダム | シード値 (0〜4294967295) |
| `sampler` | `string` | `"k_euler_ancestral"` | サンプラー |
| `noise_schedule` | `string` | `"karras"` | ノイズスケジュール |

※デフォルトネガティブ: `"nsfw, lowres, artistic error, film grain, scan artifacts, worst quality, bad quality, jpeg artifacts, very displeasing, chromatic aberration, dithering, halftone, screentone"`

#### img2img パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `action` | `string` | `"generate"` | `"generate"` / `"img2img"` / `"infill"` |
| `source_image` | `ImageInput?` | — | 元画像 (img2img/infill時必須) |
| `img2img_strength` | `number` | `0.62` | 変化の強さ (0.0〜1.0) |
| `img2img_noise` | `number` | `0.0` | ノイズ量 (0.0〜1.0) |

#### inpaint パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `mask` | `ImageInput?` | — | マスク画像 (infill時必須, 白=変更/黒=保持) |
| `mask_strength` | `number?` | — | マスク反映度 (0.01〜1.0, infill時必須) |
| `inpaint_color_correct` | `boolean` | `true` | 色補正の適用 |
| `hybrid_img2img_strength` | `number?` | — | ハイブリッド: 元画像維持率 (0.01〜0.99) |
| `hybrid_img2img_noise` | `number?` | — | ハイブリッド: ノイズ (0.0〜0.99) |

#### キャラクター設定

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `characters` | `CharacterConfig[]?` | — | キャラクター配列 (最大6) |

`CharacterConfig`:

| フィールド | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `prompt` | `string` | — (必須) | キャラクタープロンプト |
| `center_x` | `number` | `0.5` | 中心X座標 (0.0〜1.0) |
| `center_y` | `number` | `0.5` | 中心Y座標 (0.0〜1.0) |
| `negative_prompt` | `string` | `""` | ネガティブプロンプト |

#### Vibe Transfer

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `vibes` | `(VibeEncodeResult \| string)[]?` | — | Vibeファイルパスまたはエンコード結果 (最大10) |
| `vibe_strengths` | `number[]?` | 全て `0.7` | 各Vibeの適用強度 |
| `vibe_info_extracted` | `number[]?` | 自動 | 各Vibeの情報抽出量 |

#### Character Reference

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `character_reference` | `CharacterReferenceConfig?` | — | キャラクター参照設定 |

`CharacterReferenceConfig`:

| フィールド | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `image` | `ImageInput` | — (必須) | 参照画像 |
| `strength` | `number` | `0.6` | 参照強度 (0.0〜1.0) |
| `fidelity` | `number` | `1.0` | 忠実度 (0.0〜1.0) |
| `mode` | `string` | `"character&style"` | `"character"` / `"character&style"` / `"style"` |

#### 出力オプション

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `save_path` | `string?` | — | 保存先ファイルパス (排他) |
| `save_dir` | `string?` | — | 保存先ディレクトリ (自動命名, 排他) |

### GenerateResult

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `image_data` | `Buffer \| Uint8Array` | PNG画像バイナリ |
| `seed` | `number` | 使用されたシード値 |
| `anlas_remaining` | `number \| null` | 残りアンラス |
| `anlas_consumed` | `number \| null` | 消費アンラス |
| `saved_path` | `string \| null` | 保存先パス |

### バリデーションルール

- `width * height <= 3,145,728` (MAX_PIXELS)
- `width` / `height` は64の倍数
- `vibes` と `character_reference` は同時使用不可
- `action="img2img"` → `source_image` 必須
- `action="infill"` → `source_image`, `mask`, `mask_strength` 必須
- ポジティブ/ネガティブプロンプトの合計トークン数 <= 512

---

## encodeVibe()

```typescript
async encodeVibe(params: EncodeVibeParams): Promise<VibeEncodeResult>
```

画像をVibe Transfer用にエンコードする。2 Anlas消費。

### EncodeVibeParams

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `image` | `ImageInput` | — (必須) | エンコードする画像 |
| `model` | `string` | `"nai-diffusion-4-5-full"` | モデル名 |
| `information_extracted` | `number` | `0.7` | 抽出情報量 (0.0〜1.0) |
| `strength` | `number` | `0.7` | 適用強度 (0.0〜1.0) |
| `save_path` | `string?` | — | 保存先パス (排他) |
| `save_dir` | `string?` | — | 保存先ディレクトリ (排他) |
| `save_filename` | `string?` | 自動 | ファイル名 (save_dirと組み合わせ) |

### VibeEncodeResult

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `encoding` | `string` | Base64エンコードされたVibe表現 |
| `model` | `string` | 使用モデル |
| `information_extracted` | `number` | 抽出情報量 |
| `strength` | `number` | 適用強度 |
| `source_image_hash` | `string` | 元画像のSHA256ハッシュ |
| `created_at` | `Date` | 作成日時 |
| `saved_path` | `string \| null` | 保存先パス |
| `anlas_remaining` | `number \| null` | 残りアンラス |
| `anlas_consumed` | `number \| null` | 消費アンラス |

### .naiv4vibe ファイル形式

```json
{
  "identifier": "novelai-vibe-transfer",
  "version": 1,
  "type": "encoding",
  "id": "<source_image_hash>",
  "encodings": {
    "<model_key>": {
      "unknown": {
        "encoding": "<base64>",
        "params": { "information_extracted": 0.7 }
      }
    }
  },
  "name": "<hash_prefix>-<hash_suffix>",
  "createdAt": "<ISO8601>",
  "importInfo": {
    "model": "<model>",
    "information_extracted": 0.7,
    "strength": 0.7
  }
}
```

モデルキー対応表:

| モデル | キー |
|--------|------|
| `nai-diffusion-4-curated-preview` | `v4curated` |
| `nai-diffusion-4-full` | `v4full` |
| `nai-diffusion-4-5-curated` | `v4-5curated` |
| `nai-diffusion-4-5-full` | `v4-5full` |

---

## augmentImage()

```typescript
async augmentImage(params: AugmentParams): Promise<AugmentResult>
```

画像加工ツール。width/height は画像から自動検出。

### AugmentParams

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `req_type` | `string` | — (必須) | ツール種別 |
| `image` | `ImageInput` | — (必須) | 加工対象画像 |
| `prompt` | `string?` | — | プロンプト (colorize/emotionのみ) |
| `defry` | `number?` | — | 変更強度 0〜5 (colorize/emotionのみ, 0=最強) |
| `save_path` | `string?` | — | 保存先パス (排他) |
| `save_dir` | `string?` | — | 保存先ディレクトリ (排他) |

### ツール別必須パラメータ

| ツール | prompt | defry | 備考 |
|--------|--------|-------|------|
| `colorize` | オプション | **必須** | |
| `emotion` | **必須** (キーワード) | **必須** | `;;` は自動付与 |
| `declutter` | 使用不可 | 使用不可 | |
| `sketch` | 使用不可 | 使用不可 | |
| `lineart` | 使用不可 | 使用不可 | |
| `bg-removal` | 使用不可 | 使用不可 | Opus無料対象外 |

### AugmentResult

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `image_data` | `Buffer \| Uint8Array` | 加工済みPNG画像バイナリ |
| `req_type` | `string` | 使用したツール種別 |
| `anlas_remaining` | `number \| null` | 残りアンラス |
| `anlas_consumed` | `number \| null` | 消費アンラス |
| `saved_path` | `string \| null` | 保存先パス |

---

## upscaleImage()

```typescript
async upscaleImage(params: UpscaleParams): Promise<UpscaleResult>
```

画像の解像度を2倍または4倍に拡大する。width/height は画像から自動検出。

### UpscaleParams

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `image` | `ImageInput` | — (必須) | 対象画像 |
| `scale` | `number` | `4` | 拡大倍率 (2 or 4) |
| `save_path` | `string?` | — | 保存先パス (排他) |
| `save_dir` | `string?` | — | 保存先ディレクトリ (排他) |

### UpscaleResult

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `image_data` | `Buffer \| Uint8Array` | 拡大済み画像バイナリ |
| `scale` | `number` | 使用した拡大倍率 |
| `output_width` | `number` | 出力画像幅 |
| `output_height` | `number` | 出力画像高さ |
| `anlas_remaining` | `number \| null` | 残りアンラス |
| `anlas_consumed` | `number \| null` | 消費アンラス |
| `saved_path` | `string \| null` | 保存先パス |

---

## getAnlasBalance()

```typescript
async getAnlasBalance(): Promise<AnlasBalance>
```

### AnlasBalance

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `fixed` | `number` | 固定アンラス (サブスクリプション付与分) |
| `purchased` | `number` | 購入済みアンラス |
| `total` | `number` | 合計 (`fixed + purchased`) |
| `tier` | `number` | ティア (0=Free, 1=Tablet, 2=Scroll, 3=Opus) |

---

## ImageInput 型

画像パラメータ (`image`, `source_image`, `mask`) は以下の型を受け付ける:

| 型 | 例 | 判別条件 |
|-----|-----|---------|
| ファイルパス | `"reference/input.jpeg"` | 拡張子またはパス区切り文字を含む |
| Base64文字列 | `"iVBORw0KGgo..."` | 64文字超のBase64パターン |
| Data URL | `"data:image/png;base64,..."` | `data:` プレフィックス |
| `Buffer` | `Buffer.from(...)` | `Buffer.isBuffer()` |
| `Uint8Array` | `new Uint8Array(...)` | `instanceof Uint8Array` |

---

## 有効な値一覧

### モデル (`VALID_MODELS`)

| 値 | 説明 |
|----|------|
| `"nai-diffusion-4-curated-preview"` | V4 Curated Preview |
| `"nai-diffusion-4-full"` | V4 Full |
| `"nai-diffusion-4-5-curated"` | V4.5 Curated |
| `"nai-diffusion-4-5-full"` | V4.5 Full (デフォルト) |

### サンプラー (`VALID_SAMPLERS`)

`"k_euler"`, `"k_euler_ancestral"` (デフォルト), `"k_dpmpp_2s_ancestral"`, `"k_dpmpp_2m_sde"`, `"k_dpmpp_2m"`, `"k_dpmpp_sde"`

### ノイズスケジュール (`VALID_NOISE_SCHEDULES`)

`"karras"` (デフォルト), `"exponential"`, `"polyexponential"`

---

## デフォルト値一覧

| 定数名 | 値 | 用途 |
|--------|-----|------|
| `DEFAULT_MODEL` | `"nai-diffusion-4-5-full"` | モデル |
| `DEFAULT_WIDTH` | `832` | 画像幅 |
| `DEFAULT_HEIGHT` | `1216` | 画像高さ |
| `DEFAULT_STEPS` | `23` | ステップ数 |
| `DEFAULT_SCALE` | `5.0` | CFGスケール |
| `DEFAULT_SAMPLER` | `"k_euler_ancestral"` | サンプラー |
| `DEFAULT_NOISE_SCHEDULE` | `"karras"` | ノイズスケジュール |
| `DEFAULT_CFG_RESCALE` | `0` | CFGリスケール |
| `DEFAULT_VIBE_STRENGTH` | `0.7` | Vibe適用強度 |
| `DEFAULT_VIBE_INFO_EXTRACTED` | `0.7` | Vibe情報抽出量 |
| `DEFAULT_IMG2IMG_STRENGTH` | `0.62` | img2img変化強度 |
| `DEFAULT_INPAINT_COLOR_CORRECT` | `true` | Inpaint色補正 |
| `DEFAULT_UPSCALE_SCALE` | `4` | アップスケール倍率 |
| `DEFAULT_DEFRY` | `3` | Augment defry |
| `MAX_SEED` | `4294967295` | シード最大値 (2^32-1) |
| `MAX_PIXELS` | `3145728` | 最大ピクセル数 |
| `MAX_TOKENS` | `512` | プロンプト最大トークン数 |
| `MAX_CHARACTERS` | `6` | 最大キャラクター数 |
| `MAX_VIBES` | `10` | 最大Vibe数 |

---

## エクスポート型一覧

`src/schemas.ts` からエクスポートされる型:

| 型名 | 説明 |
|------|------|
| `GenerateParams` | generate() の入力パラメータ |
| `GenerateResult` | generate() の戻り値 |
| `EncodeVibeParams` | encodeVibe() の入力パラメータ |
| `VibeEncodeResult` | encodeVibe() の戻り値 |
| `AugmentParams` | augmentImage() の入力パラメータ |
| `AugmentResult` | augmentImage() の戻り値 |
| `UpscaleParams` | upscaleImage() の入力パラメータ |
| `UpscaleResult` | upscaleImage() の戻り値 |
| `CharacterConfig` | キャラクター設定 |
| `CharacterReferenceConfig` | キャラクター参照設定 |
| `AnlasBalanceResponse` | アンラス残高APIレスポンス |

`src/client.ts` からエクスポートされる型:

| 型名 | 説明 |
|------|------|
| `NovelAIClient` | メインクライアントクラス |
| `AnlasBalance` | getAnlasBalance() の戻り値 |
| `Logger` | ロガーインターフェース |
