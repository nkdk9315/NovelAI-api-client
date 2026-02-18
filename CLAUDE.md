# NovelAI 画像生成 API クライアントライブラリ (マルチ言語)

## 概要

NovelAI の非公式画像生成 API を3つの言語で実装したクライアントライブラリ群。
各実装は同一の API プロトコルに従い、同一の機能セットを提供する。

## リポジトリ構成

```
.
├── ts-api/        # TypeScript (Node.js) 実装
├── rust-api/      # Rust 実装
├── swift-api/     # Swift (macOS/iOS) 実装
└── docs/          # 共通ドキュメント
```

## 各API内部構造

### ts-api (TypeScript)

```
ts-api/src/
├── client.ts        # メインクライアント
├── schemas.ts       # Zod バリデーション
├── constants.ts     # 定数・デフォルト値
├── utils.ts         # 画像処理ヘルパー
├── tokenizer.ts     # CLIP BPE + T5 Unigram
├── anlas.ts         # コスト計算 (純粋関数)
└── anlas-browser.ts # ブラウザ用エントリ
```

### rust-api (Rust)

```
rust-api/src/
├── client/          # NovelAIClient + payload/response/retry
├── schemas/         # 型定義 + builder + validation
├── utils/           # 画像・マスク・vibe・charref
├── tokenizer/       # CLIP BPE + T5 Unigram
├── constants.rs     # 定数・enum
├── anlas.rs         # コスト計算 (純粋関数)
└── error.rs         # エラー型
```

### swift-api (Swift)

```
swift-api/Sources/NovelAIAPI/
├── Client/          # NovelAIClient + Payload/Response/Retry
├── Schemas/         # Types + Validation + Builder
├── Utils/           # ImageUtils/MaskUtils/CharRefUtils/VibeUtils
├── Tokenizer/       # CLIPTokenizer + T5Tokenizer + Cache
├── Constants.swift  # 定数・デフォルト値
├── Anlas.swift      # コスト計算 (純粋関数)
└── Error.swift      # NovelAIError enum
```

## 共通ドキュメント

必要に応じて対応中の問題に関連するドキュメントのみ参照してください。

- docs/api-protocol.md — NovelAI API の HTTP プロトコル詳細
- docs/anlas-cost-calculation.md — コスト計算ロジック逆解析
- ts-api/CLAUDE.md — TypeScript 版の技術スタック・コマンド・パターン
- rust-api/CLAUDE.md — Rust 版の技術スタック・コマンド・パターン
- swift-api/CLAUDE.md — Swift 版の技術スタック・コマンド・パターン

## 共通設計パターン

3つの実装すべてに共通する設計原則:

### 機能セット

- **txt2img / img2img / inpaint**: 統合 generate メソッドで action パラメータにより切り替え
- **Vibe Transfer**: .naiv4vibe ファイルによるスタイル転写
- **Character Reference**: 参照画像からキャラクター/スタイルを反映
- **Augment**: 6種の画像加工ツール (colorize, emotion, sketch, lineart, declutter, bg-removal)
- **Upscale**: 2x/4x 画像拡大
- **Anlas コスト計算**: 純粋関数として実装 (API呼び出し不要)

### 画像入力の抽象化

各言語でファイルパス / Base64 / Data URL / バイト列を統一的に扱う:
- ts-api: `string | Buffer | Uint8Array` (自動判別)
- rust-api: `ImageInput` enum (明示的)
- swift-api: `ImageInput` enum (明示的)

### レスポンスパース

ZIP → msgpack stream → raw PNG の3形式フォールバック:
1. ZIP シグネチャ (`PK`) → ZIP展開
2. PNG シグネチャ → そのまま返却
3. msgpack → `data`/`image` フィールド抽出
4. 埋め込み PNG 検索 → IEND まで切り出し

### セキュリティ

- パストラバーサル防御 (3層)
- ZIPボム検出 (エントリ数, 展開サイズ, 圧縮比)
- レスポンスサイズ制限

### リトライ戦略

- 対象: 429 (Too Many Requests) + ネットワークエラー
- Exponential backoff: `1000ms × 2^attempt × (1 + random × 0.3)`
- 最大3回リトライ

### トークナイザー

- CLIP BPE + T5 Unigram (SentencePiece Viterbi)
- 定義ファイルはネットワークDL + ディスクキャッシュ (7日TTL)
- プロンプト上限: 512 トークン

## 共通環境変数

| 変数名 | 用途 |
|--------|------|
| `NOVELAI_API_KEY` | API認証キー (必須) |
| `NOVELAI_API_URL` | generate-image エンドポイント上書き |
| `NOVELAI_STREAM_URL` | generate-image-stream エンドポイント上書き |
| `NOVELAI_ENCODE_URL` | encode-vibe エンドポイント上書き |
| `NOVELAI_AUGMENT_URL` | augment-image エンドポイント上書き |
| `NOVELAI_UPSCALE_URL` | upscale エンドポイント上書き |
| `NOVELAI_SUBSCRIPTION_URL` | subscription エンドポイント上書き |

> Swift版のみ `NAI_API_KEY` も受け付ける (`NAI_API_KEY` → `NOVELAI_API_KEY` の優先順)。
