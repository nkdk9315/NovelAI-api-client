# API リファレンス

## NovelAIClient

メインのクライアント構造体。全APIリクエストのエントリーポイント。

### コンストラクタ

```rust
pub fn new(logger: Option<Box<dyn Logger>>) -> Result<Self>
```

- `logger`: カスタムロガー。`None` で `DefaultLogger` (stderr出力) を使用
- 環境変数 `NOVELAI_API_KEY` からAPIキーを取得
- APIキーは `secrecy::SecretString` で保護

### メソッド

#### generate

```rust
pub async fn generate(&self, params: &GenerateParams) -> Result<GenerateResult>
```

画像を生成する。txt2img / img2img / inpaint を `GenerateAction` で切り替え。

- バリデーション (非同期トークンチェック含む)
- Vibe エンコード (未エンコードの場合)
- Character Reference 画像前処理
- ペイロード構築 → API リクエスト
- レスポンスパース → ファイル保存 (SaveTarget指定時)

#### encode_vibe

```rust
pub async fn encode_vibe(&self, params: &EncodeVibeParams) -> Result<VibeEncodeResult>
```

画像から Vibe エンコーディングを生成。`.naiv4vibe` JSON ファイルとして保存可能。

#### augment_image

```rust
pub async fn augment_image(&self, params: &AugmentParams) -> Result<AugmentResult>
```

画像加工ツール (colorize, emotion, sketch, lineart, declutter, bg-removal)。

#### upscale_image

```rust
pub async fn upscale_image(&self, params: &UpscaleParams) -> Result<UpscaleResult>
```

画像のアップスケール (2x or 4x)。

#### get_anlas_balance

```rust
pub async fn get_anlas_balance(&self) -> Result<AnlasBalanceResponse>
```

Anlas残高とサブスクリプション情報を取得。

---

## Logger trait

```rust
pub trait Logger: Send + Sync {
    fn info(&self, message: &str);
    fn warn(&self, message: &str);
    fn error(&self, message: &str);
}
```

カスタムログ出力用トレイト。`DefaultLogger` は stderr に出力。

---

## GenerateParams

画像生成の全パラメータを保持する構造体。

### フィールド

| フィールド | 型 | デフォルト | 説明 |
|-----------|-----|----------|------|
| `prompt` | `String` | `""` | ポジティブプロンプト |
| `negative_prompt` | `Option<String>` | `None` (定数使用) | ネガティブプロンプト |
| `action` | `GenerateAction` | `Generate` | 生成モード |
| `model` | `Model` | `NaiDiffusion45Full` | 使用モデル |
| `width` | `u32` | `832` | 出力幅 (64の倍数, 64-2048) |
| `height` | `u32` | `1216` | 出力高さ (64の倍数, 64-2048) |
| `steps` | `u32` | `23` | 生成ステップ数 (1-50) |
| `scale` | `f64` | `5.0` | CFGスケール (0.0-10.0) |
| `cfg_rescale` | `f64` | `0.0` | CFG Rescale (0.0-1.0) |
| `sampler` | `Sampler` | `KEulerAncestral` | サンプラー |
| `noise_schedule` | `NoiseSchedule` | `Karras` | ノイズスケジュール |
| `seed` | `Option<u64>` | `None` (ランダム) | シード値 (0-4294967295) |
| `characters` | `Option<Vec<CharacterConfig>>` | `None` | キャラクター配置 (最大6) |
| `vibes` | `Option<Vec<VibeConfig>>` | `None` | Vibe Transfer (最大10) |
| `character_reference` | `Option<Vec<CharacterReferenceConfig>>` | `None` | キャラクター参照 |
| `save` | `SaveTarget` | `None` | 保存先 |

### Builder

```rust
let params = GenerateParams::builder()
    .prompt("1girl")
    .width(832)
    .height(1216)
    .steps(23)
    .scale(5.0)
    .cfg_rescale(0.0)
    .sampler(Sampler::KEulerAncestral)
    .noise_schedule(NoiseSchedule::Karras)
    .seed(12345)
    .negative_prompt("bad quality")
    .characters(vec![...])
    .vibes(vec![...])
    .character_reference(vec![...])
    .save_dir("./output")       // SaveTarget::Directory
    // .save_path("./out.png")  // SaveTarget::ExactPath (排他)
    .build()?;
```

