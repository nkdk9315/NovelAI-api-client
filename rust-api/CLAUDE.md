# rust-api: NovelAI 画像生成 API クライアントライブラリ (Rust)

## 技術スタック

- **言語**: Rust 2021 edition
- **非同期ランタイム**: tokio
- **テスト**: cargo test (mockito, serial_test)
- **主要依存**: reqwest, serde/serde_json, image, secrecy, thiserror, flate2, zip, rmpv, lru
- **開発依存**: tempfile, mockito, serial_test, dotenvy, anyhow

## フォルダ構成

```
src/
├── lib.rs                  # クレートルート (モジュール公開)
├── error.rs                # NovelAIError 列挙型 (thiserror)
├── constants.rs            # 全定数・デフォルト値・Model/Sampler/NoiseSchedule enum
├── anlas.rs                # Anlas コスト計算 (純粋関数, API呼び出しなし)
├── schemas/
│   ├── types.rs            # 全型定義 (GenerateParams, ImageInput, SaveTarget 等)
│   ├── builder.rs          # GenerateParamsBuilder (メソッドチェーン)
│   └── validation.rs       # バリデーション (同期 + 非同期トークン検証)
├── utils/
│   ├── mod.rs              # パス安全検証 (validate_safe_path)
│   ├── image.rs            # 画像入力変換 (Buffer/Base64/ファイル自動判別)
│   ├── mask.rs             # マスクリサイズ・生成 (矩形/円形) + SHA256
│   ├── vibe.rs             # Vibe ファイル読込・エンコーディング抽出
│   └── charref.rs          # Character Reference 画像前処理 (リサイズ+パディング)
├── tokenizer/
│   ├── clip.rs             # CLIP BPE トークナイザー (GPT-2スタイル)
│   ├── t5.rs               # T5 Unigram トークナイザー (Viterbi)
│   ├── preprocess.rs       # T5 前処理 (ブラケット・ウェイト構文除去)
│   └── cache.rs            # ディスク/メモリキャッシュ + ネットワークDL + deflate解凍
└── client/
    ├── mod.rs              # NovelAIClient (generate, encode_vibe, augment, upscale, balance)
    ├── payload.rs          # JSON ペイロード構築 (v4_prompt, charRef, vibe等)
    ├── response.rs         # レスポンスパース (ZIP/msgpack/PNG フォールバック)
    └── retry.rs            # Exponential backoff リトライ (429/502/503)
examples/                   # 使用例 (example.rs, example_augment.rs, example_infill.rs)
tests/                      # 統合テスト
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

- **Builder**: `GenerateParams::builder()` → メソッドチェーン → `.build()` でバリデーション付き構築
- **データ付きEnum**: `GenerateAction::Img2Img { source_image, strength, noise }` で関連データを型安全に保持
- **柔軟入力**: `ImageInput` enum (FilePath/Base64/DataUrl/Bytes) で画像入力を統一
- **SaveTarget**: `None` / `ExactPath` / `Directory` の3択enum (排他性を型で保証)
- **Vibes**: `VibeItem` enum (FilePath/Encoded/RawEncoding) × `VibeConfig` で強度・情報抽出量を管理
- **リトライ**: 429/502/503 + ネットワークエラーに exponential backoff (最大3回, jitter付き)
- **セキュリティ**: SecretString (APIキー), パストラバーサル防御, ZIPボム検出, レスポンスサイズ制限

## 公開型概要

| カテゴリ | 型名 |
|---------|------|
| エントリーポイント | `NovelAIClient`, `Logger` trait, `DefaultLogger` |
| パラメータ | `GenerateParams`, `GenerateParamsBuilder`, `EncodeVibeParams`, `AugmentParams`, `UpscaleParams` |
| 結果 | `GenerateResult`, `AugmentResult`, `UpscaleResult`, `VibeEncodeResult` |
| 列挙型 | `GenerateAction`, `ImageInput`, `SaveTarget`, `VibeItem`, `CharRefMode`, `Model`, `Sampler`, `NoiseSchedule`, `AugmentReqType` |
| 設定 | `CharacterConfig`, `CharacterReferenceConfig`, `VibeConfig`, `CaptionDict`, `CaptionCenter` |
| エラー | `NovelAIError`, `Result<T>` |
| コスト | `GenerationCostParams`, `AugmentCostParams`, `UpscaleCostParams`, `SubscriptionTier` |

## 環境変数

- `NOVELAI_API_KEY` — API認証キー (必須)
- `NOVELAI_API_URL` / `NOVELAI_STREAM_URL` / `NOVELAI_ENCODE_URL` / `NOVELAI_AUGMENT_URL` / `NOVELAI_UPSCALE_URL` / `NOVELAI_SUBSCRIPTION_URL` — エンドポイント上書き (オプション)

## 開発コマンド

```bash
cargo test                          # 全テスト実行
cargo clippy                        # lint
cargo run --example example         # 基本例 (txt2img/img2img/vibe/charref)
cargo run --example example_augment # augment/upscale例
cargo run --example example_infill  # inpaint例
```
