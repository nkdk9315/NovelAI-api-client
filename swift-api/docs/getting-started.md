# クイックスタートガイド

NovelAI 画像生成 API を Swift から利用するためのガイド。

## セットアップ

### 1. SPM 依存の追加

`Package.swift` に依存を追加:

```swift
dependencies: [
    .package(url: "https://github.com/your-org/novelai-swift-api.git", from: "1.0.0"),
],
targets: [
    .target(
        name: "YourApp",
        dependencies: ["NovelAIAPI"]
    ),
]
```

### 2. 環境変数の設定

```bash
export NAI_API_KEY=your_api_key_here
# または
export NOVELAI_API_KEY=your_api_key_here
```

APIキーは [NovelAI](https://novelai.net/) のアカウント設定から取得できる。

### 3. コード内での初期化

```swift
import NovelAIAPI

// 環境変数から自動取得 (NAI_API_KEY → NOVELAI_API_KEY の順)
let client = try NovelAIClient()

// または直接指定
let client2 = try NovelAIClient(apiKey: "your_api_key")
```

---

## 基本的な画像生成 (txt2img)

```swift
import NovelAIAPI

let client = try NovelAIClient()

do {
    let result = try await client.generate(GenerateParams(
        prompt: "1girl, beautiful anime girl, detailed eyes, masterpiece, best quality",
        saveDir: "output/"
    ))

    print("保存先: \(result.savedPath ?? "未保存")")
    print("シード値: \(result.seed)")
    print("消費アンラス: \(result.anlasConsumed ?? 0)")
    print("残りアンラス: \(result.anlasRemaining ?? 0)")
} catch let error as NovelAIError {
    switch error {
    case .validation(let msg):
        print("バリデーションエラー: \(msg)")
    case .range(let msg):
        print("範囲エラー: \(msg)")
    case .api(let statusCode, let msg):
        print("APIエラー (\(statusCode)): \(msg)")
    default:
        print("エラー: \(error.localizedDescription)")
    }
}
```

### 結果の読み方

`GenerateResult` 構造体:

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `imageData` | `Data` | PNG画像のバイナリデータ |
| `seed` | `UInt32` | 使用されたシード値 |
| `anlasConsumed` | `Int?` | 消費したアンラス数 |
| `anlasRemaining` | `Int?` | 残りのアンラス数 |
| `savedPath` | `String?` | 保存したファイルパス |

### サイズ指定

```swift
let result = try await client.generate(GenerateParams(
    prompt: "landscape, mountain, sunset",
    width: 1024,    // 64の倍数 (64〜2048)
    height: 1024,
    saveDir: "output/"
))
```

デフォルト: 832x1216 (縦長ポートレート)

### Builder パターン

`GenerateParamsBuilder` でメソッドチェーンを使った構築も可能:

```swift
let params = try GenerateParams.builder(prompt: "1girl, masterpiece, best quality")
    .model(.naiDiffusion45Full)
    .width(1024)
    .height(1024)
    .steps(28)
    .scale(5.0)
    .sampler(.kEulerAncestral)
    .noiseSchedule(.karras)
    .saveDir("output/")
    .build()

let result = try await client.generate(params)
```

---

## Image to Image (img2img)

既存画像をベースに新しい画像を生成する。

```swift
let result = try await client.generate(GenerateParams(
    prompt: "1girl, beautiful, masterpiece",
    action: .img2img,
    sourceImage: .filePath("reference/input.jpeg"),  // ファイルパス
    img2imgStrength: 0.6,   // 変化の強さ (0.0〜1.0, デフォルト: 0.62)
    img2imgNoise: 0.1,      // ノイズ量 (0.0〜1.0, デフォルト: 0.0)
    width: 832,
    height: 1216,
    saveDir: "output/"
))
```

### ImageInput の指定方法

```swift
// ファイルパス
let img1: ImageInput = .filePath("reference/input.jpeg")

// Base64文字列
let img2: ImageInput = .base64("iVBORw0KGgo...")

// Data URL
let img3: ImageInput = .dataURL("data:image/png;base64,iVBORw0KGgo...")

// 生バイナリ (Data)
let imageData = try Data(contentsOf: URL(fileURLWithPath: "image.png"))
let img4: ImageInput = .bytes(imageData)
```

---

## Inpaint (部分再生成)

画像の一部をマスクで指定し、その領域だけを再生成する。

```swift
let result = try await client.generate(GenerateParams(
    prompt: "1girl, smiling, happy",
    action: .infill,
    sourceImage: .filePath("reference/input.jpeg"),
    mask: .filePath("reference/mask.png"),    // 白=再生成、黒=保持
    maskStrength: 0.7,                        // マスク反映度 (0.01〜1.0, 必須)
    width: 832,
    height: 1216,
    saveDir: "output/"
))
```

### プログラムによるマスク生成

ユーティリティ関数で矩形・円形マスクを生成できる:

```swift
// 矩形マスク (座標は 0.0〜1.0 の相対値)
let rectMask = try createRectangularMask(
    width: 832,
    height: 1216,
    region: MaskRegion(x: 0.2, y: 0.3, w: 0.5, h: 0.4)
)

// 円形マスク
let circleMask = try createCircularMask(
    width: 832,
    height: 1216,
    center: MaskCenter(x: 0.5, y: 0.5),
    radius: 0.3   // 幅に対する相対値
)

// マスクを ImageInput として使用
let result = try await client.generate(GenerateParams(
    prompt: "1girl, smiling",
    action: .infill,
    sourceImage: .filePath("reference/input.jpeg"),
    mask: .bytes(rectMask),
    maskStrength: 0.7,
    width: 832,
    height: 1216,
    saveDir: "output/"
))
```

### ハイブリッドモード (Inpaint + Img2Img)

マスク領域の再生成と同時に、元画像の影響度も制御する:

```swift
let result = try await client.generate(GenerateParams(
    prompt: "1girl, beautiful dress",
    action: .infill,
    sourceImage: .filePath("reference/input.jpeg"),
    mask: .filePath("reference/mask.png"),
    maskStrength: 0.68,
    hybridImg2imgStrength: 0.45,  // 元画像維持率 (0.01〜0.99)
    hybridImg2imgNoise: 0,        // 元画像ノイズ (0〜0.99)
    width: 832,
    height: 1216,
    saveDir: "output/"
))
```

---

## Vibe Transfer

エンコード済み画像のスタイルを生成に反映させる。

### ステップ1: Vibeエンコード

画像を `.naiv4vibe` ファイルにエンコードする (2 Anlas消費):

```swift
let vibeResult = try await client.encodeVibe(EncodeVibeParams(
    image: .filePath("reference/style_image.png"),
    informationExtracted: 0.5,   // 抽出情報量 (0.0〜1.0, デフォルト: 0.7)
    strength: 0.7,               // 適用強度 (0.0〜1.0, デフォルト: 0.7)
    saveDir: "./vibes",
    saveFilename: "my_style"     // → vibes/my_style.naiv4vibe
))
```

### ステップ2: Vibeを使って生成

```swift
let result = try await client.generate(GenerateParams(
    prompt: "1girl, standing, outdoor",
    vibes: [.filePath("vibes/my_style.naiv4vibe")],
    vibeStrengths: [0.7],       // 各Vibeの適用強度
    saveDir: "output/"
))
```

複数Vibeも指定可能 (最大10個、5個以上は1Vibeあたり追加2Anlas):

```swift
let result = try await client.generate(GenerateParams(
    prompt: "1girl",
    vibes: [
        .filePath("vibes/style1.naiv4vibe"),
        .filePath("vibes/style2.naiv4vibe"),
    ],
    vibeStrengths: [0.5, 0.3],
    saveDir: "output/"
))
```

エンコード結果を直接使用することも可能:

```swift
let result = try await client.generate(GenerateParams(
    prompt: "1girl",
    vibes: [.encoded(vibeResult)],  // VibeEncodeResult を直接渡す
    saveDir: "output/"
))
```

---

## Character Reference (キャラクター参照)

参照画像のキャラクターを生成に反映させる。

```swift
let result = try await client.generate(GenerateParams(
    prompt: "school classroom, sunny day",
    characters: [
        CharacterConfig(
            prompt: "1girl, standing",
            centerX: 0.5,    // キャラクター中心X (0.0〜1.0)
            centerY: 0.5     // キャラクター中心Y (0.0〜1.0)
        ),
    ],
    characterReference: CharacterReferenceConfig(
        image: .filePath("reference/character.png"),
        strength: 0.6,       // 参照強度 (0.0〜1.0, デフォルト: 0.6)
        fidelity: 0.8,       // 忠実度 (0.0〜1.0, デフォルト: 1.0)
        mode: .characterAndStyle  // .character / .characterAndStyle / .style
    ),
    saveDir: "output/"
))
```

### モード一覧

| モード | 説明 |
|--------|------|
| `.character` | キャラクターの外見のみ参照 |
| `.characterAndStyle` | キャラクター + 画風を参照 (デフォルト) |
| `.style` | 画風のみ参照 |

---

## Augment (画像加工ツール)

6種類の画像加工ツールが利用可能。

```swift
let result = try await client.augmentImage(AugmentParams(
    reqType: .colorize,
    image: .filePath("reference/mono_image.png"),
    prompt: "vibrant colors",   // colorize/emotionのみ
    defry: 3,                    // colorize/emotionのみ (0〜5, 0=最強変更)
    saveDir: "output/augment/"
))
```

### ツール別パラメータ

| ツール | 説明 | prompt | defry |
|--------|------|--------|-------|
| `.colorize` | カラー化 | オプション | **必須** (0〜5) |
| `.emotion` | 表情変換 | **必須** (キーワード) | **必須** (0〜5) |
| `.declutter` | 不要要素除去 | 使用不可 | 使用不可 |
| `.sketch` | スケッチ化 | 使用不可 | 使用不可 |
| `.lineart` | 線画抽出 | 使用不可 | 使用不可 |
| `.bgRemoval` | 背景除去 | 使用不可 | 使用不可 |

### emotion の有効キーワード

```
neutral, happy, sad, angry, scared, surprised, tired, excited,
nervous, thinking, confused, shy, disgusted, smug, bored,
laughing, irritated, aroused, embarrassed, love, worried,
determined, hurt, playful
```

```swift
// 表情変換の例
let result = try await client.augmentImage(AugmentParams(
    reqType: .emotion,
    image: .filePath("reference/face.png"),
    prompt: "happy",    // キーワード指定 (;;は自動付与)
    defry: 0,           // 最強変更
    saveDir: "output/augment/"
))
```

---

## Upscale (画像拡大)

```swift
let result = try await client.upscaleImage(UpscaleParams(
    image: .filePath("reference/input.jpeg"),
    scale: 4,          // 2 or 4 (デフォルト: 4)
    saveDir: "output/"
))

print("出力サイズ: \(result.outputWidth)x\(result.outputHeight)")
```

---

## アンラス残高確認

```swift
let balance = try await client.getAnlasBalance()
let total = balance.fixedTrainingStepsLeft + balance.purchasedTrainingSteps
print("合計: \(total)")
print("固定: \(balance.fixedTrainingStepsLeft)")
print("購入済み: \(balance.purchasedTrainingSteps)")
print("ティア: \(balance.tier)")  // 0=Free, 1=Tablet, 2=Scroll, 3=Opus
```

---

## エラーハンドリング

### NovelAIError パターンマッチング

パラメータが不正な場合やAPI呼び出しが失敗した場合、`NovelAIError` がスローされる。

```swift
do {
    let result = try await client.generate(GenerateParams(
        prompt: "test",
        width: 100  // 64の倍数でない
    ))
} catch let error as NovelAIError {
    switch error {
    case .validation(let msg):
        print("バリデーションエラー: \(msg)")
        // → "Width must be a multiple of 64"
    case .range(let msg):
        print("範囲エラー: \(msg)")
    case .image(let msg):
        print("画像エラー: \(msg)")
    case .imageFileSize(let msg):
        print("ファイルサイズ超過: \(msg)")
    case .tokenizer(let msg):
        print("トークナイザーエラー: \(msg)")
    case .tokenValidation(let msg):
        print("トークン数超過: \(msg)")
    case .api(let statusCode, let msg):
        print("APIエラー (\(statusCode)): \(msg)")
    case .parse(let msg):
        print("パースエラー: \(msg)")
    case .io(let msg):
        print("I/Oエラー: \(msg)")
    case .other(let msg):
        print("その他エラー: \(msg)")
    }
}
```

### HTTPエラー

API呼び出しが失敗した場合は `NovelAIError.api` がスローされる。429 (レートリミット) は自動リトライ (最大3回、exponential backoff)。

### リトライ動作

- **リトライ対象**: 429 (Too Many Requests) / ネットワークエラー (タイムアウト, DNS失敗, 接続拒否)
- **リトライ対象外**: 400, 401, 403, 500 等
- **バックオフ**: `1000ms * 2^attempt * (1 + random * 0.3)`
- **最大リトライ**: 3回

---

## よくあるミスと注意点

### 画像サイズは64の倍数

```swift
// NG: NovelAIError.validation
GenerateParams(prompt: "test", width: 500, height: 700)

// OK
GenerateParams(prompt: "test", width: 512, height: 704)
```

### 最大ピクセル数 (3,145,728 = 2048x1536)

```swift
// NG: width * height > 3,145,728
GenerateParams(prompt: "test", width: 2048, height: 2048)  // 4,194,304 > 3,145,728

// OK
GenerateParams(prompt: "test", width: 2048, height: 1536)  // 3,145,728
```

### vibes と characterReference は同時使用不可

```swift
// NG: NovelAIError.validation
GenerateParams(
    prompt: "test",
    vibes: [.filePath("vibes/style.naiv4vibe")],
    characterReference: CharacterReferenceConfig(image: .filePath("ref.png"))
)
```

### savePath と saveDir は排他

```swift
// NG: NovelAIError.validation
GenerateParams(prompt: "test", savePath: "output/image.png", saveDir: "output/")

// OK: どちらか一方
GenerateParams(prompt: "test", savePath: "output/image.png")
GenerateParams(prompt: "test", saveDir: "output/")
```

### トークン上限は512

ポジティブプロンプト (ベース + 全キャラクター) の合計トークン数が512を超えるとエラー。
ネガティブプロンプトも同様に512が上限。

### action と sourceImage の整合性

```swift
// NG: sourceImage がない
GenerateParams(prompt: "test", action: .img2img)

// NG: mask/maskStrength がない
GenerateParams(prompt: "test", action: .infill, sourceImage: .filePath("img.png"))
```

### emotion の prompt にはキーワードのみ

```swift
// NG: 自由テキスト
AugmentParams(reqType: .emotion, image: .filePath("face.png"), prompt: "a very happy face", defry: 3)

// OK: 定義済みキーワード
AugmentParams(reqType: .emotion, image: .filePath("face.png"), prompt: "happy", defry: 3)
```