### バリデーション

```rust
params.validate()?;        // 同期バリデーション (寸法, 範囲, 制約)
params.validate_async().await?;  // + トークン数チェック
```

---

## GenerateAction

```rust
pub enum GenerateAction {
    Generate,
    Img2Img {
        source_image: ImageInput,
        strength: f64,    // 0.0-1.0, デフォルト 0.62
        noise: f64,       // 0.0-1.0, デフォルト 0.0
    },
    Infill {
        source_image: ImageInput,
        mask: ImageInput,
        mask_strength: f64,       // 0.01-1.0
        color_correct: bool,      // デフォルト true
        hybrid_strength: Option<f64>,  // 0.01-0.99
        hybrid_noise: Option<f64>,     // 0.0-0.99
    },
}
```

---

## ImageInput

```rust
pub enum ImageInput {
    FilePath(PathBuf),
    Base64(String),
    DataUrl(String),
    Bytes(Vec<u8>),
}
```

---

## SaveTarget

```rust
pub enum SaveTarget {
    None,
    ExactPath(String),
    Directory { dir: String, filename: Option<String> },
}
```

- `None`: 保存しない (image_data のみ返却)
- `ExactPath`: 指定パスに保存
- `Directory`: ディレクトリ + 自動ファイル名 (timestamp + seed)

---

## VibeItem / VibeConfig

```rust
pub enum VibeItem {
    FilePath(PathBuf),              // .naiv4vibe ファイル
    Encoded(VibeEncodeResult),      // エンコード済み結果
    RawEncoding(String),            // 生のBase64エンコーディング
}

pub struct VibeConfig {
    pub item: VibeItem,
    pub strength: f64,         // 0.0-1.0
    pub info_extracted: f64,   // 0.0-1.0
}
```

---

## CharacterConfig

```rust
pub struct CharacterConfig {
    pub prompt: String,            // キャラクタープロンプト
    pub center_x: f64,            // 0.0-1.0 (画面上の横位置)
    pub center_y: f64,            // 0.0-1.0 (画面上の縦位置)
    pub negative_prompt: String,   // キャラクターネガティブ
}
```

最大6キャラクター。

---

## CharacterReferenceConfig

```rust
pub struct CharacterReferenceConfig {
    pub image: ImageInput,
    pub mode: CharRefMode,
    pub strength: f64,   // 0.0-1.0
    pub fidelity: f64,   // 0.0-1.0
}

pub enum CharRefMode {
    Character,
    CharacterAndStyle,
    Style,
}
```

**制約**: `vibes` と `character_reference` は同時に使用不可。

---

## EncodeVibeParams

```rust
pub struct EncodeVibeParams {
    pub image: ImageInput,
    pub information_extracted: f64,  // 0.0-1.0
    pub strength: f64,               // 0.0-1.0
    pub save: SaveTarget,
}
```

---

## AugmentParams

```rust
pub struct AugmentParams {
    pub image: ImageInput,
    pub req_type: AugmentReqType,
    pub prompt: Option<String>,
    pub defry: Option<u32>,       // 0-5
    pub save: SaveTarget,
}
```

---

## UpscaleParams

```rust
pub struct UpscaleParams {
    pub image: ImageInput,
    pub scale: u32,  // 2 or 4
    pub save: SaveTarget,
}
```

---

## 結果型

### GenerateResult

```rust
pub struct GenerateResult {
    pub image_data: Vec<u8>,
    pub seed: u64,
    pub anlas_used: Option<i64>,
    pub saved_path: Option<String>,
}
```

### VibeEncodeResult

```rust
pub struct VibeEncodeResult {
    pub encoding: String,              // Base64エンコーディング
    pub information_extracted: f64,
    pub strength: f64,
    pub source_image_hash: String,     // SHA256
    pub model: String,
    pub saved_path: Option<String>,
}
```

### AugmentResult

```rust
pub struct AugmentResult {
    pub image_data: Vec<u8>,
    pub req_type: String,
    pub anlas_used: Option<i64>,
    pub saved_path: Option<String>,
}
```

### UpscaleResult

