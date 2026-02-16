# API リファレンス

## NovelAIClient

### コンストラクタ

```swift
try NovelAIClient(apiKey: String? = nil, logger: Logger? = nil)
```

| 引数 | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `apiKey` | `String?` | `NAI_API_KEY` → `NOVELAI_API_KEY` 環境変数 | API認証キー |
| `logger` | `Logger?` | `DefaultLogger()` | ロガー (`warn`, `error` メソッドを持つプロトコル) |

APIキーが未指定かつ環境変数にもない場合は `NovelAIError.api` をスローする。

---

## generate()

```swift
func generate(_ params: GenerateParams) async throws -> GenerateResult
```

画像生成の統合メソッド。txt2img / img2img / inpaint を `action` パラメータで切り替える。

### GenerateParams

#### 基本パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `prompt` | `String` | — (必須) | プロンプト (空文字列可) |
| `negativePrompt` | `String?` | デフォルトネガティブ※ | ネガティブプロンプト |
| `model` | `Model` | `.naiDiffusion45Full` | モデル |
| `width` | `Int` | `832` | 画像幅 (64の倍数, 64〜2048) |
| `height` | `Int` | `1216` | 画像高さ (64の倍数, 64〜2048) |
| `steps` | `Int` | `23` | ステップ数 (1〜50) |
| `scale` | `Double` | `5.0` | CFGスケール (0.0〜10.0) |
| `cfgRescale` | `Double` | `0` | CFGリスケール (0〜1) |
| `seed` | `UInt32?` | ランダム | シード値 (0〜4294967295) |
| `sampler` | `Sampler` | `.kEulerAncestral` | サンプラー |
| `noiseSchedule` | `NoiseSchedule` | `.karras` | ノイズスケジュール |

※デフォルトネガティブ: `"nsfw, lowres, artistic error, film grain, scan artifacts, worst quality, bad quality, jpeg artifacts, very displeasing, chromatic aberration, dithering, halftone, screentone"`

#### img2img パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `action` | `GenerateAction` | `.generate` | `.generate` / `.img2img` / `.infill` |
| `sourceImage` | `ImageInput?` | — | 元画像 (img2img/infill時必須) |
| `img2imgStrength` | `Double` | `0.62` | 変化の強さ (0.0〜1.0) |
| `img2imgNoise` | `Double` | `0.0` | ノイズ量 (0.0〜1.0) |

#### inpaint パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `mask` | `ImageInput?` | — | マスク画像 (infill時必須, 白=変更/黒=保持) |
| `maskStrength` | `Double?` | — | マスク反映度 (0.01〜1.0, infill時必須) |
| `inpaintColorCorrect` | `Bool` | `true` | 色補正の適用 |
| `hybridImg2imgStrength` | `Double?` | — | ハイブリッド: 元画像維持率 (0.01〜0.99) |
| `hybridImg2imgNoise` | `Double?` | — | ハイブリッド: ノイズ (0.0〜0.99) |

#### キャラクター設定

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `characters` | `[CharacterConfig]?` | — | キャラクター配列 (最大6) |

`CharacterConfig`:

| フィールド | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `prompt` | `String` | — (必須) | キャラクタープロンプト |
| `centerX` | `Double` | `0.5` | 中心X座標 (0.0〜1.0) |
| `centerY` | `Double` | `0.5` | 中心Y座標 (0.0〜1.0) |
| `negativePrompt` | `String` | `""` | ネガティブプロンプト |

#### Vibe Transfer

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `vibes` | `[VibeItem]?` | — | Vibeアイテム配列 (最大10) |
| `vibeStrengths` | `[Double]?` | 全て `0.7` | 各Vibeの適用強度 |
| `vibeInfoExtracted` | `[Double]?` | 自動 | 各Vibeの情報抽出量 |

`VibeItem` enum:

