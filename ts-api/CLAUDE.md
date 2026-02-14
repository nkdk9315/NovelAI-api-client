# ts-api: NovelAI 画像生成 API クライアントライブラリ

## 技術スタック

- **ランタイム**: Node.js (ES2020, CommonJS)
- **パッケージマネージャ**: pnpm
- **テスト**: vitest
- **主要依存**: zod (バリデーション), sharp (画像処理), axios (トークナイザーDL), msgpackr (ストリームパース), adm-zip (ZIP展開), he (HTMLエンティティ)
- **オプション依存**: tokenizers (native T5, macOS ARM非対応の場合あり)

## フォルダ構成

```
src/
├── client.ts        # NovelAIClient クラス (generate, encodeVibe, augmentImage, upscaleImage, getAnlasBalance)
├── schemas.ts       # Zod スキーマ定義・バリデーション (GenerateParams, AugmentParams, UpscaleParams 等)
├── constants.ts     # 全定数・デフォルト値・API URL・制限値
├── utils.ts         # 画像処理ヘルパー (Buffer変換, リサイズ, マスク生成, Vibe/CharRef処理)
├── tokenizer.ts     # CLIP BPE + T5 Unigram トークナイザー (native/pure JS fallback)
├── anlas.ts         # Anlas コスト計算 (純粋関数, API呼び出しなし)
└── anlas-browser.ts # ブラウザ用エントリーポイント (anlas.ts + 定数の再エクスポート)
examples/            # 使用例 (example.ts, example_augment.ts, example_infill.ts, example_tokenizer.ts)
tests/               # vitest テスト
docs/                # ドキュメント
```

## ドキュメント

- @docs/getting-started.md — 利用者向けクイックスタート
- @docs/api-reference.md — メソッド・パラメータ・型の一覧
- @docs/architecture.md — モジュール構造・設計判断 (開発者向け)
- @docs/api-protocol.md — NovelAI API の HTTP プロトコル詳細 (移植者向け)
- @docs/tokenizer-internals.md — トークナイザーアルゴリズム詳細
- @docs/anlas_cost_calculation_analysis.md — コスト計算ロジック逆解析

## 主要パターン

- **画像入力**: ファイルパス / base64文字列 / Buffer / Uint8Array を自動判別 (`utils.ts:getImageBuffer`)
- **バリデーション**: Zod `superRefine` + 非同期トークンカウント (`schemas.ts:GenerateParamsSchema`)
- **リトライ**: 429 / ネットワークエラーに対し exponential backoff (最大3回, jitter付き)
- **レスポンスパース**: ZIP → msgpack stream → raw PNG のフォールバック
- **保存**: `save_path` (完全パス) と `save_dir` (自動命名) は排他
- **セキュリティ**: パストラバーサル3層防御, ZIPボム検出, レスポンスサイズ制限

## 環境変数

- `NOVELAI_API_KEY` — API認証キー (必須)
- `NOVELAI_API_URL` / `NOVELAI_STREAM_URL` / `NOVELAI_ENCODE_URL` / `NOVELAI_AUGMENT_URL` / `NOVELAI_UPSCALE_URL` / `NOVELAI_SUBSCRIPTION_URL` — エンドポイント上書き (オプション)

## 開発コマンド

```bash
pnpm test              # vitest 全テスト実行
pnpm test -- --watch   # ウォッチモード
pnpm exec tsx examples/example.ts           # 基本例
pnpm exec tsx examples/example_augment.ts   # augment/upscale例
pnpm exec tsx examples/example_infill.ts    # inpaint例
pnpm exec tsx examples/example_tokenizer.ts # トークナイザー例
```
