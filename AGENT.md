# NovelAI APIクライアント (TypeScript版) - AI Agent向けコンテキストドキュメント

このドキュメントは、コンテキストなしの別AIがこのプロジェクトを理解するためのリファレンスです。

---

## プロジェクト概要

**目的**: NovelAI の画像生成 API を扱う TypeScript クライアントライブラリ

**主な機能**:
- テキストから画像生成 (txt2img)
- 画像から画像生成 (img2img)
- インペイント (inpaint)
- Vibe Transfer（スタイル参照）
- マルチキャラクター生成（最大6体）
- キャラクター参照 (Character Reference / Director Reference)
- Vibeエンコード（画像→.naiv4vibeファイル変換）
- T5トークナイザー（プロンプトトークンカウント）

---

## テクノロジースタック

| カテゴリ | 技術 | バージョン |
|---------|------|----------|
| 言語 | TypeScript | ^5.9.3 |
| バリデーション | Zod | ^3.23.8 |
| 画像処理 | sharp | ^0.34.5 |
| HTTPクライアント | axios | ^1.13.2 |
| バイナリパース | msgpackr | ^1.11.8 |
| ZIPパース | adm-zip | ^0.5.16 |
| トークナイザー | tokenizers | ^0.13.3 |
| 環境変数 | dotenv | ^17.2.3 |
| HTMLデコード | he | ^1.2.0 |
| ランタイム | tsx | ^4.21.0 |
| テスト | vitest | ^4.0.16 |
| パッケージ管理 | pnpm | - |

---

## ディレクトリ構造

```
novelAi/
├── .env                          # APIキー（NOVELAI_API_KEY）
├── AGENT.md                      # ←このドキュメント
├── ts-api/                       # TypeScriptパッケージ ★
│   ├── package.json              # 依存関係定義
│   ├── tsconfig.json             # TypeScript設定
│   ├── vitest.config.ts          # テスト設定
│   ├── .env                      # ts-api用環境変数
│   │
│   ├── src/                      # コアライブラリ
│   │   ├── client.ts             # NovelAIClientクラス
│   │   ├── schemas.ts            # Zodスキーマ・型定義
│   │   ├── constants.ts          # 定数・デフォルト値
│   │   ├── utils.ts              # ユーティリティ関数
│   │   └── tokenizer.ts          # T5トークナイザー
│   │
│   ├── example.ts                # 基本使用例
│   ├── example_inpaint.ts        # インペイント使用例
│   ├── example_tokenizer.ts      # トークナイザー使用例
│   │
│   ├── tests/                    # テストファイル
│   │   └── schemas.test.ts       # スキーマバリデーションテスト
│   │
│   ├── output/                   # 生成画像出力先
│   ├── vibes/                    # .naiv4vibeファイル置き場
│   └── reference/                # 参照画像置き場
│
└── novelAI_offecial_folder/      # NovelAI公式リソース（参考）
```

---

## ts-api/src/ ファイル詳細

### 1. `client.ts` - メインクライアント

**クラス**: `NovelAIClient`

NovelAI画像生成の全機能を統合したクライアントクラス。

```typescript
class NovelAIClient {
  constructor(apiKey?: string)
}
```

| 引数 | 型 | 説明 |
|-----|---|------|
| `apiKey` | `string \| undefined` | APIキー。省略時は環境変数 `NOVELAI_API_KEY` から取得 |

---

#### メソッド: `getAnlasBalance()`

残りアンラス（課金単位）を取得。

**戻り値**: `Promise<AnlasBalance>`
```typescript
interface AnlasBalance {
  fixed: number;      // サブスクリプション付属アンラス
  purchased: number;  // 追加購入アンラス
  total: number;      // 合計
  tier: number;       // プラン（0:なし, 1:Tablet, 2:Scroll, 3:Opus）
}
```

---

#### メソッド: `encodeVibe()`

画像をVibe Transfer用にエンコード（**2 Anlas消費**）。