| ケース | 説明 | 例 |
|--------|------|-----|
| `.encoded(VibeEncodeResult)` | エンコード済みVibe結果 | `.encoded(vibeResult)` |
| `.filePath(String)` | .naiv4vibe ファイルパス | `.filePath("vibes/style.naiv4vibe")` |

#### Character Reference

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `characterReference` | `CharacterReferenceConfig?` | — | キャラクター参照設定 |

`CharacterReferenceConfig`:

| フィールド | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `image` | `ImageInput` | — (必須) | 参照画像 |
| `strength` | `Double` | `0.6` | 参照強度 (0.0〜1.0) |
| `fidelity` | `Double` | `1.0` | 忠実度 (0.0〜1.0) |
| `mode` | `CharRefMode` | `.characterAndStyle` | `.character` / `.characterAndStyle` / `.style` |

#### 出力オプション

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `savePath` | `String?` | — | 保存先ファイルパス (排他) |
| `saveDir` | `String?` | — | 保存先ディレクトリ (自動命名, 排他) |

### GenerateResult

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `imageData` | `Data` | PNG画像バイナリ |
| `seed` | `UInt32` | 使用されたシード値 |
| `anlasRemaining` | `Int?` | 残りアンラス |
| `anlasConsumed` | `Int?` | 消費アンラス |
| `savedPath` | `String?` | 保存先パス |

### バリデーションルール

- `width * height <= 3,145,728` (MAX_PIXELS)
- `width` / `height` は64の倍数
- `vibes` と `characterReference` は同時使用不可
- `action == .img2img` → `sourceImage` 必須
- `action == .infill` → `sourceImage`, `mask`, `maskStrength` 必須
- ポジティブ/ネガティブプロンプトの合計トークン数 <= 512

---

## GenerateParamsBuilder

メソッドチェーンによる `GenerateParams` の構築:

```swift
let params = try GenerateParams.builder(prompt: "1girl, masterpiece")
    .model(.naiDiffusion45Full)
    .width(1024)
    .height(1024)
    .steps(28)
    .sampler(.kEulerAncestral)
    .saveDir("output/")
    .build()  // validate() + return GenerateParams
```

### 利用可能なメソッド

| メソッド | 引数型 |
|---------|--------|
| `.action(_:)` | `GenerateAction` |
| `.sourceImage(_:)` | `ImageInput` |
| `.img2imgStrength(_:)` | `Double` |
| `.img2imgNoise(_:)` | `Double` |
| `.mask(_:)` | `ImageInput` |
| `.maskStrength(_:)` | `Double` |
| `.inpaintColorCorrect(_:)` | `Bool` |
| `.hybridImg2imgStrength(_:)` | `Double` |
| `.hybridImg2imgNoise(_:)` | `Double` |
| `.characters(_:)` | `[CharacterConfig]` |
| `.vibes(_:)` | `[VibeItem]` |
| `.vibeStrengths(_:)` | `[Double]` |
| `.vibeInfoExtracted(_:)` | `[Double]` |
| `.characterReference(_:)` | `CharacterReferenceConfig` |
| `.negativePrompt(_:)` | `String` |
| `.savePath(_:)` | `String` |
| `.saveDir(_:)` | `String` |
| `.model(_:)` | `Model` |
| `.width(_:)` | `Int` |
| `.height(_:)` | `Int` |
| `.steps(_:)` | `Int` |
| `.scale(_:)` | `Double` |
| `.cfgRescale(_:)` | `Double` |
| `.seed(_:)` | `UInt32` |
| `.sampler(_:)` | `Sampler` |
| `.noiseSchedule(_:)` | `NoiseSchedule` |
| `.build()` | — (throws) |

---

## encodeVibe()

```swift
func encodeVibe(_ params: EncodeVibeParams) async throws -> VibeEncodeResult
```

画像をVibe Transfer用にエンコードする。2 Anlas消費。

