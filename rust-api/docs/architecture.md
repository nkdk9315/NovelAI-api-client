# アーキテクチャ

## モジュール依存グラフ

```
client/mod.rs
  ├── client/payload.rs   (ペイロード構築)
  ├── client/response.rs  (レスポンスパース)
  ├── client/retry.rs     (リトライロジック)
  ├── schemas/*           (型定義, バリデーション)
  ├── utils/*             (画像処理, vibe, charref)
  ├── tokenizer/*         (トークナイザー, バリデーション経由)
  ├── constants           (定数, enum)
  └── error               (エラー型)

schemas/
  ├── types.rs            (構造体・enum定義)
  ├── builder.rs          (GenerateParamsBuilder, types.rs に依存)
  └── validation.rs       (バリデーション, constants + tokenizer に依存)

utils/
  ├── mod.rs              (validate_safe_path)
  ├── image.rs            (ImageInput → バイト列変換)
  ├── mask.rs             (マスク生成・リサイズ, image.rs に依存)
  ├── vibe.rs             (Vibe ファイル処理, constants に依存)
  └── charref.rs          (CharRef 画像前処理, image.rs に依存)

tokenizer/
  ├── mod.rs              (公開API re-export)
  ├── clip.rs             (CLIP BPE, 独立)
  ├── t5.rs               (T5 Unigram, preprocess に依存)
  ├── preprocess.rs       (T5前処理, 独立)
  └── cache.rs            (DL・キャッシュ・初期化, clip/t5 に依存)

anlas.rs                  (コスト計算, constants のみに依存, 独立性が高い)
constants.rs              (全定数, 依存なし)
error.rs                  (エラー型, 依存なし)
```

---

## 設計判断とその理由

### データ付き Enum (`GenerateAction`)

```rust
pub enum GenerateAction {
    Generate,
    Img2Img { source_image: ImageInput, strength: f64, noise: f64 },
    Infill { source_image: ImageInput, mask: ImageInput, ... },
}
```

**理由**: img2img には source_image/strength が必須、infill にはさらに mask/mask_strength が必須。
データ付き enum にすることで、`Generate` 時に不要なフィールドが存在しない状態を型レベルで保証。
TS版では `action: string` + Optional フィールドの組み合わせだが、Rust では不正な組み合わせをコンパイル時に排除できる。

### Builder パターン (`GenerateParamsBuilder`)

**理由**: `GenerateParams` のフィールド数が多く (14+)、コンストラクタでは扱いにくい。
Builder パターンにより:
- 必須フィールドのみ設定し、残りはデフォルト値
- `.build()` でバリデーション実行
- メソッドチェーンによる可読性の高い API

TS版では Zod スキーマでバリデーションするが、Rust では Builder + 手動バリデーション。

### SecretString (APIキー保護)

```rust
use secrecy::SecretString;
```

**理由**: APIキーが `Debug` 出力やログに漏洩するのを防止。
`SecretString` は `Display`/`Debug` でマスクされ、`.expose_secret()` で明示的にアクセスする必要がある。
TS版にはこの保護がない。

### thiserror (エラー型)

```rust
#[derive(Debug, Error)]
pub enum NovelAIError { ... }
```

**理由**: `std::error::Error` の手動実装を省略し、一貫したエラーメッセージを提供。
各バリアントにデータを持たせることで、エラーハンドリング時に構造的な情報にアクセスできる。
TS版では `Error` サブクラスだが、Rust では enum で網羅的パターンマッチが可能。

### ImageInput enum (柔軟入力)

```rust
pub enum ImageInput {
    FilePath(PathBuf),
    Base64(String),
    DataUrl(String),
    Bytes(Vec<u8>),
}
```

**理由**: ユーザーの画像データの保持形式に合わせた柔軟な入力を提供。
内部で `get_image_buffer()` が各バリアントを統一的にバイト列に変換。
TS版では `string | Buffer | Uint8Array` のユニオン型で、ヒューリスティック判定するが、
Rust では enum で明示的に型を指定。

### SaveTarget enum (保存先の排他制御)

```rust
pub enum SaveTarget {
    None,
    ExactPath(String),
    Directory { dir: String, filename: Option<String> },
}
```

**理由**: TS版では `save_path` と `save_dir` が両方 Optional で、同時指定がランタイムエラー。
Rust では enum にすることで排他性を型レベルで保証。不正な組み合わせはコンパイル不可。

### tokio::sync::OnceCell (トークナイザーシングルトン)

**理由**: トークナイザーの初期化は高コスト (ネットワーク DL + JSON パース)。
`OnceCell` でスレッドセーフな遅延初期化を実現し、並行リクエストによる重複初期化を防止。
TS版では Promise キャッシュだが、`OnceCell` はより堅牢。

### PureUnigram のみ (native fallback なし)

**理由**: TS版は `tokenizers` npm パッケージ (native) → pure JS fallback のフォールバック構成。
Rust では PureUnigram の性能が十分なため、native バインディング不要。
コードの複雑性を削減し、プラットフォーム互換性を確保。

---

## TS版との差分表

