# アーキテクチャガイド

コードの改造・別言語への移植を行う開発者向けのモジュール構造・設計判断の解説。

## モジュール依存グラフ

```
Client/NovelAIClient.swift
├── Constants.swift              (定数・URL)
├── Schemas/Validation.swift     (validate() メソッド拡張)
│   ├── Constants.swift
│   └── Tokenizer/               (非同期トークンカウント)
├── Client/Payload.swift         (JSON ペイロード構築)
│   └── Constants.swift
├── Client/Response.swift        (ZIP/msgpack/PNG パース)
├── Client/Retry.swift           (fetchWithRetry + Logger)
└── Utils/
    ├── ImageUtils.swift         (ImageInput → Data/base64 変換)
    ├── MaskUtils.swift          (マスク生成・リサイズ)
    ├── CharRefUtils.swift       (キャラクター参照画像リサイズ)
    └── VibeUtils.swift          (.naiv4vibe 読込)

Tokenizer/                       (独立モジュール, 外部HTTP通信あり)
├── CLIPTokenizer.swift          (BPE, LRU キャッシュ)
├── T5Tokenizer.swift            (PureUnigram + Viterbi)
├── Preprocess.swift             (T5 前処理)
└── TokenizerCache.swift         (actor, ディスクキャッシュ, DL)
    └── Constants.swift          (MAX_TOKENS のみ)

Anlas.swift                      (純粋関数, 外部依存なし)
└── Constants.swift
```

## データフロー

### 画像生成 (generate)

```
ユーザー入力 (GenerateParams)
    │
    ▼
[1] バリデーション (params.validate())
    │   - 同期: 型チェック + 範囲チェック
    │   - validateActionDependencies: アクション依存性
    │   - validateVibeParams: Vibe整合性
    │   - validatePixelConstraints: ピクセル制約
    │   - validateDimensions: 寸法チェック (64の倍数)
    │
    ▼
[2] ペイロード構築 (Payload.swift: buildBasePayload + apply* ヘルパー群)
    │   - 基本パラメータ → JSON Dictionary
    │   - img2img: sourceImage → base64
    │   - infill: mask リサイズ (1/8), model + "-inpainting", cache_secret_key
    │   - vibes: .naiv4vibe → encoding 抽出
    │   - charref: 画像リサイズ + base64, director_reference_* パラメータ
    │   - characters: v4_prompt 構造構築
    │
    ▼
[3] API呼び出し (Retry.swift: fetchWithRetry)
    │   - URL: 常に STREAM_URL (`/ai/generate-image-stream`)
    │     ※ 公式サイトと同様、txt2img も含めて全フローを stream に統一。
    │       非 stream の `generate-image` は早期/中間フレームを返すケースがあり、
    │       ノイズ・低解像度状の出力につながるため使用しない。
    │   - Content-Type: multipart/form-data
    │     ※ `request` フィールド (filename `blob`, Content-Type `application/json`)
    │       に JSON ペイロードを格納。NovelAIClient.buildMultipartRequestBody が組み立て。
    │   - Authorization: Bearer <apiKey>
    │   - 429/ネットワークエラー → exponential backoff リトライ
    │   - Task.sleep + withThrowingTaskGroup で60秒タイムアウト
    │
    ▼
[4] レスポンスパース (Response.swift)
    │   - 常に parseStreamResponse (ZIP → PNG → フレーム化msgpack → 埋め込みPNG → raw msgpack フォールバック)
    │
    ▼
[5] 結果構築 + 保存
    - GenerateResult (imageData: Data, seed: UInt32, anlas)
    - savePath / saveDir 指定時はファイル保存
```

## レスポンスパース詳細

NovelAI APIは3つの形式でレスポンスを返す可能性がある:

### ZIP形式 (augment / upscale / レガシー generate)
- マジックバイト: `PK` (0x50, 0x4b)
- ZIPFoundation で展開
- `.png` / `.webp` / `.jpg` / `.jpeg` 拡張子のエントリを検索
- ZIPボム防御: エントリ数上限 (10), 展開サイズ上限 (50MB), 圧縮比上限 (100)

### msgpack stream (generate 全フロー)
- msgpack-swift でデコード
- `data` または `image` フィールドからバイナリ取得
- パース失敗時: PNGマジックバイト (89 50 4E 47) を探してIENDチャンクまで切り出し

### raw PNG (フォールバック)
- PNGシグネチャ (8バイト) で検出
- IENDマーカーで正確な終端を特定

### フォールバック順序 (`parseStreamResponse`)

```
1. ZIP シグネチャ (PK) → parseZipResponse
2. PNG シグネチャ (先頭) → そのまま返却
3. フレーム化 msgpack (4バイト BE 長プレフィックス) → 最終フレーム抽出 + エラー検出
4. 埋め込み PNG バイト検索 (末尾から) → IEND まで切り出し
5. Raw msgpack パース → data/image フィールド (後方互換フォールバック)
6. すべて失敗 → NovelAIError.parse
```