```typescript
encodeVibe(params: EncodeVibeParams): Promise<VibeEncodeResult>
```

**引数** (`EncodeVibeParams`):

| プロパティ | 型 | デフォルト | 説明 |
|-----------|---|----------|------|
| `image` | `string \| Buffer` | 必須 | 画像パス、バイトデータ、Base64文字列 |
| `model` | `string` | `"nai-diffusion-4-5-full"` | モデル名 |
| `information_extracted` | `number` | `0.7` | 抽出情報量 (0.0-1.0) |
| `strength` | `number` | `0.7` | 推奨Vibe強度 |
| `save_path` | `string` | `undefined` | 保存先パス |
| `save_dir` | `string` | `undefined` | 保存先ディレクトリ（自動命名） |
| `save_filename` | `string` | `undefined` | 保存ファイル名（save_dir必須） |

**戻り値**: `Promise<VibeEncodeResult>`

---

#### メソッド: `saveVibe()`

VibeEncodeResultを.naiv4vibeファイルとして保存。

```typescript
saveVibe(result: VibeEncodeResult, savePath: string): void
```

---

#### メソッド: `generate()`

統合画像生成メソッド。txt2img、img2img、inpaint、Vibe、マルチキャラクター、キャラクター参照すべてに対応。

```typescript
generate(params: GenerateParams): Promise<GenerateResult>
```

**主要引数** (`GenerateParams`):

| プロパティ | 型 | デフォルト | 説明 |
|-----------|---|----------|------|
| `prompt` | `string` | 必須 | メインプロンプト（キャラ無し時）/ 背景プロンプト（キャラ有り時） |
| `action` | `"generate" \| "img2img" \| "infill"` | `"generate"` | 生成モード（infill=インペイント） |
| `source_image` | `string \| Buffer` | `undefined` | img2img/infill入力画像 |
| `img2img_strength` | `number` | `0.62` | img2img変換強度（0=元画像に近い） |
| `img2img_noise` | `number` | `0.0` | img2imgノイズ追加量 |
| `mask` | `string \| Buffer` | `undefined` | インペイントマスク画像（白=再生成、黒=保持） |
| `mask_strength` | `number` | **必須（infill時）** | マスク反映度（0.01-1.0）|
| `hybrid_img2img_strength` | `number` | `undefined` | ハイブリッドモード時の元画像維持率（0.01-0.99） |
| `hybrid_img2img_noise` | `number` | `undefined` | ハイブリッドモード時の元画像ノイズ（0-0.99） |
| `inpaint_color_correct` | `boolean` | `true` | インペイント時の色補正 |
| `characters` | `CharacterConfig[]` | `undefined` | キャラクター配置設定（最大6体） |
| `vibes` | `(string \| VibeEncodeResult)[]` | `undefined` | Vibeファイルリスト（最大10個） |
| `vibe_strengths` | `number[]` | `undefined` | 各Vibeの強度（省略時0.7） |
| `vibe_info_extracted` | `number[]` | `undefined` | 各Vibeの情報抽出量 |
| `character_reference` | `CharacterReferenceConfig` | `undefined` | キャラ参照（5 Anlas消費） |
| `negative_prompt` | `string` | デフォルトネガティブ | ネガティブプロンプト |
| `model` | `string` | `"nai-diffusion-4-5-full"` | モデル名 |
| `width` | `number` | `832` | 幅（64の倍数） |
| `height` | `number` | `1216` | 高さ（64の倍数） |
| `steps` | `number` | `23` | ステップ数（1-50） |
| `scale` | `number` | `5.0` | CFG Scale（0.0-10.0） |
| `seed` | `number` | `undefined` | シード値（undefinedでランダム） |
| `sampler` | `string` | `"k_euler_ancestral"` | サンプラー |
| `noise_schedule` | `string` | `"karras"` | ノイズスケジュール |
| `save_path` | `string` | `undefined` | 保存先パス |
| `save_dir` | `string` | `undefined` | 保存先ディレクトリ |

