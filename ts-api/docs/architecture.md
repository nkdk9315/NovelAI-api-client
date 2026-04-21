# アーキテクチャガイド

コードの改造・別言語への移植を行う開発者向けのモジュール構造・設計判断の解説。

## モジュール依存グラフ

```
client.ts
├── constants.ts     (定数・URL)
├── schemas.ts       (Zodバリデーション)
│   ├── constants.ts
│   └── tokenizer.ts (非同期トークンカウント)
└── utils.ts         (画像処理)
    ├── constants.ts
    └── schemas.ts   (型のみ)

tokenizer.ts         (独立モジュール, 外部HTTP通信あり)
├── constants.ts     (MAX_TOKENS のみ)
└── axios            (トークナイザー定義のダウンロード)

anlas.ts             (純粋関数, 外部依存なし)
└── constants.ts

anlas-browser.ts     (再エクスポートのみ)
├── anlas.ts
└── constants.ts
```

## データフロー

### 画像生成 (generate)

```
ユーザー入力 (GenerateParams)
    │
    ▼
[1] Zod バリデーション (GenerateParamsSchema.parseAsync)
    │   - 型チェック + デフォルト値適用
    │   - superRefine: アクション依存性, Vibe整合性, ピクセル制約
    │   - 非同期: T5トークンカウント (<= 512)
    │
    ▼
[2] ペイロード構築 (buildBasePayload + apply* ヘルパー群)
    │   - 基本パラメータ → GenerationPayload
    │   - img2img: source_image → base64
    │   - infill: mask リサイズ (1/8), model + "-inpainting", cache_secret_key
    │   - vibes: .naiv4vibe → encoding 抽出
    │   - charref: 画像リサイズ + base64, director_reference_* パラメータ
    │   - characters: v4_prompt 構造構築
    │
    ▼
[3] API呼び出し (fetchWithRetry)
    │   - URL: 常に STREAM_URL (`/ai/generate-image-stream`)
    │     ※ 公式サイトと同様、txt2img も含めて全フローを stream に統一。
    │       非 stream の `generate-image` は早期/中間フレームを返すケースがあり、
    │       ノイズ・低解像度状の出力につながるため使用しない。
    │   - Content-Type: multipart/form-data
    │     ※ `request` フィールドに JSON ペイロードを Blob (filename `blob`,
    │       Content-Type `application/json`) として格納。公式サイトと同形式。
    │   - Authorization: Bearer <apiKey>
    │   - 429/ネットワークエラー → exponential backoff リトライ
    │
    ▼
[4] レスポンスパース
    │   - 常に parseStreamResponse (ZIP → PNG → フレーム化msgpack → raw msgpack → 埋め込みPNG フォールバック)
    │
    ▼
[5] 結果構築 + 保存
    - GenerateResult (image_data, seed, anlas)
    - save_path / save_dir 指定時はファイル保存
```

## レスポンスパース詳細

NovelAI APIは3つの形式でレスポンスを返す可能性がある:

### ZIP形式 (augment / upscale / レガシー generate)
- マジックバイト: `PK` (0x50, 0x4b)
- adm-zip で展開
- `.png` / `.webp` / `.jpg` 拡張子のエントリを検索
- ZIPボム防御: エントリ数上限 (10), 展開サイズ上限 (50MB), 圧縮比上限 (100)

### msgpack stream (generate 全フロー)
- msgpackr の `unpackMultiple` でパース
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
4. Raw msgpack パース → 最後の data/image フィールド (後方互換フォールバック)
5. 埋め込み PNG マジックバイト検索 → IEND までスライス
6. すべて失敗 → Error
```

> フレーム化msgpackが埋め込みPNG検索より優先される。フレーム化パースは最終フレーム（フル解像度画像）を正しく抽出できるが、PNG検索では途中のプレビュー画像にマッチする可能性があるため。
> TS版では raw msgpack が埋め込みPNG より先に試行される (Swift/Rust版とは逆順)。

## バリデーション設計

### Zod superRefine パターン

`GenerateParamsSchema` は基本スキーマ (`GenerateParamsBaseSchema`) に `superRefine` を連鎖させる設計:

```typescript
GenerateParamsBaseSchema
  .superRefine(async (data, ctx) => {
    validateActionDependencies(data, ctx);  // 同期
    validateVibeParams(data, ctx);          // 同期
    validatePixelConstraints(data, ctx);    // 同期
    validateSaveOptionsExclusive(data, ctx); // 同期
    await validateTokenCounts(data, ctx);   // 非同期 (T5トークナイザー)
  });
