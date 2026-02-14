# ts-api → Rust 移植プラン

## Context

`ts-api/`（TypeScript, ~3,700行ソース + ~3,800行テスト）を `rust-api/` に移植する。
ts-apiはNovelAI画像生成APIのクライアントライブラリで、バリデーション・コスト計算・トークナイザー・HTTP通信・レスポンスパースを含む。

## 方針: 順次シングルセッション × 6回

**チームモードは不採用** — モジュール依存が直列（constants → anlas → schemas → utils → tokenizer → client）であり、並列化の余地が少ない。調整コストが利益を上回る。

**サブエージェントは各セッション内で適宜使用** — テスト実行やファイル探索に活用。

---

## Rust プロジェクト構造

```
rust-api/
  Cargo.toml
  src/
    lib.rs                 # クレートルート、モジュール宣言、feature gates
    error.rs               # thiserror による統一エラー型
    constants.rs           # 全定数・列挙型・デフォルト値
    anlas.rs               # コスト計算（純粋関数、I/Oなし）
    schemas/
      mod.rs               # 再エクスポート
      types.rs             # 構造体定義（GenerateParams, ImageInput等）
      validation.rs        # バリデーションロジック（validate()メソッド）
      builder.rs           # Builderパターン（GenerateParamsBuilder等）
    utils/
      mod.rs
      image.rs             # 画像バッファ処理、base64、パス判別
      mask.rs              # マスク生成（矩形・円形）
      vibe.rs              # Vibeファイル読込、エンコーディング抽出
      charref.rs           # キャラクター参照画像リサイズ
    tokenizer/
      mod.rs               # 公開API（get_t5_tokenizer, get_clip_tokenizer）
      clip.rs              # CLIP BPEトークナイザー
      t5.rs                # T5 Unigram/Viterbiトークナイザー
      cache.rs             # ディスク＋メモリキャッシュ
      preprocess.rs        # 前処理（T5, CLIP）
    client/
      mod.rs               # NovelAIClient構造体とメソッド
      payload.rs           # ペイロード構築ヘルパー群
      response.rs          # ZIP/msgpack/PNGレスポンスパース
      retry.rs             # リトライロジック（exponential backoff）
  tests/
    constants_test.rs
    anlas_test.rs
    schemas_test.rs
    utils_test.rs
    tokenizer_test.rs
    client_test.rs
```

### 主要クレート依存

| 用途 | TypeScript | Rust クレート |
|------|-----------|--------------|
| HTTP | fetch/axios | `reqwest` 0.12 |
| 非同期ランタイム | - | `tokio` 1 |
| シリアライゼーション | JSON | `serde` + `serde_json` |
| エラー処理 | throw | `thiserror` 2 |
| 画像処理 | sharp | `image` 0.25 |
| Base64 | Buffer | `base64` 0.22 |
| SHA256 | crypto | `sha2` 0.10 |
| ZIP | adm-zip | `zip` 2 |
| MessagePack | msgpackr | `rmp-serde` 1 |
| Deflate | zlib | `flate2` 1 |
| トークナイザー | tokenizers(npm) | `tokenizers` 0.20 (Rust native!) |
| HTML entity | he | `html_escape` 0.2 |
| 正規表現 | RegExp | `regex` 1 |
| LRUキャッシュ | Map手動 | `lru` 0.12 |
| 乱数 | Math.random | `rand` 0.8 |

---

## セッション1: 基盤（constants + error + anlas）

**対象TS**: `constants.ts`(232行) + `anlas.ts`(534行)
**対象テスト**: `constants.test.ts`(233行) + `anlas.test.ts`(892行)
**成果物**: ~800行ソース + ~1,000行テスト

### 作業内容

1. **`Cargo.toml` 作成** — feature flags含む完全な依存定義
2. **`src/lib.rs`** — モジュール宣言（後続フェーズ用もstub）
3. **`src/error.rs`** — `thiserror` で統一エラー型
   - `ValidationError`, `ImageError`, `TokenizerError`, `TokenValidationError`, `ApiError`, `ParseError`