**戻り値**: `Promise<GenerateResult>`

---

#### メソッド: `saveImage()`

GenerateResultを画像ファイルとして保存。

```typescript
saveImage(result: GenerateResult, savePath: string): void
```

---

### 2. `schemas.ts` - Zodスキーマ・型定義

Zodによるバリデーション付きデータモデル。

#### `CharacterConfig`
マルチキャラクター生成時の各キャラクター設定。

```typescript
const CharacterConfigSchema = z.object({
  prompt: z.string().min(1).max(2000),
  center_x: z.number().min(0.0).max(1.0).default(0.5),
  center_y: z.number().min(0.0).max(1.0).default(0.5),
  negative_prompt: z.string().max(2000).default(""),
});
type CharacterConfig = z.infer<typeof CharacterConfigSchema>;
```

**ヘルパー関数**:
- `characterToCaptionDict(config)` - キャプション形式に変換
- `characterToNegativeCaptionDict(config)` - ネガティブキャプション形式に変換

---

#### `CharacterReferenceConfig`
キャラクター参照（Director Reference）設定。**5 Anlas消費**。

```typescript
const CharacterReferenceConfigSchema = z.object({
  image: ImageInputSchema,  // string | Buffer
  fidelity: z.number().min(0.0).max(1.0).default(1.0),
  include_style: z.boolean().default(true),
});
type CharacterReferenceConfig = z.infer<typeof CharacterReferenceConfigSchema>;
```

---

#### `VibeEncodeResult`
Vibeエンコード結果。

```typescript
const VibeEncodeResultSchema = z.object({
  encoding: z.string(),                    // Base64エンコード済みVibeデータ
  model: z.string(),                       // 使用モデル
  information_extracted: z.number().min(0).max(1),
  strength: z.number().min(0).max(1),
  source_image_hash: z.string().regex(/^[a-f0-9]{64}$/),  // SHA256
  created_at: z.date(),
  saved_path: z.string().optional().nullable(),
  anlas_remaining: z.number().min(0).optional().nullable(),
  anlas_consumed: z.number().min(0).optional().nullable(),
});
type VibeEncodeResult = z.infer<typeof VibeEncodeResultSchema>;
```

---

#### `GenerateResult`
画像生成結果。

```typescript
const GenerateResultSchema = z.object({
  image_data: z.instanceof(Buffer),
  seed: z.number().min(0).max(4294967295),
  anlas_remaining: z.number().min(0).optional().nullable(),
  anlas_consumed: z.number().min(0).optional().nullable(),
  saved_path: z.string().optional().nullable(),
});
type GenerateResult = z.infer<typeof GenerateResultSchema>;
```

---

#### `GenerateParamsSchema`
generate()引数のバリデーション。複雑なバリデーションルールを含む。

**主なバリデーション**:
- `width × height ≤ 1,048,576` (MAX_PIXELS)
- `width/height` は64の倍数
- **プロンプト合計トークン数 ≤ 512**（ベースプロンプト + 全キャラクタープロンプトの合計）
- `save_path` と `save_dir` は同時指定不可
- `vibes` と `character_reference` は同時使用不可

---

### 3. `constants.ts` - 定数定義

#### API URL
```typescript
export const API_URL = "https://image.novelai.net/ai/generate-image";
export const STREAM_URL = "https://image.novelai.net/ai/generate-image-stream";
export const ENCODE_URL = "https://image.novelai.net/ai/encode-vibe";
export const SUBSCRIPTION_URL = "https://api.novelai.net/user/subscription";
```