```

設計ポイント:
- **parseAsync 必須**: トークンカウントが非同期のため `.parseAsync()` で呼び出す必要がある
- **部分的バリデーション関数**: 各検証ロジックを独立関数に分離し、テスタビリティを確保
- **トークナイザー障害時のグレースフル劣化**: トークナイザーが利用不能な場合はバリデーションをスキップ (警告ログのみ)
- **共通バリデータ**: `validateSaveOptionsExclusive` は Generate / EncodeVibe / Augment / Upscale で共用

## 画像入力の抽象化

`utils.ts:getImageBuffer` が画像入力の自動判別を行う:

```
入力 ─┬─ Buffer → そのまま返却
      ├─ Uint8Array → Buffer.from()
      └─ string ─┬─ looksLikeFilePath() = true → fs.readFileSync()
                  └─ looksLikeFilePath() = false → Base64デコード
```

### `looksLikeFilePath` の判別ロジック

1. `data:` プレフィックス → false (Data URL)
2. 64文字超のBase64パターン → false
3. `/` 始まり + 画像拡張子 or 2段以上のパス → true (Unix絶対パス)
4. `X:\` パターン → true (Windows絶対パス)
5. パス区切り文字 + 画像拡張子 → true (相対パス)
6. 画像拡張子のみ → true
7. パス区切り文字あり → true
8. それ以外 → false (Base64と判断)

## セキュリティモデル

### パストラバーサル防御 (3層)

1. **Zodスキーマ層** (`SafePathSchema`): `..` を含むパスを拒否
2. **画像入力層** (`ImageInputSchema`): ファイルパス文字列の `..` チェック (Data URL/Base64はスキップ)
3. **ファイル書き込み層** (`client.ts:validateSavePath`): `path.normalize()` 後の `..` チェック

### ZIPボム防御

- `MAX_ZIP_ENTRIES = 10`: エントリ数制限
- `MAX_DECOMPRESSED_IMAGE_SIZE = 50MB`: 展開後サイズ制限
- `MAX_COMPRESSION_RATIO = 100`: 圧縮比制限

### レスポンスサイズ制限

- `MAX_RESPONSE_SIZE = 200MB`: Content-Length チェック + バッファサイズチェック

## トークナイザー戦略

詳細は [docs/tokenizer-internals.md](./tokenizer-internals.md) を参照。

概要:
- **T5 (メイン)**: プロンプトのトークンカウントに使用。native `tokenizers` パッケージ → PureJSUnigram フォールバック
- **CLIP (補助)**: BPEアルゴリズム。純JavaScript実装のみ
- **キャッシュ**: ディスク7日TTL + メモリシングルトン (Promise キャッシュで並行リクエスト防止)

## ブラウザ互換 (anlas-browser.ts)

`anlas-browser.ts` は Anlas コスト計算機能をブラウザで使うためのエントリーポイント:

```bash
pnpm exec esbuild src/anlas-browser.ts --bundle --format=iife --global-name=Anlas --outfile=../anlas.bundle.js
```

- `anlas.ts` の全エクスポート + コスト計算に必要な定数を再エクスポート
- Node.js 依存 (fs, sharp, axios 等) を含まないため、ブラウザバンドル可能
- 用途: フロントエンドでのリアルタイムコスト表示

## 移植チェックリスト

別言語に移植する際の推奨実装順序:

### フェーズ1: 基盤

1. **constants** — 定数定義。依存なし
2. **anlas** — コスト計算。constants のみ依存。テストで正確性を検証可能

### フェーズ2: バリデーション

3. **schemas** — 入力バリデーション。Zodの代替が必要 (言語により構造体+手動バリデーション)
   - 注意: 非同期トークンカウントは後回しにしてよい

### フェーズ3: 画像処理

4. **utils** — 画像リサイズ (sharp の代替が必要), Base64変換, パス判別
   - マスクの1/8リサイズ、キャラクター参照画像のアスペクト比別リサイズが重要

### フェーズ4: トークナイザー (オプション)

5. **tokenizer** — CLIP BPE + T5 Unigram/Viterbi。最も複雑
   - 移植が困難な場合、トークンカウントをスキップしてサーバー側エラーに依存する戦略もあり

### フェーズ5: クライアント

6. **client** — HTTP通信 + ペイロード構築 + レスポンスパース
   - ペイロード構造は [docs/api-protocol.md](./api-protocol.md) を参照
   - ZIP / msgpack / raw PNG の3形式パースが必要