| 項目 | TS版 | Rust版 |
|------|------|--------|
| アクション指定 | `action: "generate" \| "img2img" \| "infill"` + Optional | `GenerateAction` enum (データ付き) |
| 保存先 | `save_path?: string, save_dir?: string` (排他チェックはランタイム) | `SaveTarget` enum (型で排他保証) |
| 画像入力 | `string \| Buffer \| Uint8Array` (ヒューリスティック判定) | `ImageInput` enum (明示的) |
| APIキー保護 | なし | `secrecy::SecretString` |
| バリデーション | Zod スキーマ (`superRefine`) | Builder `.build()` + `validate()`/`validate_async()` |
| エラー型 | `Error` サブクラス | `NovelAIError` enum (thiserror) |
| T5トークナイザー | native → pure JS fallback | PureUnigram のみ |
| BPEキャッシュ | 手動 Map LRU | `lru::LruCache` + Mutex (poisoning recovery) |
| シングルトン | Promise キャッシュ | `tokio::sync::OnceCell` |
| 画像処理 | `sharp` (libvips) | `image` クレート (pure Rust) |
| JSON処理 | Zod parse | `serde_json::Value` 手動構築 |
| 画像リサイズ (CharRef) | `sharp` contain fit | `image::resize` + 黒キャンバスにオーバーレイ |
| マスクリサイズ | `sharp` fill fit + グレースケール | `image::resize_exact` + `to_luma8` |
| ZIPボム防御 | ヘッダサイズ + 圧縮比チェック | `.take()` で実際の解凍サイズを制限 |
| リトライ対象 | 429 + ネットワークエラー | 429 + 502/503 + ネットワークエラー |
| count最適化 | なし | `viterbi_count` で Vec 生成省略 |

---

## セキュリティモデル

### APIキー保護
- `secrecy::SecretString` でメモリ上の APIキーを保護
- `Debug`/`Display` でマスク、`expose_secret()` で明示アクセス

### パストラバーサル防御 (3層)
1. **SaveTarget バリデーション** (`validation.rs`): `..` セグメントを拒否
2. **画像ファイル読み込み** (`image.rs`): `validate_safe_path()` で検証
3. **トークナイザーキャッシュ** (`cache.rs`): `validate_cache_path()` でキャッシュディレクトリ外書き込みを拒否

### ZIPボム防御
- ZIP エントリ数制限 (`MAX_ZIP_ENTRIES = 10`)
- `.take()` で解凍サイズを制限 (`MAX_DECOMPRESSED_IMAGE_SIZE = 50MB`)
- ヘッダのサイズ宣言を信頼せず、実際の解凍バイト数で検証

### レスポンスサイズ制限
- Content-Length ヘッダによる事前チェック
- ダウンロード後の実サイズチェック (`MAX_RESPONSE_SIZE = 50MB`)

### 画像デコンプレッションボム防御
- `load_image_safe()`: ヘッダのみ読み取りで寸法チェック → ピクセル上限超過で拒否
- 全デコードは寸法チェック通過後のみ

### Base64入力サイズ制限
- `MAX_BASE64_INPUT_LEN = 14MB` (デコード後 ~10MB)

### トークナイザーデータ解凍
- `flate2` の `.take()` で解凍サイズを 50MB に制限

---

## generate() データフロー

```
1. GenerateParams
     ↓ validate_async() [寸法/範囲/制約 + T5トークン数チェック]
2. シード決定 (指定 or ランダム生成)
     ↓
3. Vibe処理 (未エンコードならAPI経由でエンコード)
     ↓
4. Character Reference処理 (画像リサイズ + パディング)
     ↓
5. ペイロード構築
     ├── build_base_payload()
     ├── build_v4_prompt_structure()
     ├── apply_character_prompts()
     ├── apply_img2img_params() / apply_infill_params()
     ├── apply_vibe_params()
     └── apply_char_ref_params()
     ↓
6. エンドポイント選択
     ├── charref or infill → STREAM_URL
     └── otherwise → API_URL
     ↓
7. fetch_with_retry() [exponential backoff, max 3 retries]
     ↓
8. レスポンスパース
     ├── ZIP → parse_zip_response()
     └── Stream → parse_stream_response()
         ├── ZIP signature → parse_zip_response()
         ├── PNG signature (先頭) → そのまま
         ├── PNG magic (末尾検索) → IEND まで切り出し
         └── msgpack → data/image フィールド抽出
     ↓
9. ファイル保存 (SaveTarget に応じて)
     ↓
10. GenerateResult 返却
```

---

## テスト戦略

### テスト構成

```
tests/
├── anlas_test.rs       # コスト計算のユニットテスト (純粋関数, ネットワーク不要)
├── client_test.rs      # クライアントのテスト (mockito でHTTPモック)
├── constants_test.rs   # 定数・enum のテスト
├── schemas_test.rs     # バリデーションテスト
└── utils_test.rs       # ユーティリティテスト (画像処理, パス検証等)
```

### テスト原則

- **ネットワーク不要**: `mockito` でHTTPレスポンスをモック
- **並列実行制御**: `serial_test` で環境変数依存テストを直列化
- **テストデータ**: `tempfile` で一時ファイル/ディレクトリを自動クリーンアップ
- **コスト計算**: `anlas.rs` は純粋関数のため、ネットワークなしで完全テスト可能