> フレーム化msgpackが埋め込みPNG検索より優先される。フレーム化パースは最終フレーム（フル解像度画像）を正しく抽出できるが、PNG検索では途中のプレビュー画像にマッチする可能性があるため。

## バリデーション設計

### validate() メソッドパターン

TypeScript 版の Zod `superRefine` パターンを、Swift では各構造体の `validate()` メソッド拡張に置き換え:

```swift
// Validation.swift
extension GenerateParams {
    public func validate() throws {
        try validateDimensions()               // 寸法チェック
        try validateGenerationParameters()     // steps, scale, seed 範囲
        try validateImg2ImgParameters()        // strength/noise 範囲
        try validateActionDependencies()       // action と source_image の整合性
        try validateVibeParams()               // vibes 配列整合性
        try validatePixelConstraints()         // width * height <= MAX_PIXELS
        try validateSaveOptions()              // savePath / saveDir 排他
    }
}
```

設計ポイント:
- **同期バリデーション**: `validate()` は `throws` のみ (非同期不要)
- **トークンカウントは別途**: `TokenizerCacheManager.shared.validateTokenCount()` で非同期実行
- **部分的バリデーション関数**: 各検証ロジックを独立メソッドに分離し、テスタビリティを確保
- **共通バリデータ**: `validateSaveOptionsExclusive` は Generate / EncodeVibe / Augment / Upscale で共用
- **エラー型**: `NovelAIError.validation` / `.range` / `.image` で分類

## 画像入力の抽象化

`ImageUtils.swift:getImageBuffer` が `ImageInput` enum の各ケースを処理:

```
ImageInput ─┬─ .bytes(Data)      → そのまま返却
            ├─ .filePath(String)  → FileManager で読み込み
            ├─ .base64(String)    → Data(base64Encoded:) でデコード
            └─ .dataURL(String)   → "data:...;base64," プレフィックス除去 → base64デコード
```

TypeScript 版との違い:
- **型安全**: `ImageInput` enum で入力種別をコンパイル時に明示 (文字列の自動判別不要)
- **looksLikeFilePath 不要**: enum のケースで明示的に区別済み

## セキュリティモデル

### パストラバーサル防御 (3層)

1. **Validation層** (`validateSafePath`): パスコンポーネントに `..` を含むパスを拒否
2. **ImageInput層** (`validateImageInputPath`): `.filePath` ケースのパスの `..` チェック
3. **ファイル書き込み層** (`NovelAIClient:validateSavePathTraversal`): `resolvingSymlinksInPath` 後の `..` チェック

### ZIPボム防御

- `MAX_ZIP_ENTRIES = 10`: エントリ数制限
- `MAX_DECOMPRESSED_IMAGE_SIZE = 50MB`: 展開後サイズ制限
- `MAX_COMPRESSION_RATIO = 100`: 圧縮比制限
- CRC32 検証

### レスポンスサイズ制限

- `MAX_RESPONSE_SIZE = 200MB`: バッファサイズチェック

## トークナイザー戦略

詳細は [docs/tokenizer-internals.md](./tokenizer-internals.md) を参照。

概要:
- **T5 (メイン)**: プロンプトのトークンカウントに使用。純Swift `PureUnigram` 実装のみ (native バインディング不要)
- **CLIP (補助)**: BPEアルゴリズム。純Swift 実装、`NSLock` + `Dictionary` による LRU キャッシュ
- **キャッシュ**: ディスク7日TTL + `TokenizerCacheManager` actor (actor の直列化で並行リクエストによる重複DLを防止)

## Swift Concurrency モデル

### NovelAIClient

`NovelAIClient` は `final class` で `@unchecked Sendable` マーク。内部状態 (`apiKey`, `logger`, `session`) はイミュータブルで初期化後は変更されないため安全。

```swift
public final class NovelAIClient: @unchecked Sendable {
    private let apiKey: String
    private let logger: Logger
    private let session: URLSession  // URLSession.shared (スレッドセーフ)
}
```

### TokenizerCacheManager

Swift `actor` で実装。actor の直列化により:
- 並行リクエストによる重複ダウンロードを防止
- ディスクキャッシュへのアクセスを安全に直列化
- TypeScript 版の「Promise キャッシュ」パターンが不要

```swift
public actor TokenizerCacheManager {
    public static let shared = TokenizerCacheManager()
    // actor の直列化で重複DL防止 — Promise キャッシュ不要
}
```

### CLIPTokenizer

LRU キャッシュの保護に `NSLock` を使用。`@unchecked Sendable` でマーク:

```swift
class NovelAIClipTokenizer: @unchecked Sendable {
    private let cacheLock = NSLock()
    private var bpeCache: [String: String]  // LRU キャッシュ
}
```

### fetchWithRetry

`withThrowingTaskGroup` でタイムアウトとリトライタスクを並行実行。先に完了した方が勝つ競争モデル:

```swift
withThrowingTaskGroup(of: (Data, HTTPURLResponse).self) { group in
    group.addTask { /* timeout */ }
    group.addTask { /* retry loop */ }
    let result = try await group.next()
    group.cancelAll()
    return result
}
```
