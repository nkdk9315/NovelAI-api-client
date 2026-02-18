# swift-api: NovelAI 画像生成 API クライアントライブラリ (Swift)

## 技術スタック

- **言語**: Swift 5.9+
- **パッケージマネージャ**: Swift Package Manager (SPM)
- **テスト**: XCTest
- **主要依存**: ZIPFoundation (ZIP展開), msgpack-swift (ストリームパース)
- **標準フレームワーク**: CoreGraphics (画像処理), CryptoKit (SHA256), Compression (zlib解凍)
- **プラットフォーム**: macOS 13+ / iOS 16+

## フォルダ構成

```
Sources/NovelAIAPI/
├── Client/
│   ├── NovelAIClient.swift  # メインクラス (generate, encodeVibe, augmentImage, upscaleImage, getAnlasBalance)
│   ├── Payload.swift        # JSON ペイロード構築ヘルパー
│   ├── Response.swift       # レスポンスパース (ZIP/msgpack/PNG)
│   └── Retry.swift          # fetchWithRetry + Logger protocol + exponential backoff
├── Schemas/
│   ├── Types.swift          # 全構造体・enum 定義 (GenerateParams, ImageInput 等)
│   ├── Validation.swift     # validate() メソッド拡張 (全パラメータ型)
│   └── Builder.swift        # GenerateParamsBuilder (メソッドチェーン)
├── Tokenizer/
│   ├── CLIPTokenizer.swift  # CLIP BPE トークナイザー (LRU キャッシュ付き)
│   ├── T5Tokenizer.swift    # T5 Unigram (PureUnigram + Viterbi)
│   ├── Preprocess.swift     # T5 前処理 (ブラケット・ウェイト構文除去)
│   └── TokenizerCache.swift # TokenizerCacheManager actor (DL + ディスクキャッシュ)
├── Utils/
│   ├── ImageUtils.swift     # 画像変換 (ImageInput → Data/base64, 寸法取得)
│   ├── MaskUtils.swift      # マスク生成 (矩形・円形, CoreGraphics)
│   ├── CharRefUtils.swift   # キャラクター参照画像リサイズ
│   └── VibeUtils.swift      # .naiv4vibe ファイル読込・エンコーディング抽出
├── Constants.swift          # 全定数・デフォルト値・API URL・制限値
├── Error.swift              # NovelAIError enum (10 cases)
├── Anlas.swift              # Anlas コスト計算 (純粋関数)
└── NovelAIAPI.swift         # 再エクスポート
Sources/Example*/            # 使用例 (ExampleGenerate, ExampleAugment, ExampleInfill, ExampleTokenizer, ExampleValidation)
Tests/NovelAIAPITests/       # XCTest テスト
```

## ドキュメント

必要に応じて対応中の問題に関連するドキュメントのみ参照してください。

- docs/getting-started.md — 利用者向けクイックスタート
- docs/api-reference.md — メソッド・パラメータ・型の一覧
- docs/architecture.md — モジュール構造・設計判断 (開発者向け)
- docs/tokenizer-internals.md — トークナイザーアルゴリズム詳細
- ../../docs/api-protocol.md — NovelAI API の HTTP プロトコル詳細 (共通)
- ../../docs/anlas-cost-calculation.md — コスト計算ロジック逆解析 (共通)

## 主要パターン

- **画像入力**: `ImageInput` enum (`.filePath`, `.base64`, `.dataURL`, `.bytes`) で型安全に判別
- **バリデーション**: 各パラメータ型の `validate()` メソッド拡張 (同期) + 非同期トークンカウント
- **ビルダー**: `GenerateParamsBuilder` メソッドチェーン + `build()` でバリデーション付きビルド
- **リトライ**: 429 / ネットワークエラーに対し exponential backoff (最大3回, jitter付き)
- **レスポンスパース**: ZIP → msgpack stream → raw PNG のフォールバック
- **保存**: `savePath` (完全パス) と `saveDir` (自動命名) は排他
- **セキュリティ**: パストラバーサル3層防御, ZIPボム検出, レスポンスサイズ制限
- **concurrency**: `@unchecked Sendable` (NovelAIClient), `actor` (TokenizerCacheManager), `NSLock` (LRU キャッシュ)

## 環境変数

- `NAI_API_KEY` / `NOVELAI_API_KEY` — API認証キー (必須, 左が優先)
- `NOVELAI_API_URL` / `NOVELAI_STREAM_URL` / `NOVELAI_ENCODE_URL` / `NOVELAI_AUGMENT_URL` / `NOVELAI_UPSCALE_URL` / `NOVELAI_SUBSCRIPTION_URL` — エンドポイント上書き (オプション)

## 開発コマンド

```bash
swift build                    # ビルド
swift test                     # 全テスト実行
swift run ExampleGenerate      # 基本生成例
swift run ExampleAugment       # augment/upscale例
swift run ExampleInfill        # inpaint例
swift run ExampleTokenizer     # トークナイザー例
swift run ExampleValidation    # バリデーション例
```
