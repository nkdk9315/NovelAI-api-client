# NovelAI アンラス消費計算ロジック 完全解析

## 概要

NovelAI フロントエンドの minified JS チャンクから、画像生成時のアンラス（サイト内通貨）消費計算ロジックを完全に特定・復元した。

**ソースファイル**: `pages/_app-3519e562baaa896c.js` (メインバンドル)
**対象モジュール位置**: _app ファイル内 position 879000〜899000

---

## 1. 通常画像生成コスト

### 1.1 メイン関数 `eX`（エクスポート名: `GI`）

```
合計コスト = 1枚あたりコスト × 課金対象枚数
```

#### V4/V4.5 モデルの1枚あたりコスト計算式（関数 `o`）

**現行モデル（nai-diffusion-4-5-full 等）はすべてこのパスを使用。**

```javascript
function o(width, height, steps, sm, sm_dyn) {
    let pixels = width * height;
    let baseCost = Math.ceil(
        2.951823174884865e-6 * pixels + 5.753298233447344e-7 * pixels * steps
    );
    let multiplier = (sm && sm_dyn) ? 1.4 : sm ? 1.2 : 1.0;
    return baseCost * multiplier;
}
```

**数式表記:**

```
baseCost = ⌈ (2.951823174884865×10⁻⁶ × W×H) + (5.753298233447344×10⁻⁷ × W×H × steps) ⌉

perImageCost = baseCost × smMultiplier

smMultiplier:
  - SMEA + SMEA Dynamic: 1.4
  - SMEA only:           1.2
  - なし:                1.0
```

#### img2img / inpaint 時の補正

```
strengthMultiplier:
  - txt2img:  1.0
  - img2img:  strength値（デフォルト 0.62）
  - inpaint:  inpaintImg2ImgStrength値（デフォルト 1.0）

adjustedCost = max(⌈ perImageCost × strengthMultiplier ⌉, 2)
```

**最低コスト**: 2 Anlas/枚（adjustedCost の下限）
**最大コスト上限**: 140 Anlas/枚を超えると -3（エラー）を返す

#### 最終合計

```
課金対象枚数 = n_samples - (Opus無料の場合 ? 1 : 0)
最終コスト = adjustedCost × 課金対象枚数
```

### 1.2 Inpainting 時のサイズ補正（関数 `tr`、エクスポート名: `re`）

inpaint 時、マスクサイズからコスト計算用のサイズが再計算される:

```javascript
function re(width, height, maxPixels, gridSize) {
    // width = 8 * mask_width, height = 8 * mask_height
    let pixels = width * height;
    if (pixels >= 0.8 * maxPixels) return {width, height};
    let scale = Math.sqrt(maxPixels / pixels);
    return {
        width:  Math.floor(Math.floor(width * scale) / gridSize) * gridSize,
        height: Math.floor(Math.floor(height * scale) / gridSize) * gridSize
    };
}
// maxPixels = 1048576, gridSize = 64
```

マスク領域のピクセル数が `1048576 × 0.8 = 838,861` 未満の場合、1048576 を満たすようにスケールアップされ、64の倍数にスナップされる。

---

## 2. Opus 無料判定ロジック

### 関数 `eW`（エクスポート名: `t1`）

```javascript
function isOpusFree(settings) {
    return !settings.characterRef
        && settings.width * settings.height <= 1048576
        && settings.steps <= 28;
}
```

**Opus 無料条件（すべて満たす必要あり）:**

| 条件 | 値 |
|------|-----|
| キャラクター参照 | 使用していない |
| 画像ピクセル数 (width × height) | ≤ 1,048,576 (= 1024 × 1024) |
| ステップ数 | ≤ 28 |
| サブスクリプションティア | ≥ 3 (Opus) |
| サブスクリプション有効 | `(0,s.ax)(subscription)` が true |

**Opus 無料時の挙動:**
- `n_samples` から 1 を引く（1枚目が無料）
- 2枚以上生成する場合、2枚目以降は通常料金
- Vibe エンコード、キャラクター参照、Vibe 5個以上のコストは Opus でも発生

---

## 3. 追加コスト

### 3.1 Vibe Transfer コスト

**個別 Vibe エンコードコスト:**
```
各 Vibe について:
  - 既にエンコード済み (.naiv4vibe): 0 Anlas
  - 未エンコード（画像）でキャッシュあり: 0 Anlas
  - 未エンコード（画像）でキャッシュなし: 2 Anlas
```