4. **`src/constants.rs`**
   - API URL: `pub fn api_url() -> String` （環境変数フォールバック）
   - 列挙型: `Sampler`, `Model`, `NoiseSchedule`, `AugmentReqType`, `EmotionKeyword` （`#[derive(Serialize, Deserialize)]`）
   - 全定数: `MAX_PIXELS`, `MAX_TOKENS`, `DEFAULT_WIDTH` 等
   - `MODEL_KEY_MAP`: match式
   - `UPSCALE_COST_TABLE`: `const &[(u64, u32)]`
5. **`src/anlas.rs`**
   - `calc_v4_base_cost()`, `get_smea_multiplier()`, `is_opus_free_generation()`
   - `calc_vibe_batch_cost()`, `calc_char_ref_cost()`
   - `expand_to_min_pixels()`, `clamp_to_max_pixels()`
   - `calc_inpaint_size_correction()`
   - `calculate_generation_cost()`, `calculate_augment_cost()`, `calculate_upscale_cost()`
   - 型: `GenerationCostParams`(enum), `GenerationCostResult`, `AugmentCostParams/Result`, `UpscaleCostParams/Result`
6. **テスト**: 全テストケースを移植

### 検証
```bash
cd rust-api && cargo test
```

---

## セッション2: スキーマ・バリデーション（schemas）

**対象TS**: `schemas.ts`(627行)
**対象テスト**: `schemas.test.ts`(1,529行)
**成果物**: ~700行ソース + ~1,200行テスト

### 作業内容

1. **`src/schemas/types.rs`** — 全構造体定義
   - `ImageInput` enum: `FilePath(String)`, `Base64(String)`, `DataUrl(String)`, `Bytes(Vec<u8>)`
   - `CharacterConfig`, `CharacterReferenceConfig`
   - `GenerateParams`（全フィールドOption付き）, `GenerateResult`
   - `EncodeVibeParams`, `VibeEncodeResult`
   - `AugmentParams`, `AugmentResult`
   - `UpscaleParams`, `UpscaleResult`
   - `AnlasBalanceResponse`
2. **`src/schemas/validation.rs`** — バリデーション
   - `GenerateParams::validate()` → `validate_action_dependencies()`, `validate_vibe_params()`, `validate_pixel_constraints()`, `validate_save_options_exclusive()`
   - `GenerateParams::validate_async()` → 上記 + `validate_token_counts()`（トークナイザーfeature時）
   - `AugmentParams::validate()`, `UpscaleParams::validate()`, `EncodeVibeParams::validate()`
   - パストラバーサル: `validate_safe_path()`
3. **`src/schemas/builder.rs`** — Builderパターン
   - `GenerateParamsBuilder` — `.prompt()`, `.width()`, `.height()` 等、`.build()` でデフォルト適用+バリデーション
4. **ヘルパー関数**: `character_to_caption_dict()`, `character_to_negative_caption_dict()`
5. **テスト**: 全テストケースを移植

### 設計ポイント
- Zodの `z.default()` → Rust `Default` trait + `Option<T>.unwrap_or_default()`
- Zodの `superRefine` → 明示的な `validate()` メソッド
- 非同期バリデーション（トークンカウント）は `validate_async()` に分離

### 検証
```bash
cd rust-api && cargo test
```

---

## セッション3: ユーティリティ（utils）

**対象TS**: `utils.ts`(485行)
**対象テスト**: `utils.test.ts`(527行)
**成果物**: ~500行ソース + ~500行テスト

### 作業内容

1. **`src/utils/image.rs`**
   - `get_image_buffer(input: &ImageInput) -> Result<Vec<u8>>`
   - `looks_like_file_path(s: &str) -> bool`
   - `validate_image_data_size(data: &[u8]) -> Result<()>`
   - `get_image_base64(input: &ImageInput) -> Result<String>`
   - `get_image_dimensions(data: &[u8]) -> Result<(u32, u32)>` （`image` crateで）