#### デフォルト値
```typescript
export const DEFAULT_MODEL = "nai-diffusion-4-5-full";
export const DEFAULT_WIDTH = 832;
export const DEFAULT_HEIGHT = 1216;
export const DEFAULT_STEPS = 23;
export const DEFAULT_SCALE = 5.0;
export const DEFAULT_SAMPLER = "k_euler_ancestral";
export const DEFAULT_NOISE_SCHEDULE = "karras";
export const DEFAULT_VIBE_STRENGTH = 0.7;
export const DEFAULT_VIBE_INFO_EXTRACTED = 0.7;
export const DEFAULT_IMG2IMG_STRENGTH = 0.62;
export const DEFAULT_INPAINT_STRENGTH = 0.7;
export const DEFAULT_INPAINT_NOISE = 0;
export const DEFAULT_INPAINT_COLOR_CORRECT = true;

export const DEFAULT_NEGATIVE = 
  "nsfw, lowres, artistic error, film grain, scan artifacts, " +
  "worst quality, bad quality, jpeg artifacts, very displeasing, " +
  "chromatic aberration, dithering, halftone, screentone";

export const DEFAULT_INPAINT_COLOR_CORRECT = true;
```

#### 有効な値
```typescript
export const VALID_MODELS = [
  "nai-diffusion-4-curated-preview",
  "nai-diffusion-4-full",
  "nai-diffusion-4-5-curated",
  "nai-diffusion-4-5-full",  // 推奨
] as const;

export const VALID_SAMPLERS = [
  "k_euler",
  "k_euler_ancestral",  // デフォルト
  "k_dpmpp_2s_ancestral",
  "k_dpmpp_2m_sde",
  "k_dpmpp_2m",
  "k_dpmpp_sde",
] as const;

export const VALID_NOISE_SCHEDULES = [
  "native", "karras", "exponential", "polyexponential"
] as const;

export const MODEL_KEY_MAP: Record<string, string> = {
  "nai-diffusion-4-curated-preview": "v4curated",
  "nai-diffusion-4-full": "v4full",
  "nai-diffusion-4-5-curated": "v4-5curated",
  "nai-diffusion-4-5-full": "v4-5full",
};
```

#### 制限値
```typescript
export const MAX_PROMPT_CHARS = 2000;
export const MAX_TOKENS = 512;
export const MAX_PIXELS = 1_048_576;       // width × height 上限
export const MIN_DIMENSION = 64;
export const MAX_DIMENSION = 1024;
export const MAX_CHARACTERS = 6;
export const MAX_VIBES = 10;               // 5個以上は1Vibeあたり2Anlas
export const MIN_STEPS = 1;
export const MAX_STEPS = 50;
export const MIN_SCALE = 0.0;
export const MAX_SCALE = 10.0;
export const MAX_SEED = 4294967295;        // 2^32 - 1
export const MAX_REF_IMAGE_SIZE_MB = 10;
export const MAX_REF_IMAGE_DIMENSION = 4096;
```

---

### 4. `utils.ts` - ユーティリティ関数

#### 画像ヘルパー

| 関数 | 引数 | 戻り値 | 説明 |
|------|------|--------|------|
| `getImageBuffer(image)` | `string \| Buffer` | `Buffer` | 画像データをBufferに変換 |
| `getImageBase64(image)` | `string \| Buffer` | `string` | 画像をBase64文字列に変換 |

#### Vibeヘルパー

| 関数 | 引数 | 戻り値 | 説明 |
|------|------|--------|------|
| `loadVibeFile(vibePath)` | `string` | `any` | .naiv4vibeファイル読み込み |
| `extractEncoding(vibeData, model)` | `any, string` | `{ encoding, information_extracted }` | Vibeデータからエンコード抽出 |
| `processVibes(vibes, model)` | `Array, string` | `Promise<{ encodings[], info_extracted_list[] }>` | Vibeリストをエンコードリストに変換 |

#### キャラクター参照ヘルパー

| 関数 | 引数 | 戻り値 | 説明 |
|------|------|--------|------|
| `prepareCharacterReferenceImage(imageBuffer)` | `Buffer` | `Promise<Buffer>` | キャラ参照画像をリサイズ・パディング |
| `processCharacterReferences(refs)` | `CharacterReferenceConfig[]` | `Promise<{ images, descriptions, ... }>` | キャラ参照をAPI用パラメータに変換 |