### EncodeVibeParams

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `image` | `ImageInput` | — (必須) | エンコードする画像 |
| `model` | `Model` | `.naiDiffusion45Full` | モデル |
| `informationExtracted` | `Double` | `0.7` | 抽出情報量 (0.0〜1.0) |
| `strength` | `Double` | `0.7` | 適用強度 (0.0〜1.0) |
| `savePath` | `String?` | — | 保存先パス (排他) |
| `saveDir` | `String?` | — | 保存先ディレクトリ (排他) |
| `saveFilename` | `String?` | 自動 | ファイル名 (saveDirと組み合わせ) |

### VibeEncodeResult

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `encoding` | `String` | Base64エンコードされたVibe表現 |
| `model` | `Model` | 使用モデル |
| `informationExtracted` | `Double` | 抽出情報量 |
| `strength` | `Double` | 適用強度 |
| `sourceImageHash` | `String` | 元画像のSHA256ハッシュ |
| `createdAt` | `Date` | 作成日時 |
| `savedPath` | `String?` | 保存先パス |
| `anlasRemaining` | `Int?` | 残りアンラス |
| `anlasConsumed` | `Int?` | 消費アンラス |

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

| モデル (Swift enum) | rawValue | キー |
|---------------------|----------|------|
| `.naiDiffusion4CuratedPreview` | `nai-diffusion-4-curated-preview` | `v4curated` |
| `.naiDiffusion4Full` | `nai-diffusion-4-full` | `v4full` |
| `.naiDiffusion45Curated` | `nai-diffusion-4-5-curated` | `v4-5curated` |
| `.naiDiffusion45Full` | `nai-diffusion-4-5-full` | `v4-5full` |

---

## augmentImage()

```swift
func augmentImage(_ params: AugmentParams) async throws -> AugmentResult
```

画像加工ツール。width/height は画像から自動検出。

### AugmentParams

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `reqType` | `AugmentReqType` | — (必須) | ツール種別 |
| `image` | `ImageInput` | — (必須) | 加工対象画像 |
| `prompt` | `String?` | — | プロンプト (colorize/emotionのみ) |
| `defry` | `Int?` | — | 変更強度 0〜5 (colorize/emotionのみ, 0=最強) |
| `savePath` | `String?` | — | 保存先パス (排他) |
| `saveDir` | `String?` | — | 保存先ディレクトリ (排他) |

### AugmentReqType enum

| ケース | rawValue | 説明 |
|--------|----------|------|
| `.colorize` | `"colorize"` | カラー化 |
| `.declutter` | `"declutter"` | 不要要素除去 |
| `.emotion` | `"emotion"` | 表情変換 |
| `.sketch` | `"sketch"` | スケッチ化 |
| `.lineart` | `"lineart"` | 線画抽出 |
| `.bgRemoval` | `"bg-removal"` | 背景除去 |

### ツール別必須パラメータ

| ツール | prompt | defry | 備考 |
|--------|--------|-------|------|
| `.colorize` | オプション | **必須** | |
| `.emotion` | **必須** (キーワード) | **必須** | `;;` は自動付与 |
| `.declutter` | 使用不可 | 使用不可 | |
| `.sketch` | 使用不可 | 使用不可 | |
| `.lineart` | 使用不可 | 使用不可 | |
| `.bgRemoval` | 使用不可 | 使用不可 | Opus無料対象外 |

### AugmentResult

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `imageData` | `Data` | 加工済みPNG画像バイナリ |
| `reqType` | `AugmentReqType` | 使用したツール種別 |
| `anlasRemaining` | `Int?` | 残りアンラス |
| `anlasConsumed` | `Int?` | 消費アンラス |
| `savedPath` | `String?` | 保存先パス |

---

## upscaleImage()

```swift
func upscaleImage(_ params: UpscaleParams) async throws -> UpscaleResult
```

画像の解像度を2倍または4倍に拡大する。width/height は画像から自動検出。

### UpscaleParams

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `image` | `ImageInput` | — (必須) | 対象画像 |
| `scale` | `Int` | `4` | 拡大倍率 (2 or 4) |
| `savePath` | `String?` | — | 保存先パス (排他) |
| `saveDir` | `String?` | — | 保存先ディレクトリ (排他) |