2. **`src/utils/mask.rs`**
   - `calculate_cache_secret_key(data: &[u8]) -> String` （SHA256）
   - `resize_mask_image(mask: &[u8], w: u32, h: u32) -> Result<Vec<u8>>` （1/8リサイズ）
   - `create_rectangular_mask(w, h, region) -> Result<Vec<u8>>` （ピクセル直接操作+PNG）
   - `create_circular_mask(w, h, center, radius) -> Result<Vec<u8>>`
3. **`src/utils/vibe.rs`**
   - `load_vibe_file(path: &str) -> Result<serde_json::Value>`
   - `extract_encoding(data: &Value, model: &str) -> Result<(String, f64)>`
   - `process_vibes(vibes, model) -> Result<ProcessedVibes>`
4. **`src/utils/charref.rs`**
   - `prepare_character_reference_image(buffer: &[u8]) -> Result<Vec<u8>>`（アスペクト比判別→リサイズ+黒パディング）
   - `process_character_references(refs) -> Result<CharRefProcessResult>`
5. **テスト**: 全テストケースを移植

### リスク: 画像処理の互換性
- `sharp` → `image` crateでリサイズ結果が完全一致しない可能性
- `fit: 'contain'`（黒パディング付き）は手動実装が必要
- `fit: 'fill'`（アスペクト比無視）は `image::imageops::resize` で対応

### 検証
```bash
cd rust-api && cargo test
```

---

## セッション4: トークナイザー（tokenizer）

**対象TS**: `tokenizer.ts`(789行)
**対象テスト**: `tokenizer.test.ts`(602行)
**成果物**: ~800行ソース + ~600行テスト

### 作業内容

1. **`src/tokenizer/clip.rs`** — CLIP BPEトークナイザー
   - `bytes_to_unicode()`: バイト→Unicode文字マッピング
   - `NovelAIClipTokenizer`: BPEマージ、正規表現プリトークン化、LRUキャッシュ(10k件)
   - CLIP前処理: HTMLエンティティ2回デコード、小文字化、空白正規化
2. **`src/tokenizer/t5.rs`** — T5 Unigramトークナイザー
   - `PureUnigram`: Viterbi DP、コードポイント単位イテレーション、NFKC正規化
   - `NovelAIT5Tokenizer`: HF `tokenizers` crate（Rust native!）をプライマリ、PureUnigram をフォールバック
3. **`src/tokenizer/preprocess.rs`**
   - `preprocess_t5()`: ブラケット除去、ウェイト構文除去
   - CLIP前処理関数
4. **`src/tokenizer/cache.rs`**
   - ディスクキャッシュ（7日TTL）: `.cache/tokenizers/` に保存
   - メモリシングルトン: `tokio::sync::OnceCell<Arc<T>>`
   - HTTP取得 + deflate解凍（`flate2`）
5. **テスト**: 全テストケースを移植

### 大きな利点
HF `tokenizers` crateはRustネイティブ（npmの`tokenizers`パッケージはこのRustコードのバインディング）。TSの「native path」と同等以上の性能がデフォルトで得られる。

### 検証
```bash
cd rust-api && cargo test
```

---

## セッション5: クライアント前半（HTTP・ペイロード・リトライ）

**対象TS**: `client.ts`(993行)の前半 — コンストラクタ、リトライ、ペイロード構築、ファイル保存
**成果物**: ~600行ソース

### 作業内容

1. **`src/client/mod.rs`** — NovelAIClient
   - `struct NovelAIClient { api_key, http_client, logger }`
   - `new(api_key: Option<String>) -> Result<Self>`
   - `get_anlas_balance() -> Result<AnlasBalance>`
   - `encode_vibe()`, `generate()`, `augment_image()`, `upscale_image()` のスケルトン
   - `Logger` trait定義
   - ファイル保存: `validate_save_path()`, `ensure_dir()`, `save_to_file()`
2. **`src/client/retry.rs`** — リトライロジック
   - `fetch_with_retry()`: exponential backoff (1000ms × 2^attempt × jitter)
   - 429 + ネットワークエラーでリトライ、400/401/500等は即エラー
   - タイムアウト: `reqwest::Client::timeout(60s)`