#### マスク・インペイントヘルパー

| 関数 | 説明 |
|------|------|
| `calculateCacheSecretKey(imageData)` | 画像のSHA256ハッシュを計算 |
| `resizeMaskImage(mask, width, height)` | マスク画像を1/8サイズにリサイズ |
| `createRectangularMask(width, height, region)` | 矩形マスク画像を生成 |
| `createCircularMask(width, height, center, radius)` | 円形マスク画像を生成 |

---

### 5. `tokenizer.ts` - T5トークナイザー

NovelAI公式と同一のT5トークナイザー実装。**キャッシュ機能**によりネットワークリクエストを最小化。

#### キャッシュ機能

- **ディスクキャッシュ**: `ts-api/.cache/tokenizers/` にトークナイザー定義を保存
- **メモリキャッシュ**: シングルトンパターンで同一プロセス内での再取得を回避
- キャッシュは自動的に作成・管理されます

#### クラス: `NovelAIT5Tokenizer`

```typescript
class NovelAIT5Tokenizer {
  // ファクトリメソッド（コンストラクタはプライベート）
  static async create(tokenizer: Tokenizer): Promise<NovelAIT5Tokenizer>
  
  encode(text: string): Promise<number[]>    // トークンIDリストを返す（EOS含む）
  countTokens(text: string): Promise<number> // トークン数を返す（表示用、EOS除く）
}
```

#### エラークラス

| クラス | 説明 |
|--------|------|
| `TokenizerError` | トークナイザーの初期化・取得時のエラー |
| `TokenValidationError` | トークン数が上限を超えた場合のエラー（`tokenCount`, `maxTokens` プロパティ付き） |

#### エクスポート関数

| 関数 | 説明 |
|------|------|
| `getT5Tokenizer(forceRefresh?)` | シングルトンのT5トークナイザーを取得。`forceRefresh=true` でキャッシュを無視 |
| `getClipTokenizer(forceRefresh?)` | CLIPトークナイザーを取得 |
| `preprocessT5(text)` | T5用にテキストを前処理（括弧・ウェイト構文を削除） |
| `validateTokenCount(text)` | トークン数が512以下かバリデーション |
| `clearTokenizerCache()` | メモリキャッシュをクリア（テスト用） |

#### クラス: `NovelAIClipTokenizer`

CLIPトークナイザー実装（内部使用）。

---

## 使用例

### 基本的な画像生成
```typescript
import "dotenv/config";
import { NovelAIClient } from "./src/client";

const client = new NovelAIClient();
const result = await client.generate({
  prompt: "1girl, beautiful anime girl, masterpiece",
  save_dir: "output/",
});
console.log(`Saved: ${result.saved_path}, Seed: ${result.seed}`);
```

### マルチキャラクター
```typescript
import { NovelAIClient } from "./src/client";
import { CharacterConfig } from "./src/schemas";

const characters: CharacterConfig[] = [
  { prompt: "1girl, blonde hair", center_x: 0.3, center_y: 0.5, negative_prompt: "" },
  { prompt: "1boy, black hair", center_x: 0.7, center_y: 0.5, negative_prompt: "" },
];

const result = await client.generate({
  prompt: "school classroom, sunny day",
  characters,
  width: 1024,
  height: 1024,
});
```

### Vibe Transfer
```typescript
const result = await client.generate({
  prompt: "1girl, beautiful",
  vibes: ["style.naiv4vibe"],
  vibe_strengths: [0.7],
});
```

### Image2Image
```typescript
const result = await client.generate({
  prompt: "1girl, anime style",
  action: "img2img",
  source_image: "input.png",
  img2img_strength: 0.6,
});
```

### インペイント（Infill）
```typescript
import { createRectangularMask } from "./src/utils";

// 全体の左半分を塗り替える矩形マスクを生成
const mask = await createRectangularMask(832, 1216, { x: 0, y: 0, w: 0.5, h: 1.0 });

const result = await client.generate({
  prompt: "1girl, red hair",
  action: "infill",  // ← 'inpaint' ではなく 'infill'
  source_image: "original.png",
  mask: mask,
  mask_strength: 0.7,  // 必須：マスク反映度
});
```