**Vibe バッチコスト（関数 `tz`、エクスポート名: `H_`）:**
```javascript
const VIBE_BATCH_PRICE = 2;
function vibeBatchCost(enabledVibeCount) {
    return Math.max(0, enabledVibeCount - 4) * VIBE_BATCH_PRICE;
}
```

| 有効 Vibe 数 | バッチコスト |
|-------------|------------|
| 1〜4個 | 0 Anlas |
| 5個 | 2 Anlas |
| 6個 | 4 Anlas |
| N個 (N≥5) | (N - 4) × 2 Anlas |

**Vibe コスト発生条件:**
- Vibe が存在する
- `encodedVibes` 機能がモデルで有効
- キャラクター参照を使用していない（Vibe とキャラ参照は排他）
- inpaint モードではない（mask がない）

### 3.2 キャラクター参照コスト

```
characterRefCost = 5 × キャラクター参照数 × n_samples
```

**発生条件:**
- キャラクター参照が存在する
- `characterReferences` 機能がモデルで有効
- inpaint モードでない、または `charRefInpainting` 機能が有効

### 3.3 総合コスト計算（関数 `sf` in `3893-2af421c99b3cf421.js`）

```
additionalPrice = 0

// Vibe コスト（Vibe あり、キャラ参照なし、inpaint でない場合のみ）
if (vibes.length > 0 && encodedVibesEnabled && charRefs.length === 0 && !mask):
    additionalPrice += Σ getPrice(vibe)   // 各 Vibe のエンコードコスト
    additionalPrice += vibeBatchCost(enabledVibeCount)  // バッチコスト

// キャラクター参照コスト
if (charRefs.length > 0 && characterReferencesEnabled && (!mask || charRefInpaintingEnabled)):
    additionalPrice += 5 × charRefs.length × n_samples

// 最終コスト
totalPrice = GI(settings, user, model) + additionalPrice
```

---

## 4. Augment ツール（画像編集）コスト

### 4.1 ツール一覧と基本コスト

**ソースファイル**: `814-e073566363208847.js`

Augment ツールは `naiDiffusionV3` モデルを指定するが、`nai-diffusion-3` は `stableDiffusionXL` ファミリーにマッピングされるため、**V4 計算式が適用される**。
サイズは `1048576` ピクセル以上に拡大される（Rj関数）。

| ツール | req_type | Opus 無料? | コスト計算 |
|-------|----------|-----------|-----------|
| 背景削除 | `bg-removal` | **常に有料** | `3 × baseCost + 5` |
| 線画抽出 | `lineart` | 条件付き無料 | `baseCost` |
| スケッチ | `sketch` | 条件付き無料 | `baseCost` |
| カラー化 | `colorize` | 条件付き無料 | `baseCost` |
| 表情変更 | `emotion` | 条件付き無料 | `baseCost` |
| デクラッター | `declutter` | 条件付き無料 | `baseCost` |
| アップスケール | `upscale` | 条件付き無料 | 専用テーブル |

### 4.2 Augment ツールのコスト計算フロー

```javascript
// 1. サイズ調整: 3145728 以上なら縮小 (Bu関数)
let {height, width} = Bu(original_width, original_height, 3145728); // max上限

// 2. サイズ調整: 1048576 以下なら拡大 (Rj/e6関数)
({height, width} = Rj(height, width, 1048576)); // 最小ピクセル数まで拡大

// 3. パラメータ固定値
settings.steps = 28;
settings.sm = false;
settings.sm_dyn = false;
settings.n_samples = 1;

// 4. V4計算式でコスト計算（nai-diffusion-3 → stableDiffusionXL → V4パス）
let baseCost = GI(settings, user, naiDiffusionV3, isBgRemoval);
// → 内部で V4式: ceil(2.951823e-6 * px + 5.753298e-7 * px * 28)

// 5. bgRemoval の特別計算
if (tool === 'bg-removal') {
    finalCost = 3 * baseCost + 5;
}
```

#### Rj関数（e6）の実装

```javascript
function e6(e, t, r) {
    let n = e * t;
    if (n >= r) return {width: e, height: t};  // 既に大きければそのまま
    let i = Math.sqrt(r / n);
    return {width: Math.floor(e * i), height: Math.floor(t * i)};  // 拡大
}
```

- 64グリッドスナップなし（`Math.floor`のみ）
- 1048576px 以下の画像を 1048576px に近づくよう拡大する

### 4.3 Augment Opus 無料条件

```
isOpusFree = (tool !== 'bg-removal') && (width × height ≤ 1048576) && (steps ≤ 28)
```

- **bg-removal は Opus でも常に有料**
- 他のツールは `1024×1024` 以下なら Opus 無料

