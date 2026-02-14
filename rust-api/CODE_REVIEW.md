# rust-api 批判的コードレビュー

> **レビュー日**: 2026-02-14
> **対象**: `rust-api/src/` 全モジュール (約3,800行)
> **レビュアー**: Senior Rust Developer (AI-assisted)

---

## 目次

1. [エグゼクティブサマリー](#1-エグゼクティブサマリー)
2. [Critical (即時対応必須)](#2-critical-即時対応必須)
3. [High (早期対応推奨)](#3-high-早期対応推奨)
4. [Medium (計画的対応)](#4-medium-計画的対応)
5. [Low (改善推奨)](#5-low-改善推奨)
6. [モジュール別詳細分析](#6-モジュール別詳細分析)
7. [アーキテクチャ全体の改善提案](#7-アーキテクチャ全体の改善提案)

---

## 1. エグゼクティブサマリー

| 深刻度 | 件数 | 主な領域 |
|--------|------|----------|
| **Critical** | 5 | UTF-8パニック、ZIP爆弾、メモリ枯渇、ゼロ次元生成 |
| **High** | 12 | SSRF、APIキー漏洩、パス走査、認証情報平文保持 |
| **Medium** | 18 | 型安全性、ロジック不整合、並行性、コード重複 |
| **Low** | 15 | パフォーマンス、API設計、人間工学的改善 |

**最も危険な問題**: `truncate_text` のUTF-8境界パニック (`client/retry.rs:125`) — NovelAI APIが非ASCII文字を含むエラーを返した場合に**確実にクラッシュ**する。

---

## 2. Critical (即時対応必須)

### 2.1 `truncate_text` UTF-8境界でのパニック

**ファイル**: `client/retry.rs:123-128`

```rust
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() > max_len {
        format!("{}...[truncated]", &text[..max_len])  // ← パニック！
    } else {
        text.to_string()
    }
}
```

**問題**: `&text[..max_len]` はバイトオフセット200でスライスする。マルチバイトUTF-8文字 (日本語、中国語、絵文字) の途中でスライスすると `byte index is not a char boundary` パニックが発生する。

**修正案**:
```rust
fn truncate_text(text: &str, max_len: usize) -> String {
    match text.char_indices().nth(max_len) {
        Some((idx, _)) => format!("{}...[truncated]", &text[..idx]),
        None => text.to_string(),
    }
}
```

---

### 2.2 画像デコンプレッション爆弾 (Decompression Bomb)

**ファイル**: `utils/charref.rs:37`, `utils/image.rs:105`, `utils/mask.rs:45`

```rust
let img = image::load_from_memory(&buffer).map_err(|_| { ... })?;
```

**問題**: 100バイトのPNGファイルが65535x65535ピクセルを宣言可能 → デコード時に **~16GBのメモリ**を消費。`validate_image_data_size()` は圧縮後サイズのみチェックし、展開後の次元は検証しない。

**修正案**:
```rust
fn load_image_safe(buffer: &[u8], max_pixels: u64) -> Result<DynamicImage> {
    let reader = image::io::Reader::new(Cursor::new(buffer))
        .with_guessed_format()
        .map_err(|e| NovelAIError::Image(format!("Format detection failed: {}", e)))?;
    let (w, h) = reader.into_dimensions()
        .map_err(|e| NovelAIError::Image(format!("Cannot read dimensions: {}", e)))?;
    if (w as u64) * (h as u64) > max_pixels {
        return Err(NovelAIError::Image("Image dimensions too large".into()));
    }
    image::load_from_memory(buffer).map_err(|e| NovelAIError::Image(e.to_string()))
}
```

---

### 2.3 Base64デコード時のメモリ枯渇

**ファイル**: `utils/image.rs:43-58`

**問題**: `decode_base64_image` は任意サイズのbase64文字列をデコードする。1GBのbase64 → 約750MBのデコードデータ。`get_image_buffer` / `get_image_base64` はサイズチェックを呼ばない。

**修正案**: `decode_base64_image` の先頭でbase64文字列長を検証:
```rust
const MAX_BASE64_INPUT_LEN: usize = 14 * 1024 * 1024; // ~10MB decoded
if base64_str.len() > MAX_BASE64_INPUT_LEN {
    return Err(NovelAIError::ImageFileSize { ... });
}
```

---

### 2.4 Inpaintマスクのグリッドスナップでゼロ次元発生

**ファイル**: `anlas.rs:297-298`

```rust
let new_w = ((mask_width as f64 * scale).floor() / grid).floor() * grid;
let new_h = ((mask_height as f64 * scale).floor() / grid).floor() * grid;
```

**問題**: 極端なアスペクト比 (例: `mask_width=1, mask_height=10000`) の場合:
- `new_w = floor(floor(1 * 10.24) / 64) * 64 = floor(10/64) * 64 = 0`
- **幅ゼロ**が返却 → `calc_v4_base_cost` でコスト0 → 課金されない不正な生成

**修正案**:
```rust
let new_w = ((mask_width as f64 * scale).floor() / grid).floor() * grid;
let new_w = new_w.max(grid); // 最低1グリッド (64px) を保証
```

---

### 2.5 ZIP `size()` ヘッダの信頼 (ZIP爆弾)

**ファイル**: `client/response.rs:67-88`

```rust
let uncompressed_size = file.size(); // ← ZIPヘッダから取得 (偽装可能)
if uncompressed_size > constants::MAX_DECOMPRESSED_IMAGE_SIZE as u64 { ... }
// ...
file.read_to_end(&mut image_data)?; // ← 実際のサイズは無制限
```

**問題**: `file.size()` はZIPローカルファイルヘッダの値であり偽装可能。宣言1KBだが実際は10GBというペイロードを受け入れてしまう。

**修正案**:
```rust
let mut limited_reader = file.take(MAX_DECOMPRESSED_IMAGE_SIZE as u64 + 1);
let mut image_data = Vec::new();
limited_reader.read_to_end(&mut image_data)?;
if image_data.len() > MAX_DECOMPRESSED_IMAGE_SIZE {
    return Err(NovelAIError::Parse("Decompressed size exceeds limit".into()));
}
```

---

## 3. High (早期対応推奨)

### 3.1 環境変数URL注入によるSSRF

**ファイル**: `constants.rs:7-35`

```rust
pub fn api_url() -> String {
    std::env::var("NOVELAI_API_URL")
        .unwrap_or_else(|_| "https://image.novelai.net/ai/generate-image".to_string())
}
```

**問題**: 環境変数が `http://169.254.169.254/latest/meta-data/` に設定された場合、全APIトラフィックがリダイレクトされる。`Authorization: Bearer {api_key}` ヘッダが攻撃者サーバに送信される。

**修正案**:
```rust
pub fn api_url() -> String {
    let url = std::env::var("NOVELAI_API_URL")
        .unwrap_or_else(|_| DEFAULT_API_URL.to_string());
    let parsed = url::Url::parse(&url).expect("Invalid API URL");
    assert!(parsed.scheme() == "https", "Only HTTPS URLs are allowed");
    assert!(
        parsed.host_str().map_or(false, |h| h.ends_with("novelai.net")),
        "URL must be a novelai.net domain"
    );
    url
}
```

---

### 3.2 APIキーの平文メモリ保持

**ファイル**: `client/mod.rs:54`

```rust
pub struct NovelAIClient {
    api_key: String,  // ← ドロップ時にゼロ化されない
}
```

**問題**: プロセスのコアダンプやメモリインスペクションで認証情報が露出。`format!("Bearer {}", api_key)` でさらにコピーが生成される。

**修正案**: `secrecy::SecretString` を使用し、`Drop` 時にゼロ化。

---

### 3.3 パス走査チェックのバイパス

**ファイル**: `schemas/validation.rs:11-19`, `utils/image.rs:31-39`, `utils/vibe.rs:22-31`

```rust
let normalized = path.replace('\\', "/");
if normalized.contains("..") { ... }
```

**問題**: 3箇所にコピペ重複。かつ以下をブロックできない:
- 絶対パス (`/etc/passwd`) — 制限なし
- シンボリックリンク経由の走査
- `..abc` のような安全な文字列の誤検出 (false positive)
- パスセグメント単位でなく部分文字列マッチ

**修正案**: パスセグメント単位のチェックに変更し、共通関数に統合:
```rust
pub(crate) fn validate_safe_path(path: &str) -> Result<()> {
    let path = std::path::Path::new(path);
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(NovelAIError::Validation("Path traversal detected".into()));
        }
    }
    Ok(())
}
```

---

### 3.4 Tokenizerキャッシュの Thundering Herd

**ファイル**: `tokenizer/cache.rs:228-249`

```rust
// 読み取りロック → キャッシュミス → ロック解放 → ネットワークフェッチ → 書き込みロック
let guard = CLIP_TOKENIZER.read().unwrap();
if let Some(cached) = guard.as_ref() { return Ok(cached.clone()); }
drop(guard);
// ↓ この間に複数スレッドが同時にフェッチを開始
let data_str = fetch_data(CLIP_TOKENIZER_URL, force_refresh).await?;
```

**問題**: 読み取りロック解放〜書き込みロック取得の間にN個のスレッドが同時にHTTPフェッチ+パース+構築を実行。

**修正案**: `tokio::sync::OnceCell` を使用:
```rust
static CLIP_TOKENIZER: tokio::sync::OnceCell<Arc<NovelAIClipTokenizer>> = tokio::sync::OnceCell::const_new();
```

---

### 3.5 未呼出のパス走査防御関数

**ファイル**: `tokenizer/cache.rs:87-104`

```rust
#[allow(dead_code)]
fn validate_cache_path(cache_path: &Path) -> Result<()> { ... }
```

**問題**: パス走査防御のために実装されたが **`#[allow(dead_code)]` で放置され、一度も呼ばれていない**。`read_from_cache` / `write_to_cache` はバリデーションなしでパスを使用。

---

### 3.6 HTTPステータスコード未検証 (Tokenizer)

**ファイル**: `tokenizer/cache.rs:154-170`

**問題**: `response.status().is_success()` のチェックがない。404/500のレスポンスボディがそのままデコンプレッサに送られ、「デコンプレッション失敗」という誤ったエラーメッセージになる。

---

### 3.7 レスポンス全体ダウンロード後のサイズチェック

**ファイル**: `tokenizer/cache.rs:172-181`, `client/response.rs:20-22`

```rust
let bytes = response.bytes().await?;  // ← 全データをメモリに読み込み
if bytes.len() > MAX_RESPONSE_SIZE { ... }  // ← 手遅れ
```

**修正案**: `Content-Length` ヘッダを先にチェック:
```rust
if let Some(len) = response.content_length() {
    if len as usize > MAX_RESPONSE_SIZE { return Err(...); }
}
```

---

### 3.8 信頼できないJSONに対する `.unwrap()` (パニック)

**ファイル**: `tokenizer/cache.rs:292-297`

```rust
.map(|entry| {
    let arr = entry.as_array().unwrap();       // ← パニック
    let piece = arr[0].as_str().unwrap();      // ← パニック
    let score = arr[1].as_f64().unwrap();      // ← パニック
    (piece, score)
})
```

**問題**: 外部から取得したJSONデータに対して3回の `unwrap()`。不正な形式でアプリケーション全体がクラッシュ。

---

### 3.9 MessagePackパースの無制限ネスト

**ファイル**: `client/response.rs:155-182`

**問題**: `rmpv::decode::read_value` は任意深度のネスト構造をパース。悪意あるペイロードでスタックオーバーフローやメモリ枯渇が発生。200MBバッファ全体がパース対象。

---

### 3.10 デコンプレッション出力サイズ無制限

**ファイル**: `tokenizer/cache.rs:196-221`

```rust
let mut result = Vec::new();
decoder.read_to_end(&mut result)?;  // ← 出力サイズ無制限
```

**問題**: 小さな圧縮ペイロードが数GBに展開される "zip bomb" に対する防御がない。

---

### 3.11 Vibeファイル読み込みサイズ無制限

**ファイル**: `utils/vibe.rs:40-42`

```rust
let content = std::fs::read_to_string(&safe_path)?;  // ← サイズ無制限
```

**問題**: `/dev/urandom` へのパスや巨大ファイルでメモリ枯渇。

---

### 3.12 `i64` 次元値の `as u64` キャスト

**ファイル**: `anlas.rs:139-140, 344-347`

```rust
pub struct InpaintCorrectionResult {
    pub width: i64,   // ← なぜ i64?
    pub height: i64,
}
// ...
effective_width = correction.width as u64;  // ← 負値が巨大なu64に
```

**問題**: `i64` の負値が `as u64` で静かに巨大値にラップ → 誤ったコスト計算。

---

## 4. Medium (計画的対応)

### 4.1 3つの並列Vibeベクタは不正状態を表現可能

**ファイル**: `schemas/types.rs:148-150`

```rust
pub vibes: Option<Vec<VibeItem>>,
pub vibe_strengths: Option<Vec<f64>>,
pub vibe_info_extracted: Option<Vec<f64>>,
```

**問題**: 長さ不一致、片方だけ `Some` などの不正状態がコンパイル時に防げない。50行以上のバリデーションコードが必要。

**修正案**:
```rust
pub struct VibeConfig {
    pub item: VibeItem,
    pub strength: f64,
    pub info_extracted: f64,
}
pub vibes: Option<Vec<VibeConfig>>,
```

---

### 4.2 `GenerateAction` のOption地獄

**ファイル**: `schemas/types.rs:136-165`

```rust
pub action: GenerateAction,
pub source_image: Option<ImageInput>,  // Img2Img/Infillで必須
pub mask: Option<ImageInput>,          // Infillでのみ必須
pub mask_strength: Option<f64>,        // Infillでのみ必須
pub img2img_strength: Option<f64>,     // Img2Imgでのみ使用
pub img2img_noise: Option<f64>,        // Img2Imgでのみ使用
```

**問題**: 「Generateなのに `source_image` がSome」「Infillなのに `mask` がNone」が型レベルで防げない。

**修正案** (不正状態を表現不能にする):
```rust
pub enum GenerateAction {
    Generate,
    Img2Img { source_image: ImageInput, strength: f64, noise: f64 },
    Infill { source_image: ImageInput, mask: ImageInput, mask_strength: f64, ... },
}
```

---

### 4.3 save_path / save_dir / save_filename の相互排他制約

**ファイル**: `schemas/types.rs:224-227`

**問題**: 3つの `Option<String>` の組み合わせ制約がランタイムバリデーションに依存。

**修正案**:
```rust
pub enum SaveTarget {
    None,
    ExactPath(PathBuf),
    Directory { dir: PathBuf, filename: Option<String> },
}
```

---

### 4.4 `Validation` vs `Range` エラー区分の曖昧さ

**ファイル**: `error.rs:7-10`

**問題**: `NovelAIError::Validation(String)` と `NovelAIError::Range(String)` の使い分けが一貫していない。呼び出し側は両方にマッチする必要がある。

**修正案**: 統合するか、構造化されたバリデーションエラー型を導入:
```rust
enum ValidationKind {
    InvalidSampler { got: String },
    DimensionNotMultipleOf64 { value: u32 },
    OutOfRange { field: &'static str, min: f64, max: f64, got: f64 },
    // ...
}
```

---

### 4.5 Opus無料判定と課金計算の非対称性

**ファイル**: `anlas.rs:374-382`

```rust
// 無料判定: オリジナル次元を使用
let is_opus_free = is_opus_free_generation(params.width, params.height, ...);
// コスト計算: 補正後の次元を使用
let base_cost = calc_v4_base_cost(effective_width, effective_height, ...);
```

**問題**: Inpaintでマスク補正により次元が拡大された場合、コストは拡大後で計算されるが無料判定はオリジナルで行われる → 本来有料のはずの生成が無料になる可能性。

---

### 4.6 エラー結果が `Ok(...)` で返る

**ファイル**: `anlas.rs:410-431`

```rust
let total_cost = if error { 0 } else { ... };
Ok(GenerationCostResult { error: true, error_code: Some(-3), ... })
```

**問題**: エラー状態なのに `Ok(...)` で返却。中間値 (`base_cost`, `per_image_cost`) は正常値が入ったまま。Rustの `Result` 型の意図に反する設計。

---

### 4.7 Vibe強度のバリデーション漏れ

**ファイル**: `schemas/validation.rs:372-423`

**問題**: `vibe_strengths` / `vibe_info_extracted` のベクタ長は検証するが、個々の値の範囲 (`0.0..=1.0`) を検証しない。`vec![999.0]` が通過する。対照的に `EncodeVibeParams::validate` (line 513-518) では範囲検証している。

---

### 4.8 `VibeItem::FilePath` のパス走査未検証

**ファイル**: `schemas/validation.rs:481-486`

```rust
VibeItem::FilePath(path) => {
    if path.is_empty() { ... }
    // ← validate_safe_path() が呼ばれていない!
}
```

**問題**: `save_path` / `save_dir` にはパス走査チェックがあるが、Vibeファイルパスにはない。

---

### 4.9 enum/文字列スライス/serde rename の三重管理

**ファイル**: `constants.rs:65-226`

| 管理箇所 | 例 |
|----------|-----|
| `Sampler` enum variants | `KDpmpp2sAncestral` |
| `#[serde(rename = "...")]` | `"k_dpmpp_2s_ancestral"` |
| `as_str()` メソッド | `"k_dpmpp_2s_ancestral"` |
| `VALID_SAMPLERS: &[&str]` | `"k_dpmpp_2s_ancestral"` |

新バリアント追加時に4箇所の同期が必要。`VALID_SAMPLERS` 等は使用箇所なし (dead code の疑い)。

**修正案**: `strum` クレートの `#[derive(AsRefStr, EnumString)]` で統一。

---

### 4.10 `std::sync::RwLock` の非同期コンテキストでの使用

**ファイル**: `tokenizer/cache.rs:26-27`

**問題**: `std::sync::RwLock` はロック取得中にスレッドをブロック。tokioランタイムでは他タスクを飢餓状態にする。`tokio::sync::RwLock` を使うべき。

---

### 4.11 Mutex/RwLock ポイズニングによるパニック連鎖

**ファイル**: `tokenizer/clip.rs:158`, `tokenizer/cache.rs:230,249,308,309`

```rust
self.cache.lock().unwrap()         // いずれかのスレッドがパニックすると全スレッドパニック
CLIP_TOKENIZER.read().unwrap()
```

**修正案**: `.unwrap_or_else(|e| e.into_inner())` でポイズニングから回復。

---

### 4.12 `generate()` が130行の巨大関数

**ファイル**: `client/mod.rs:238-366`

**問題**: パラメータデフォルト化、リファレンス処理、Vibe処理、ペイロード構築、HTTP通信、レスポースパース、ファイル保存まで1関数に集中。

---

### 4.13 毎操作2回の余分なHTTPリクエスト (残高チェック)

**ファイル**: `client/mod.rs:315, 350-351`

```rust
let anlas_before = self.try_get_balance().await;   // ← 追加リクエスト1
// ... API呼び出し ...
let (anlas_remaining, anlas_consumed) = self.get_anlas_after(anlas_before).await; // ← 追加リクエスト2
```

**問題**: 毎回3 HTTPリクエスト (本来は1で十分)。レイテンシが3倍。

---

### 4.14 `body_str.clone()` がリトライごとに実行

**ファイル**: `client/retry.rs:33`

**問題**: img2imgの大きなBase64ペイロード (数十MB) がリトライごとにクローンされる。`Arc<String>` または `Bytes` を使用すべき。

---

### 4.15 `NaN` スコアがViterbi DPテーブルを汚染

**ファイル**: `tokenizer/t5.rs:29-40`

```rust
let mut min_score: f64 = 0.0;  // ← f64::INFINITY であるべき
```

**問題**: `NaN` スコアがボキャブラリに存在すると `candidate = best[i-l].0 + NaN = NaN` が伝播し、全DPテーブルが壊れる。`min_score` の初期値 `0.0` も全スコアが正の場合に誤動作。

---

### 4.16 サーバーエラーのレスポンスボディがログに出力

**ファイル**: `client/retry.rs:82-86`

**問題**: APIサーバーが `Authorization` ヘッダのエコーやその他の機密データをエラーレスポンスに含む可能性。200文字のレスポンスがstderrにログ出力される。

---

### 4.17 HTTP 502/503がリトライされない

**ファイル**: `client/retry.rs:69-94`

**問題**: 429のみリトライ。502 (Bad Gateway) / 503 (Service Unavailable) はロードバランサの一時的エラーだがハードエラーとして返される。

---

### 4.18 並行実行時の残高追跡競合

**ファイル**: `client/mod.rs:315, 350-351`

**問題**: 2つの `generate()` が並行実行すると、両方が同じ "before" 残高を読み取り、"after" との差分が不正になる。

---

## 5. Low (改善推奨)

### 5.1 URL関数が毎呼び出しでアロケーション

**ファイル**: `constants.rs:7-35`

```rust
pub fn api_url() -> String {  // ← 毎回 String を生成
    std::env::var("NOVELAI_API_URL").unwrap_or_else(|_| "...".to_string())
}
```

**修正案**: `OnceLock<String>` でキャッシュ。

### 5.2 `MAX_SEED: u32 = 4_294_967_295` は `u32::MAX`

**ファイル**: `constants.rs:259`

マジックナンバー。`u32::MAX` を使い、seed フィールド自体も `u32` にするか `MAX_SEED` を `u64` に。

### 5.3 `get_image_dimensions` がフルデコード

**ファイル**: `utils/image.rs:97-123`

**問題**: 次元取得のためだけに画像全体をデコード (1536x1024 RGBA = ~6MB)。`Reader::into_dimensions()` でヘッダのみ読み取れば高速。

### 5.4 `sanitize_file_path` が3箇所に重複

**ファイル**: `utils/image.rs:31-39`, `utils/vibe.rs:22-31`, `schemas/validation.rs:11-19`

同一コードの完全コピペ。`pub(crate)` の共通関数に統合すべき。

### 5.5 PNG エンコードパターンが3箇所に重複

**ファイル**: `utils/mask.rs:52-58`, `utils/mask.rs:174-181`, `utils/charref.rs:79-85`

```rust
let dynamic = DynamicImage::ImageLuma8(img.clone());
let mut buf = Cursor::new(Vec::new());
dynamic.write_to(&mut buf, ImageFormat::Png)?;
Ok(buf.into_inner())
```

共通ヘルパー `fn encode_to_png(img: &DynamicImage) -> Result<Vec<u8>>` に統合。

### 5.6 `model_key_from_str` が `Model::model_key` と重複

**ファイル**: `constants.rs:156-164` vs `145-152`

`FromStr` を `Model` に実装して `Model::from_str(s)?.model_key()` で統一。

### 5.7 enum に `FromStr` / `Hash` 未実装

**ファイル**: `constants.rs:91-216`

`Sampler`, `Model`, `NoiseSchedule`, `AugmentReqType` いずれも `FromStr` / `Hash` がない。

### 5.8 `0.0..=1.0` 範囲チェックが11回重複

**ファイル**: `schemas/validation.rs` (lines 56, 60, 77, 82, 119, 124, 269, 290, 295, 513, 518)

```rust
if !(0.0..=1.0).contains(&value) { ... }
```

ヘルパー `validate_unit_range(value: f64, field: &str) -> Result<()>` に統合。

### 5.9 BPEセパレータが脆弱なセンチネル文字列

**ファイル**: `tokenizer/clip.rs:12`

```rust
const BPE_SEPARATOR: &str = "\u{b7}\u{1F60E}\u{b7}";  // ·😎·
```

BPEマージルールにこの文字列が含まれると衝突。タプルキー `(String, String)` に変更すべき。

### 5.10 BPE内部ループの過剰なアロケーション

**ファイル**: `tokenizer/clip.rs:186-246`

```rust
format!("{}{}{}", pair.0, BPE_SEPARATOR, pair.1)  // ← 全ペアで毎回format!
```

### 5.11 Viterbi内部ループの `String` アロケーション

**ファイル**: `tokenizer/t5.rs:102`

```rust
let substr: String = chars[i - l..i].iter().collect();  // ← O(n * max_piece_len) 回
```

バイトオフセットを事前計算し `&str` スライスを使用すべき。

### 5.12 `count_tokens()` がフルVecを構築して `.len()` のみ使用

**ファイル**: `tokenizer/t5.rs:170-172`

### 5.13 HTTPクライアントがリクエストごとに再作成

**ファイル**: `tokenizer/cache.rs:149-152`

コネクションプールが活用されない。`OnceLock<reqwest::Client>` でキャッシュ。

### 5.14 `User-Agent` ヘッダ未設定

**ファイル**: `client/retry.rs:24-27`

一部のAPIゲートウェイは `User-Agent` なしのリクエストをレート制限/拒否する。

### 5.15 `ImageInput::FilePath` が `String` (not `PathBuf`)

**ファイル**: `schemas/types.rs:55`

ファイルパスに `String` を使用するのはRustのアンチパターン。`PathBuf` はOS固有のパスエンコーディングを正しく扱う。

---

## 6. モジュール別詳細分析

### 6.1 `error.rs` — エラー型設計

| 問題 | 深刻度 | 概要 |
|------|--------|------|
| 文字列型エラーバリアント | Medium | 6/11バリアントが `String` を持ち、プログラマティックなマッチング不可 |
| `reqwest::Error` の `#[from]` 未実装 | Medium | 全箇所で手動 `.map_err(|e| Other(format!(...)))` |
| `Api` の `status_code` が `u16` | Low | `reqwest::StatusCode` の方が安全 |
| `Timeout` / `RateLimit` / `Authentication` バリアント不在 | Medium | 429、401/403、タイムアウトの区別が不可能 |
| `TokenValidation` にフィールド名なし | Low | どのプロンプト (正/負) がトークン超過かわからない |
| `ImageFileSize` の `file_size_mb` が `f64` | Low | `u64` (バイト単位) の方が精密 |

### 6.2 `constants.rs` — 定数・設定

| 問題 | 深刻度 | 概要 |
|------|--------|------|
| SSRF可能なURL関数 | High | 環境変数の検証なし |
| enum/slice/serde の三重管理 | Medium | 4つのenum全てで同期リスク |
| `VALID_SAMPLERS` 等がdead code疑い | Low | enum型を使用しているため不要の可能性 |
| `UPSCALE_COST_TABLE` のソート順保証なし | Low | コンパイル時/実行時のアサーション欠如 |
| `CHARREF_*_THRESHOLD` の境界曖昧 | Low | 0.8と1.25の間の処理が暗黙的 |

### 6.3 `anlas.rs` — コスト計算

| 問題 | 深刻度 | 概要 |
|------|--------|------|
| グリッドスナップでゼロ次元 | Critical | 極端なアスペクト比で発生 |
| `i64` 次元 → `as u64` ラップ | High | 負値で巨大値に |
| `SubscriptionTier` が `u32` エイリアス | Medium | 型安全性ゼロ、不正値受入 |
| Opus無料判定の非対称性 | Medium | 補正後次元でコスト計算、オリジナルで無料判定 |
| Inpaintモードでvibeコスト黙殺 | Medium | エラーも警告もなし |
| マジックエラーコード `-3` 再利用 | Medium | 2つの異なるエラー状態に同一コード |
| `f64::INFINITY.ceil() as u64` | Medium | 極端入力でUB/飽和 |
| `expand_to_min_pixels` にゼロ除算 | Medium | `width=0` or `height=0` で `Inf` |
| `steps` / `width` / `height` 上限チェックなし | Medium | `MAX_STEPS` 等が検証されない |

### 6.4 `schemas/` — 型・バリデーション・ビルダー

| 問題 | 深刻度 | 概要 |
|------|--------|------|
| 並列vibeベクタ | Medium | 不正状態が表現可能 |
| Option地獄のGenerateAction | Medium | enum化で型安全に |
| save_path制約の表現不能 | Medium | enum `SaveTarget` で解決 |
| `EncodeVibeParams::default()` が常にバリデーション失敗 | Low | 空バイト列画像 |
| `pub` フィールドでバリデーション任意 | Medium | `pub(crate)` にして builder必須化 |
| `validate_safe_path` バイパス可能 | High | セグメント単位チェックに変更 |
| `build_async()` 未提供 | Low | トークンバリデーションがbuilder経由で不可 |
| glob再エクスポート (`pub use types::*`) | Low | APIサーフェスの暗黙拡大 |
| `Serialize`/`Deserialize` の不一致 | Low | 一部の結果型でserde未実装 |

### 6.5 `utils/` — ユーティリティ

| 問題 | 深刻度 | 概要 |
|------|--------|------|
| デコンプレッション爆弾 | Critical | 全 `load_from_memory` 箇所 |
| Base64メモリ枯渇 | Critical | サイズチェックなしのデコード |
| `sanitize_file_path` 3箇所重複 | Low | 共通関数に統合 |
| PNG エンコード3箇所重複 | Low | ヘルパー関数に統合 |
| `get_image_dimensions` フルデコード | Low | ヘッダのみ読み取りで十分 |
| `looks_like_file_path` 偽陽性 | Low | base64がパスとして誤判定 |
| `extract_encoding` の黙示的フォールバック | Medium | 不正モデルで別モデルのエンコーディング使用 |
| mask次元 < 8 で0x0マスク | Medium | `width/8 = 0` が黙殺される |
| `BASE64_ONLY_REGEX` と手動チェックの不一致 | Medium | URL-safe base64の扱いが矛盾 |

### 6.6 `tokenizer/` — トークナイザ

| 問題 | 深刻度 | 概要 |
|------|--------|------|
| Thundering herd | High | 複数スレッドが同時にフェッチ |
| `unwrap()` on untrusted JSON | High | パニッククラッシュ |
| `validate_cache_path` が dead code | High | 実装はあるが未使用 |
| HTTPステータス未検証 | High | 404/500がデコンプレッサに到達 |
| デコンプレッション無制限 | High | zip bomb脆弱性 |
| NaNスコアがDPテーブル汚染 | Medium | `min_score` 初期値も誤り |
| `std::sync::RwLock` in async | Medium | tokioワーカースレッドをブロック |
| 不要な `String` アロケーション (BPE, Viterbi) | Low | パフォーマンスへの影響 |
| weight正規表現が番号プレフィクスなしで発火 | Medium | `::text::` が誤って削除 |
| TOCTOU競合 (キャッシュファイル) | Low | メタデータチェックと読み取りの間に変更 |

### 6.7 `client/` — HTTPクライアント

| 問題 | 深刻度 | 概要 |
|------|--------|------|
| `truncate_text` パニック | Critical | マルチバイトUTF-8 |
| ZIP size() 信頼 | Critical | ZIP爆弾 |
| APIキー平文保持 | High | `secrecy::SecretString` 推奨 |
| パス走査チェック不完全 | High | `Path::new()` は正規化しない |
| MsgPack無制限パース | High | スタックオーバーフロー |
| 502/503リトライなし | Medium | 一時的ゲートウェイエラー |
| `generate()` 130行 | Medium | 分割すべき |
| JSON手動構築 (string key) | Medium | 型付き構造体で安全に |
| 残高チェック競合 | Medium | 並行実行時の不整合 |
| `extra_seed` 計算2箇所重複 | Low | ヘルパー関数に統合 |
| `User-Agent` 未設定 | Low | 一部ゲートウェイで拒否される |
| `utc_now()` 自前実装 | Low | `time` クレートで代替 |

---

## 7. アーキテクチャ全体の改善提案

### 7.1 型安全性の強化 (Impact: 高)

```
現状: Option<T> によるランタイムバリデーション (約150行)
改善: enum/newtype で不正状態をコンパイル時に排除 (バリデーションコード ~100行削減)
```

**具体的なアクション**:
1. `GenerateAction` をデータ付きenumに変更
2. `VibeConfig` 構造体でvibeパラメータを統合
3. `SaveTarget` enumでsave_path制約を型で表現
4. `SubscriptionTier` をenumに変更
5. `ImageInput::FilePath(String)` → `ImageInput::FilePath(PathBuf)`

### 7.2 セキュリティ層の統合 (Impact: 高)

```
現状: セキュリティチェックが分散・重複・不完全
改善: 共通のセキュリティミドルウェア層
```

**具体的なアクション**:
1. 共通の `validate_safe_path()` を1箇所に統合 (3箇所の重複解消)
2. `load_image_safe()` — サイズ制限 + 次元制限付き画像ロード関数
3. 環境変数URLのバリデーション (scheme, domain allowlist)
4. レスポンスの段階的サイズチェック (Content-Length → ストリーミング)

### 7.3 エラー型の再設計 (Impact: 中)

```
現状: 11バリアント中6つが String、 #[from] 未実装
改善: 構造化されたエラー型 + 自動変換
```

**具体的なアクション**:
1. `#[from] reqwest::Error` を追加
2. `Validation(String)` → `Validation(ValidationKind)` (enum)
3. `Timeout`, `RateLimit`, `Authentication` バリアントを追加
4. `Validation` と `Range` を統合

### 7.4 並行性の改善 (Impact: 中)

**具体的なアクション**:
1. `tokio::sync::OnceCell` でtokenizerシングルトンを初期化
2. `std::sync::RwLock` → `tokio::sync::RwLock` (asyncコンテキスト)
3. `.unwrap()` → ポイズニング回復
4. 残高チェックを `tokio::join!` でAPI呼び出しと並列化

### 7.5 コード重複の削減 (Impact: 低)

| 重複パターン | 出現数 | 削減行数 |
|------------|--------|----------|
| `sanitize_file_path` | 3箇所 | ~20行 |
| PNG エンコード | 3箇所 | ~15行 |
| `0.0..=1.0` チェック | 11箇所 | ~30行 |
| save_option バリデーション | 4箇所 | ~30行 |
| `as_str()` メソッド | 6 enum | `strum` で自動化 |
| `extra_seed` 計算 | 2箇所 | ~5行 |
| 合計 | | **~100行削減** |

---

> **注意**: 本レビューはコードの静的分析に基づいています。実行時の動作検証やペネトレーションテストは行っていません。セキュリティ指摘事項については、脅威モデルとデプロイメント環境を考慮した上で優先度を判断してください。
