# クイックスタートガイド

NovelAI 画像生成 API を TypeScript から利用するためのガイド。

## セットアップ

### 1. インストール

```bash
pnpm install
```

### 2. 環境変数の設定

プロジェクトルートに `.env` ファイルを作成:

```env
NOVELAI_API_KEY=your_api_key_here
```

APIキーは [NovelAI](https://novelai.net/) のアカウント設定から取得できる。

### 3. コード内での初期化

```typescript
import dotenv from 'dotenv';
import { NovelAIClient } from './src/client';

dotenv.config();

// 環境変数から自動取得
const client = new NovelAIClient();

// または直接指定
const client2 = new NovelAIClient("your_api_key");
```

---

## 基本的な画像生成 (txt2img)

```typescript
import { NovelAIClient } from './src/client';
import { ZodError } from 'zod';

const client = new NovelAIClient();

try {
  const result = await client.generate({
    prompt: "1girl, beautiful anime girl, detailed eyes, masterpiece, best quality",
    save_dir: "output/",
  });

  console.log(`保存先: ${result.saved_path}`);
  console.log(`シード値: ${result.seed}`);
  console.log(`消費アンラス: ${result.anlas_consumed}`);
  console.log(`残りアンラス: ${result.anlas_remaining}`);
} catch (e) {
  if (e instanceof ZodError) {
    // バリデーションエラー
    e.issues.forEach(issue => {
      console.error(`${issue.path.join('.')}: ${issue.message}`);
    });
  } else {
    console.error(e);
  }
}
```

### 結果の読み方

`GenerateResult` オブジェクト:

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `image_data` | `Buffer` | PNG画像のバイナリデータ |
| `seed` | `number` | 使用されたシード値 |
| `anlas_consumed` | `number \| null` | 消費したアンラス数 |
| `anlas_remaining` | `number \| null` | 残りのアンラス数 |
| `saved_path` | `string \| null` | 保存したファイルパス |

### サイズ指定

```typescript
const result = await client.generate({
  prompt: "landscape, mountain, sunset",
  width: 1024,    // 64の倍数 (64〜2048)
  height: 1024,
  save_dir: "output/",
});
```

デフォルト: 832x1216 (縦長ポートレート)

---

## Image to Image (img2img)

既存画像をベースに新しい画像を生成する。

```typescript
const result = await client.generate({
  prompt: "1girl, beautiful, masterpiece",
  action: "img2img",
  source_image: "reference/input.jpeg",  // ファイルパス
  img2img_strength: 0.6,   // 変化の強さ (0.0〜1.0, デフォルト: 0.62)
  img2img_noise: 0.1,      // ノイズ量 (0.0〜1.0, デフォルト: 0.0)
  width: 832,
  height: 1216,
  save_dir: "output/",
});
```

画像入力は以下の形式に対応:
- ファイルパス (`"reference/input.jpeg"`)
- Base64文字列
- `Buffer` / `Uint8Array`

---

## Inpaint (部分再生成)

画像の一部をマスクで指定し、その領域だけを再生成する。

```typescript
const result = await client.generate({
  prompt: "1girl, smiling, happy",
  action: "infill",
  source_image: "reference/input.jpeg",
  mask: "reference/mask.png",        // 白=再生成、黒=保持
  mask_strength: 0.7,                // マスク反映度 (0.01〜1.0, 必須)
  width: 832,
  height: 1216,
  save_dir: "output/",
});
```

### プログラムによるマスク生成

ユーティリティ関数で矩形・円形マスクを生成できる:

```typescript
import { createRectangularMask, createCircularMask } from './src/utils';

// 矩形マスク (座標は 0.0〜1.0 の相対値)
const rectMask = await createRectangularMask(832, 1216, {
  x: 0.2, y: 0.3, w: 0.5, h: 0.4,
});

// 円形マスク
const circleMask = await createCircularMask(832, 1216,
  { x: 0.5, y: 0.5 },  // 中心
  0.3,                   // 半径 (幅に対する相対値)
);
```

### ハイブリッドモード (Inpaint + Img2Img)

マスク領域の再生成と同時に、元画像の影響度も制御する:

```typescript
const result = await client.generate({
  prompt: "1girl, beautiful dress",
  action: "infill",
  source_image: "reference/input.jpeg",
  mask: "reference/mask.png",
  mask_strength: 0.68,
  hybrid_img2img_strength: 0.45,  // 元画像維持率 (0.01〜0.99)
  hybrid_img2img_noise: 0,         // 元画像ノイズ (0〜0.99)
  width: 832,
  height: 1216,
  save_dir: "output/",
});
```

---

## Vibe Transfer

エンコード済み画像のスタイルを生成に反映させる。

### ステップ1: Vibeエンコード

画像を `.naiv4vibe` ファイルにエンコードする (2 Anlas消費):

```typescript
const vibeResult = await client.encodeVibe({
  image: "reference/style_image.png",
  information_extracted: 0.5,   // 抽出情報量 (0.0〜1.0, デフォルト: 0.7)
  strength: 0.7,                // 適用強度 (0.0〜1.0, デフォルト: 0.7)
  save_dir: "./vibes",
  save_filename: "my_style",    // → vibes/my_style.naiv4vibe
});
```

### ステップ2: Vibeを使って生成

```typescript
const result = await client.generate({
  prompt: "1girl, standing, outdoor",
  vibes: ["vibes/my_style.naiv4vibe"],
  vibe_strengths: [0.7],       // 各Vibeの適用強度
  save_dir: "output/",
});
```

複数Vibeも指定可能 (最大10個、5個以上は1Vibeあたり追加2Anlas):

```typescript
const result = await client.generate({
  prompt: "1girl",
  vibes: ["vibes/style1.naiv4vibe", "vibes/style2.naiv4vibe"],
  vibe_strengths: [0.5, 0.3],
  save_dir: "output/",
});
```

---

## Character Reference (キャラクター参照)

参照画像のキャラクターを生成に反映させる。

```typescript
const result = await client.generate({
  prompt: "school classroom, sunny day",
  characters: [
    {
      prompt: "1girl, standing",
      center_x: 0.5,    // キャラクター中心X (0.0〜1.0)
      center_y: 0.5,    // キャラクター中心Y (0.0〜1.0)
    },
  ],
  character_reference: {
    image: "reference/character.png",
    strength: 0.6,    // 参照強度 (0.0〜1.0, デフォルト: 0.6)
    fidelity: 0.8,    // 忠実度 (0.0〜1.0, デフォルト: 1.0)
    mode: "character&style",  // "character" | "character&style" | "style"
  },
  save_dir: "output/",
});
```

### モード一覧

| モード | 説明 |
|--------|------|
| `"character"` | キャラクターの外見のみ参照 |
| `"character&style"` | キャラクター + 画風を参照 (デフォルト) |
| `"style"` | 画風のみ参照 |

---

## Augment (画像加工ツール)

6種類の画像加工ツールが利用可能。

```typescript
const result = await client.augmentImage({
  req_type: "colorize",
  image: "reference/mono_image.png",
  prompt: "vibrant colors",   // colorize/emotionのみ
  defry: 3,                    // colorize/emotionのみ (0〜5, 0=最強変更)
  save_dir: "output/augment/",
});
```

### ツール別パラメータ

| ツール | 説明 | prompt | defry |
|--------|------|--------|-------|
| `colorize` | カラー化 | オプション | **必須** (0〜5) |
| `emotion` | 表情変換 | **必須** (キーワード) | **必須** (0〜5) |
| `declutter` | 不要要素除去 | 使用不可 | 使用不可 |
| `sketch` | スケッチ化 | 使用不可 | 使用不可 |
| `lineart` | 線画抽出 | 使用不可 | 使用不可 |
| `bg-removal` | 背景除去 | 使用不可 | 使用不可 |

### emotion の有効キーワード

```
neutral, happy, sad, angry, scared, surprised, tired, excited,
nervous, thinking, confused, shy, disgusted, smug, bored,
laughing, irritated, aroused, embarrassed, love, worried,
determined, hurt, playful
```

```typescript
// 表情変換の例
const result = await client.augmentImage({
  req_type: "emotion",
  image: "reference/face.png",
  prompt: "happy",    // キーワード指定 (;;は自動付与)
  defry: 0,           // 最強変更
  save_dir: "output/augment/",
});
```

---

## Upscale (画像拡大)

```typescript
const result = await client.upscaleImage({
  image: "reference/input.jpeg",
  scale: 4,          // 2 or 4 (デフォルト: 4)
  save_dir: "output/",
});

console.log(`出力サイズ: ${result.output_width}x${result.output_height}`);
```

---

## アンラス残高確認

```typescript
const balance = await client.getAnlasBalance();
console.log(`合計: ${balance.total}`);
console.log(`固定: ${balance.fixed}`);
console.log(`購入済み: ${balance.purchased}`);
console.log(`ティア: ${balance.tier}`);  // 0=Free, 1=Tablet, 2=Scroll, 3=Opus
```

---

## エラーハンドリング

### バリデーションエラー (ZodError)

パラメータが不正な場合、`generate()` / `augmentImage()` 等は `ZodError` をスローする。

```typescript
import { ZodError } from 'zod';

try {
  await client.generate({ prompt: "test", width: 100 }); // 64の倍数でない
} catch (e) {
  if (e instanceof ZodError) {
    e.issues.forEach(issue => {
      console.error(`${issue.path.join('.')}: ${issue.message}`);
      // → "width: Width must be a multiple of 64"
    });
  }
}
```

### HTTPエラー

API呼び出しが失敗した場合は `Error` がスローされる。429 (レートリミット) は自動リトライ (最大3回、exponential backoff)。

### リトライ動作

- **リトライ対象**: 429 (Too Many Requests) / ネットワークエラー (タイムアウト, DNS失敗, 接続拒否)
- **リトライ対象外**: 400, 401, 403, 500 等
- **バックオフ**: `1000ms * 2^attempt * (1 + random * 0.3)`
- **最大リトライ**: 3回

---

## よくあるミスと注意点

### 画像サイズは64の倍数

```typescript
// NG: ZodError
{ width: 500, height: 700 }

// OK
{ width: 512, height: 704 }
```

### 最大ピクセル数 (3,145,728 = 2048x1536)

```typescript
// NG: width * height > 3,145,728
{ width: 2048, height: 2048 }  // 4,194,304 > 3,145,728

// OK
{ width: 2048, height: 1536 }  // 3,145,728
```

### vibes と character_reference は同時使用不可

```typescript
// NG: ZodError
{
  vibes: ["vibes/style.naiv4vibe"],
  character_reference: { image: "ref.png" },
}
```

### save_path と save_dir は排他

```typescript
// NG: ZodError
{ save_path: "output/image.png", save_dir: "output/" }

// OK: どちらか一方
{ save_path: "output/image.png" }
{ save_dir: "output/" }
```

### トークン上限は512

ポジティブプロンプト (ベース + 全キャラクター) の合計トークン数が512を超えるとエラー。
ネガティブプロンプトも同様に512が上限。

### action と source_image の整合性

```typescript
// NG: source_image がない
{ action: "img2img", prompt: "test" }

// NG: mask/mask_strength がない
{ action: "infill", source_image: "img.png", prompt: "test" }
```

### emotion の prompt にはキーワードのみ

```typescript
// NG: 自由テキスト
{ req_type: "emotion", prompt: "a very happy face", defry: 3 }

// OK: 定義済みキーワード
{ req_type: "emotion", prompt: "happy", defry: 3 }
```