### 4.4 アップスケールコスト（関数 `e$`、エクスポート名: `tY`）

専用のコストテーブルを使用:

```javascript
const UPSCALE_TABLE = [
    [1048576, 7],
    [786432,  5],
    [524288,  3],
    [409600,  2],
    [262144,  1]
];

function upscaleCost(width, height, user) {
    let pixels = width * height;
    // Opus 無料: 409600px 以下
    if (pixels <= 409600 && user.subscription.tier >= 3 && isSubscriptionActive(user.subscription))
        return 0;
    let cost = -3; // エラー（テーブルに該当なし）
    for (let [threshold, price] of UPSCALE_TABLE) {
        if (pixels <= threshold) cost = price;
    }
    return cost;
}
```

| ピクセル数 (width × height) | コスト | Opus 無料? |
|---------------------------|-------|-----------|
| ≤ 262,144 | 1 Anlas | 無料 |
| ≤ 409,600 | 2 Anlas | 無料 |
| ≤ 524,288 | 3 Anlas | 有料 |
| ≤ 786,432 | 5 Anlas | 有料 |
| ≤ 1,048,576 | 7 Anlas | 有料 |
| > 1,048,576 | エラー (-3) | - |

---

## 5. 検証

### 5.1 デフォルト値での検算 (832×1216, 23steps)

```
pixels = 832 × 1216 = 1,011,712

Term1 = 2.951823174884865e-6 × 1,011,712 = 2.9868
Term2 = 5.753298233447344e-7 × 1,011,712 × 23 = 13.387

baseCost = ⌈2.9868 + 13.387⌉ = ⌈16.374⌉ = 17

smMultiplier = 1.0 (SMEA off)
perImageCost = 17 × 1.0 = 17

strengthMultiplier = 1.0 (txt2img)
adjustedCost = max(⌈17 × 1.0⌉, 2) = 17

Opus 無料判定: 1,011,712 ≤ 1,048,576 ✓, 23 ≤ 28 ✓
課金対象枚数 = 1 - 1 = 0

最終コスト = 17 × 0 = 0 Anlas ✓ (Opus 無料)
非Opus: 17 × 1 = 17 Anlas
```

### 5.2 Opus 無料上限 (1024×1024, 28steps, 1枚)

```
pixels = 1,048,576

Term1 = 2.951823174884865e-6 × 1,048,576 = 3.0946
Term2 = 5.753298233447344e-7 × 1,048,576 × 28 = 16.889

baseCost = ⌈3.0946 + 16.889⌉ = ⌈19.984⌉ = 20

Opus 無料: 1,048,576 ≤ 1,048,576 ✓, 28 ≤ 28 ✓
課金対象枚数 = 1 - 1 = 0

最終コスト = 0 Anlas ✓
```

### 5.3 最大サイズ (2048×1536, 50steps)

```
pixels = 3,145,728

Term1 = 2.951823174884865e-6 × 3,145,728 = 9.284
Term2 = 5.753298233447344e-7 × 3,145,728 × 50 = 90.527

baseCost = ⌈9.284 + 90.527⌉ = ⌈99.811⌉ = 100

Opus 無料: 3,145,728 > 1,048,576 ✗ → 無料ではない
adjustedCost = max(100, 2) = 100
100 ≤ 140 (上限) → OK

最終コスト = 100 × n_samples
```

### 5.4 Opus 2枚生成 (832×1216, 23steps, 2枚)

```
baseCost = 17 (上記と同じ)
Opus 無料判定: ✓
課金対象枚数 = 2 - 1 = 1

最終コスト = 17 × 1 = 17 Anlas (2枚目のみ課金)
```

### 5.5 SMEA 有効時 (832×1216, 23steps)

```
baseCost = 17 (同上)

SMEA のみ: 17 × 1.2 = 20.4 → adjustedCost = max(⌈20.4⌉, 2) = 21
SMEA + Dynamic: 17 × 1.4 = 23.8 → adjustedCost = max(⌈23.8⌉, 2) = 24
```

### 5.6 img2img (832×1216, 23steps, strength=0.62)

```
perImageCost = 17 (同上)
strengthMultiplier = 0.62

adjustedCost = max(⌈17 × 0.62⌉, 2) = max(⌈10.54⌉, 2) = max(11, 2) = 11

最終コスト = 11 × n_samples (Opus判定後)
```

---

## 6. 定数・制限値一覧