3. **`src/client/payload.rs`** — ペイロード構築
   - `GenerationPayload`, `GenerationPayloadParameters` 構造体（`#[derive(Serialize)]`）
   - `build_base_payload()`: 基本パラメータ組み立て
   - `apply_img2img_params()`: source_image → base64
   - `apply_infill_params()`: mask 1/8リサイズ、model "-inpainting"、cache_secret_key
   - `apply_vibe_params()`: reference_image_multiple配列
   - `apply_char_ref_params()`: director_reference_*パラメータ
   - `build_v4_prompt_structure()`: v4_prompt/v4_negative_prompt
   - `apply_character_prompts()`: characterPrompts配列

### 検証
```bash
cd rust-api && cargo test  # コンパイル確認、ユニットテスト
```

---

## セッション6: クライアント後半（レスポンスパース・統合テスト）

**対象TS**: `client.ts` 後半 — レスポンスパース、generate/augment/upscale/encodeVibeの完成
**成果物**: ~400行ソース + ~300行テスト

### 作業内容

1. **`src/client/response.rs`** — レスポンスパース
   - `parse_zip_response()`: ZIPからPNG抽出 + セキュリティチェック（エントリ数、サイズ、圧縮比）
   - `parse_stream_response()`: フォールバックチェーン
     1. ZIP署名(PK) → `parse_zip_response()`
     2. PNGシグネチャ → そのまま返却
     3. msgpackパース → data/imageフィールド
     4. PNGマジックバイト検索 → IENDまでスライス
   - ZIPボム防御: `MAX_ZIP_ENTRIES=10`, `MAX_DECOMPRESSED_SIZE=50MB`, `MAX_COMPRESSION_RATIO=100`
2. **`generate()` 完成** — バリデーション→ペイロード構築→API呼出→レスポンスパース→保存
3. **`augment_image()` 完成** — 画像サイズ自動検出→ペイロード構築→API呼出→パース→保存
4. **`upscale_image()` 完成**
5. **`encode_vibe()` 完成** — .naiv4vibeファイル構造生成・保存
6. **統合テスト** — HTTPモック（`mockito` crate等）でE2Eフロー検証
7. **`README.md`** + 使用例

### 検証
```bash
cd rust-api && cargo test
cd rust-api && cargo clippy
cd rust-api && cargo doc --open
```

---

## リスク一覧

| リスク | 影響度 | 対策 |
|--------|--------|------|
| `image` crateのリサイズ結果が`sharp`と異なる | 高 | FilterType調整、実APIで検証 |
| f64精度差によるコスト計算の1ずれ | 中 | 892件のテスト全移植で即検出 |
| CLIP BPEのUnicode正規表現の挙動差 | 中 | `regex` crateの`unicode-perl`有効化 |
| msgpack `unpackMultiple` の再現 | 中 | `rmp::decode::read_value`ループ |
| `tokenizers` crateバージョン互換 | 低 | PureUnigramフォールバック保持 |

---

## まとめ

| セッション | フェーズ | 推定ソース行 | 推定テスト行 | 指示の要点 |
|-----------|---------|------------|------------|-----------|
| 1 | constants + error + anlas | ~800 | ~1,000 | 基盤＋純粋関数。ここが全ての土台 |
| 2 | schemas/validation | ~700 | ~1,200 | 型定義＋バリデーション。最もテストが多い |
| 3 | utils | ~500 | ~500 | 画像処理。`image` crateとの格闘 |
| 4 | tokenizer | ~800 | ~600 | CLIP BPE + T5。HF crateはRust native |
| 5 | client前半 | ~600 | — | HTTP/リトライ/ペイロード構築 |
| 6 | client後半 + 統合 | ~400 | ~300 | レスポンスパース/統合テスト/仕上げ |
| **合計** | | **~3,800** | **~3,600** | |

各セッションでは該当TSソース・テストファイルの全文と、このプランの該当セクションを提示すること。