### UpscaleResult

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `imageData` | `Data` | 拡大済み画像バイナリ |
| `scale` | `Int` | 使用した拡大倍率 |
| `outputWidth` | `Int` | 出力画像幅 |
| `outputHeight` | `Int` | 出力画像高さ |
| `anlasRemaining` | `Int?` | 残りアンラス |
| `anlasConsumed` | `Int?` | 消費アンラス |
| `savedPath` | `String?` | 保存先パス |

---

## getAnlasBalance()

```swift
func getAnlasBalance() async throws -> AnlasBalance
```

### AnlasBalance

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `fixedTrainingStepsLeft` | `Int` | 固定アンラス (サブスクリプション付与分) |
| `purchasedTrainingSteps` | `Int` | 購入済みアンラス |
| `tier` | `Int` | ティア (0=Free, 1=Tablet, 2=Scroll, 3=Opus) |

TypeScript 版との違い: `total` / `fixed` / `purchased` ではなく、API レスポンスのフィールド名をそのまま使用。合計は `fixedTrainingStepsLeft + purchasedTrainingSteps` で算出。

---

## ImageInput enum

画像パラメータ (`image`, `sourceImage`, `mask`) は `ImageInput` enum で型安全に指定:

| ケース | 例 | 説明 |
|--------|-----|------|
| `.filePath(String)` | `.filePath("reference/input.jpeg")` | ファイルパス |
| `.base64(String)` | `.base64("iVBORw0KGgo...")` | Base64文字列 |
| `.dataURL(String)` | `.dataURL("data:image/png;base64,...")` | Data URL |
| `.bytes(Data)` | `.bytes(imageData)` | 生バイナリデータ |

TypeScript 版との違い: 文字列の自動判別 (`looksLikeFilePath`) ではなく、enum のケースで明示的に入力種別を指定。

---

## 有効な値一覧

### Model enum

| ケース | rawValue | 説明 |
|--------|----------|------|
| `.naiDiffusion4CuratedPreview` | `"nai-diffusion-4-curated-preview"` | V4 Curated Preview |
| `.naiDiffusion4Full` | `"nai-diffusion-4-full"` | V4 Full |
| `.naiDiffusion45Curated` | `"nai-diffusion-4-5-curated"` | V4.5 Curated |
| `.naiDiffusion45Full` | `"nai-diffusion-4-5-full"` | V4.5 Full (デフォルト) |

### Sampler enum

| ケース | rawValue |
|--------|----------|
| `.kEuler` | `"k_euler"` |
| `.kEulerAncestral` | `"k_euler_ancestral"` (デフォルト) |
| `.kDpmpp2sAncestral` | `"k_dpmpp_2s_ancestral"` |
| `.kDpmpp2mSde` | `"k_dpmpp_2m_sde"` |
| `.kDpmpp2m` | `"k_dpmpp_2m"` |
| `.kDpmppSde` | `"k_dpmpp_sde"` |

### NoiseSchedule enum

| ケース | rawValue |
|--------|----------|
| `.karras` | `"karras"` (デフォルト) |
| `.exponential` | `"exponential"` |
| `.polyexponential` | `"polyexponential"` |

### EmotionKeyword enum

```
.neutral, .happy, .sad, .angry, .scared, .surprised,
.tired, .excited, .nervous, .thinking, .confused, .shy,
.disgusted, .smug, .bored, .laughing, .irritated, .aroused,
.embarrassed, .love, .worried, .determined, .hurt, .playful
```

---

## デフォルト値一覧