| 定数名 | 値 | 用途 |
|--------|-----|------|
| `ef` (MAX_PIXELS) | 3,145,728 | width × height の絶対上限 (2048×1536) |
| `ep` (OPUS_FREE_PIXELS) | 1,048,576 | Opus 無料ピクセル上限 (1024×1024) |
| `g.dZ` (MAX_COST_PER_IMAGE) | 140 | 1枚あたりコスト上限 (超過でエラー) |
| `g.kJ` (MAX_STEPS_UI) | 900 | UI上限（実効はAPI側で50） |
| `g.Hi` | 75 | (未確認 - UI関連) |
| `tJ` (VIBE_BATCH_PRICE) | 2 | Vibe 5個以上の追加単価 |
| `eE` ($d / GRID_SIZE) | 64 | V4モデルの次元グリッドサイズ |
| 最小ピクセル数 | 65,536 | コスト計算時の下限 (256×256相当) |
| CHAR_REF_PRICE | 5 | キャラ参照1件あたりの単価 |

---

## 7. レガシーモデル用コスト関数（参考）

V4 以前のモデル（SD, NAI Diffusion V3 等）には2つの追加計算パスがある。
**注意**: Augment ツールは `naiDiffusionV3` を指定するが、`nai-diffusion-3` は `stableDiffusionXL` ファミリーにマッピングされるため、実際には V4 計算式（関数 `o`）が使用される。以下は純粋なレガシー参考情報。

### 関数 `i`（SD系 + 特定サンプラー + 1048576px 以下）

```javascript
function i(width, height, steps) {
    return (15.266497014243718 * Math.exp(width * height / 1048576 * 0.6326248927474729)
            + -15.225164493059737) / 28 * steps;
}
```

**対象条件**: pixels ≤ 1048576 かつ sampler が PLMS/DDIM/Euler/Euler Ancestral/LMS

### 関数 `a`（SD系 + その他サンプラー）

ステップスケジュールの lookup テーブル `c` を使用する複雑な関数。
サンプラーごとに異なるテーブル（`h`, `u`, `d`, `f`）を参照し、
`floor(width/64) * floor(height/64)` をインデックスとしてコストを算出。

### 関数 `e6`（エクスポート名: `Rj`）- サイズ拡大

Augment ツールで使用。指定ピクセル数以下の画像を拡大する。

```javascript
function e6(e, t, r) {
    let n = e * t;
    if (n >= r) return {width: e, height: t};  // 既に大きければそのまま
    let i = Math.sqrt(r / n);
    return {width: Math.floor(e * i), height: Math.floor(t * i)};  // 拡大
}
// r = 1048576 (Augment用最小ピクセル数)
```

- `clampToMaxPixels`（Bu/e5）とは逆の動作：小さい画像を拡大する
- 64グリッドスナップなし（`Math.floor`のみ）

---

## 8. コード内の関数名マッピング

| Minified名 | エクスポート名 | 機能 |
|------------|--------------|------|
| `eX` | `GI` | メイン生成コスト計算 |
| `eW` | `t1` | Opus 無料判定 |
| `e$` | `tY` | アップスケールコスト |
| `tz` | `H_` | Vibe バッチコスト |
| `ew` | `Dk` | パラメータバリデーション |
| `eE` | `$d` | モデルグリッドサイズ |
| `tr` | `re` | Inpaint サイズ補正 |
| `e5` | `Bu` | サイズ縮小（上限内に収める） |
| `e6` | `Rj` | サイズ拡大（下限まで拡張） |
| `sf` | (local) | 総合コスト計算 (3893チャンク) |

---

## 9. ソースコード位置

| ファイル | 位置 | 内容 |
|---------|------|------|
| `_app-3519e562baaa896c.js:879060` | function `eX` | V4 コスト計算関数 |
| `_app-3519e562baaa896c.js:879000` | function `eW` | Opus 無料判定 |
| `_app-3519e562baaa896c.js:880600` | `eZ`, `e$` | アップスケールテーブル・関数 |
| `_app-3519e562baaa896c.js:898172` | function `tz` | Vibe バッチコスト |
| `_app-3519e562baaa896c.js:859511` | function `ew` | パラメータバリデーション |
| `3893-2af421c99b3cf421.js:637000` | function `sf` | 総合コスト集計 |
| `814-e073566363208847.js:58230` | augment callback | Augment ツールコスト |
| `814-e073566363208847.js:58730` | bgRemoval callback | BG除去特別計算 |
| `814-e073566363208847.js:77467` | isOpusFree | Augment Opus無料判定 |
| `5248-4846c9ca7075ad68.js` | `getPrice` | Vibe エンコードコスト |