### インペイント + Img2Img ハイブリッドモード
```typescript
// マスク領域を再生成しつつ、元画像の特徴も維持
const result = await client.generate({
  prompt: "1girl, beautiful dress",
  action: "infill",
  source_image: "original.png",
  mask: mask,
  mask_strength: 0.68,              // マスク反映度
  hybrid_img2img_strength: 0.45,    // 元画像維持率（0.01-0.99）
  hybrid_img2img_noise: 0,          // 元画像ノイズ
});

### キャラクター参照（5 Anlas）
```typescript
import { CharacterReferenceConfig } from "./src/schemas";

const result = await client.generate({
  prompt: "1girl, standing",
  character_reference: {
    image: "reference.png",
    fidelity: 1.0,
    include_style: true,
  },
});
```

### トークナイザー
```typescript
import { getT5Tokenizer, validateTokenCount } from "./src/tokenizer";

const tokenizer = await getT5Tokenizer();
const count = await tokenizer.countTokens("1girl, beautiful");
console.log(`Token count: ${count}`);

// バリデーション（512超えでエラー）
await validateTokenCount("your prompt here");
```

---

## 注意事項

1. **APIキー**: `.env` に `NOVELAI_API_KEY=pst-xxx` を設定
2. **アンラス消費**:
   - 通常生成: 無料（Opusプラン、1024×1024以下、steps≤28）
   - Vibeエンコード: 2 Anlas
   - キャラクター参照: 5 Anlas
   - Vibe 5個以上: 1Vibeあたり2 Anlas追加
3. **Vibeとキャラクター参照は同時使用不可**
4. **画像サイズ**: 64の倍数、width×height ≤ 1,048,576
5. **プロンプト**: ベース + 全キャラクタープロンプトの合計が最大512トークン（T5トークナイザー）
6. **非同期バリデーション**: `GenerateParamsSchema` のパースは `safeParseAsync()` を使用
7. **同時実行禁止**: NovelAI APIは同時実行をサポートしていません（429エラー）
8. **参照画像サイズ**: 最大10MB（`MAX_REF_IMAGE_SIZE_MB`）

---

## 自動リトライ機能

クライアントは429エラー（レート制限/同時実行ロック）に対して自動リトライを行います。

```typescript
// 設定（client.ts 内部）
private readonly maxRetries = 3;
private readonly baseRetryDelayMs = 1000;
```

| リトライ | 待機時間 |
|----------|----------|
| 1回目 | 1秒 |
| 2回目 | 2秒 |
| 3回目 | 4秒 |

**対象メソッド**:
- `generate()` / `augmentImage()` / `upscaleImage()`

---

## テスト実行

```bash
cd ts-api
pnpm install
pnpm test
```

### 使用例の実行

```bash
cd ts-api
pnpm exec tsx example.ts
pnpm exec tsx example_inpaint.ts
pnpm exec tsx example_tokenizer.ts
```

---

## トラブルシューティング

### "Async refinement encountered during synchronous parse operation"
→ `GenerateParamsSchema.parse()` の代わりに `GenerateParamsSchema.safeParseAsync()` を使用

### "Cannot find module 'tokenizers'"
→ `pnpm install` を実行。Linux/WSL環境では `tokenizers-linux-x64-gnu` が必要

### 画像サイズエラー
→ `width × height ≤ 1,048,576` かつ64の倍数であることを確認

### "429 Too Many Requests" / "Concurrent generation is locked"
→ NovelAI APIは同時実行を許可していません。クライアントは自動リトライ（最大3回、指数バックオフ）を行います。並列でリクエストを送信すると一部が失敗する可能性があります。

### "Image file size exceeds maximum allowed size"
→ 参照画像のファイルサイズが10MBを超えています。画像を圧縮するか、解像度を下げてください。