| 定数名 | 値 | 用途 |
|--------|-----|------|
| `DEFAULT_MODEL` | `.naiDiffusion45Full` | モデル |
| `DEFAULT_WIDTH` | `832` | 画像幅 |
| `DEFAULT_HEIGHT` | `1216` | 画像高さ |
| `DEFAULT_STEPS` | `23` | ステップ数 |
| `DEFAULT_SCALE` | `5.0` | CFGスケール |
| `DEFAULT_SAMPLER` | `.kEulerAncestral` | サンプラー |
| `DEFAULT_NOISE_SCHEDULE` | `.karras` | ノイズスケジュール |
| `DEFAULT_CFG_RESCALE` | `0` | CFGリスケール |
| `DEFAULT_VIBE_STRENGTH` | `0.7` | Vibe適用強度 |
| `DEFAULT_VIBE_INFO_EXTRACTED` | `0.7` | Vibe情報抽出量 |
| `DEFAULT_IMG2IMG_STRENGTH` | `0.62` | img2img変化強度 |
| `DEFAULT_INPAINT_COLOR_CORRECT` | `true` | Inpaint色補正 |
| `DEFAULT_UPSCALE_SCALE` | `4` | アップスケール倍率 |
| `DEFAULT_DEFRY` | `3` | Augment defry |
| `MAX_SEED` | `4_294_967_295` | シード最大値 (2^32-1) |
| `MAX_PIXELS` | `3_145_728` | 最大ピクセル数 |
| `MAX_TOKENS` | `512` | プロンプト最大トークン数 |
| `MAX_CHARACTERS` | `6` | 最大キャラクター数 |
| `MAX_VIBES` | `10` | 最大Vibe数 |

---

## エラー型 (NovelAIError)

`Sources/NovelAIAPI/Error.swift` で定義:

| ケース | 用途 |
|--------|------|
| `.validation(String)` | スキーマ/パラメータバリデーションエラー |
| `.range(String)` | 数値範囲エラー |
| `.image(String)` | 画像処理エラー |
| `.imageFileSize(String)` | 画像ファイルサイズ超過 |
| `.tokenizer(String)` | トークナイザー初期化/読込エラー |
| `.tokenValidation(String)` | トークン数超過 |
| `.api(statusCode: Int, message: String)` | API リクエスト/レスポンスエラー |
| `.parse(String)` | レスポンスパースエラー |
| `.io(String)` | ファイル I/O エラー |
| `.other(String)` | その他/予期しないエラー |

---

## Logger プロトコル

```swift
public protocol Logger: Sendable {
    func warn(_ message: String)
    func error(_ message: String)
}
```

`DefaultLogger` はメッセージを `stderr` に出力。カスタムロガーを `NovelAIClient` のイニシャライザに渡すことで差し替え可能。

---

## 公開型一覧

`import NovelAIAPI` で利用可能なすべての公開型:

| 型名 | 説明 |
|------|------|
| `NovelAIClient` | メインクライアントクラス |
| `GenerateParams` | generate() の入力パラメータ |
| `GenerateResult` | generate() の戻り値 |
| `GenerateParamsBuilder` | メソッドチェーンビルダー |
| `EncodeVibeParams` | encodeVibe() の入力パラメータ |
| `VibeEncodeResult` | encodeVibe() の戻り値 |
| `AugmentParams` | augmentImage() の入力パラメータ |
| `AugmentResult` | augmentImage() の戻り値 |
| `UpscaleParams` | upscaleImage() の入力パラメータ |
| `UpscaleResult` | upscaleImage() の戻り値 |
| `AnlasBalance` | getAnlasBalance() の戻り値 |
| `ImageInput` | 画像入力 enum |
| `VibeItem` | Vibe入力 enum |
| `CharacterConfig` | キャラクター設定 |
| `CharacterReferenceConfig` | キャラクター参照設定 |
| `GenerateAction` | 生成アクション enum |
| `CharRefMode` | キャラクター参照モード enum |
| `Model` | モデル enum |
| `Sampler` | サンプラー enum |
| `NoiseSchedule` | ノイズスケジュール enum |
| `AugmentReqType` | Augmentツール種別 enum |
| `EmotionKeyword` | 表情キーワード enum |
| `NovelAIError` | エラー enum |
| `Logger` | ロガープロトコル |