```rust
pub struct UpscaleResult {
    pub image_data: Vec<u8>,
    pub scale: u32,
    pub output_width: u32,
    pub output_height: u32,
    pub anlas_used: Option<i64>,
    pub saved_path: Option<String>,
}
```

---

## 列挙型

### Model

```rust
pub enum Model {
    NaiDiffusion45Full,
    NaiDiffusion45Curated,
    NaiDiffusion4Full,
    NaiDiffusion4Curated,
    NaiDiffusion4CuratedPreview,
    NaiDiffusion3,
}
```

### Sampler

```rust
pub enum Sampler {
    KEulerAncestral,
    KEuler,
    KDPMPP2SAncestral,
    KDPMPP2M,
    KDPMPPSDE,
    KDDIM,
}
```

### NoiseSchedule

```rust
pub enum NoiseSchedule {
    Karras,
    Exponential,
    Polyexponential,
    Native,
}
```

### AugmentReqType

```rust
pub enum AugmentReqType {
    Colorize,
    Emotion,
    Sketch,
    Lineart,
    Declutter,
    BgRemoval,
}
```

---

## エラー型

```rust
pub enum NovelAIError {
    Validation(String),
    Image(String),
    ImageFileSize { file_size_mb: f64, max_size_mb: u32, file_source: Option<String> },
    Tokenizer(String),
    TokenValidation { token_count: usize, max_tokens: usize },
    Api { status_code: u16, message: String },
    Parse(String),
    Io(std::io::Error),
    Json(serde_json::Error),
    Other(String),
}

pub type Result<T> = std::result::Result<T, NovelAIError>;
```

---

## 主要定数 (`constants.rs`)

### デフォルト値

| 定数 | 値 | 説明 |
|------|-----|------|
| `DEFAULT_WIDTH` | 832 | デフォルト幅 |
| `DEFAULT_HEIGHT` | 1216 | デフォルト高さ |
| `DEFAULT_STEPS` | 23 | デフォルトステップ数 |
| `DEFAULT_SCALE` | 5.0 | デフォルトCFGスケール |
| `DEFAULT_CFG_RESCALE` | 0.0 | デフォルトCFG Rescale |

### 制限値

| 定数 | 値 | 説明 |
|------|-----|------|
| `MAX_TOKENS` | 512 | プロンプトトークン上限 |
| `MAX_PIXELS` | 3,145,728 | 最大ピクセル数 (2048x1536) |
| `MAX_SEED` | 4,294,967,295 | 最大シード値 |
| `MIN_DIMENSION` / `MAX_GENERATION_DIMENSION` | 64 / 2048 | 寸法範囲 |
| `MIN_STEPS` / `MAX_STEPS` | 1 / 50 | ステップ数範囲 |
| `MIN_SCALE` / `MAX_SCALE` | 0.0 / 10.0 | CFGスケール範囲 |
| `MAX_CHARACTERS` | 6 | 最大キャラクター数 |
| `MAX_VIBES` | 10 | 最大Vibe数 |

### ネットワーク

| 定数 | 値 | 説明 |
|------|-----|------|
| `DEFAULT_REQUEST_TIMEOUT_MS` | 60,000 | リクエストタイムアウト |
| `MAX_RESPONSE_SIZE` | 50MB | 最大レスポンスサイズ |
| `MAX_DECOMPRESSED_IMAGE_SIZE` | 50MB | 最大展開画像サイズ |
| `MAX_ZIP_ENTRIES` | 10 | 最大ZIPエントリ数 |

---

## コスト計算 (`anlas.rs`)

純粋関数で Anlas コストを計算。API呼び出しなし。

### calculate_generation_cost

```rust
pub fn calculate_generation_cost(params: &GenerationCostParams) -> Result<GenerationCostResult>
```

### calculate_augment_cost

```rust
pub fn calculate_augment_cost(params: &AugmentCostParams) -> Result<AugmentCostResult>
```

### calculate_upscale_cost

```rust
pub fn calculate_upscale_cost(params: &UpscaleCostParams) -> Result<UpscaleCostResult>
```

### is_opus_free_generation

```rust
pub fn is_opus_free_generation(params: &GenerationCostParams) -> bool
```

Opus無料条件: width*height ≤ 1,048,576 かつ steps ≤ 28 かつ character_reference なし。

詳細は `@docs/anlas-cost-calculation.md` を参照。
