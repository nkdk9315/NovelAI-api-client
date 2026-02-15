# トークナイザー内部実装

`src/tokenizer/` モジュール群のアルゴリズム詳細。移植時の参考用。

## 概要

2種類のトークナイザーを実装:

| トークナイザー | アルゴリズム | 用途 | 実装方式 |
|---------------|------------|------|---------|
| CLIP | BPE (Byte Pair Encoding) | 生トークン数カウント | 純Rust (`src/tokenizer/clip.rs`) |
| T5 | Unigram (SentencePiece) | プロンプトバリデーション (上限512) | 純Rust (`src/tokenizer/t5.rs`) |

画像生成 API のプロンプトバリデーションには T5 トークナイザーが使用される。

---

## データソース

### トークナイザー定義ファイル

| トークナイザー | URL | 形式 |
|---------------|-----|------|
| CLIP | `https://novelai.net/tokenizer/compressed/clip_tokenizer.def?v=2&static=true` | deflate圧縮 → JSON |
| T5 | `https://novelai.net/tokenizer/compressed/t5_tokenizer.def?v=2&static=true` | deflate圧縮 → JSON |

### キャッシュ戦略

**ディスクキャッシュ** (`src/tokenizer/cache.rs`):
- 保存先: `.cache/tokenizers/` (カレントディレクトリ相対、`NOVELAI_CACHE_DIR` で上書き可)
- ファイル名: `<tokenizer_name>_v<version>.json` (URLから自動生成、サニタイズ済み)
- TTL: 7日間 (`CACHE_TTL = 604,800秒`)
- パストラバーサル防御: `validate_cache_path()` でキャッシュディレクトリ外への書き込みを拒否

**メモリキャッシュ (シングルトン)**:
- `tokio::sync::OnceCell<Arc<NovelAIClipTokenizer>>` / `OnceCell<Arc<NovelAIT5Tokenizer>>`
- **OnceCell によるスレッドセーフ初期化**: 並行リクエストによる重複ダウンロードを防止
- force_refresh=true で OnceCell をバイパスし再取得可能

### デコンプレッション (`cache.rs:decompress_data`)

サーバーレスポンスは deflate 圧縮:
1. `flate2::DeflateDecoder` を試行 (raw deflate) — `.take()` でサイズ制限 (50MB)
2. 失敗時: `flate2::ZlibDecoder` を試行 (zlib wrapped deflate) — 同様にサイズ制限
3. 両方失敗: `NovelAIError::Tokenizer` エラー

---

## CLIP BPE アルゴリズム (`src/tokenizer/clip.rs`)

### bytesToUnicode マッピング

バイト値 (0-255) を Unicode 文字にマッピングする関数。GPT-2 スタイルのBPEで使用:

- `!` (33) 〜 `~` (126), `¡` (161) 〜 `¬` (172), `®` (174) 〜 `ÿ` (255) → そのまま
- それ以外のバイト → 256 以降の Unicode 文字にマッピング

### ボキャブラリ構築

1. 初期ボキャブラリ: `initial_vocab` (ASCII + Latin拡張、約250文字)
2. 各文字 + `</w>` サフィックスを追加
3. マージペア (定義ファイルの2〜48895行目) の結合結果を追加
4. `<|startoftext|>`, `<|endoftext|>` を追加

### 正規表現プリトークン化

```regex
/<\|startoftext\|>|<\|endoftext\|>|'s|'t|'re|'ve|'m|'ll|'d|[\p{L}]+|[\p{N}]|[^\s\p{L}\p{N}]+/
```

テキストをこの正規表現でマッチングし、各トークンに対してBPEを適用。

### BPE マージアルゴリズム

1. トークンを文字に分割。最後の文字に `</w>` を付与
2. 全隣接ペアを取得 (インデックスペアで重複排除)
3. マージランクが最小のペアを結合
4. 1文字になるかマージ不可になるまで繰り返す
5. 結果をスペース区切り文字列としてキャッシュ

### LRU キャッシュ

- サイズ上限: 10,000件 (`BPE_CACHE_MAX_SIZE`)
- 実装: `lru::LruCache` クレート
- Mutex で保護 (poisoning recovery 付き)

### CLIP 前処理

```rust
// 1. HTMLエンティティを2回デコード (html_escape::decode_html_entities)
// 2. 空白正規化 + トリム
// 3. 小文字化 (to_lowercase)
```

### エンコード手順

1. 前処理 (HTMLデコード、小文字化、空白正規化)
2. 正規表現でプリトークン化
3. 各トークンを UTF-8 バイト列に変換
4. `bytes_to_unicode` で Unicode 文字列に変換
5. BPE アルゴリズム適用
6. スペース分割 → ボキャブラリ辞書でトークンIDに変換

---

## T5 Unigram アルゴリズム (`src/tokenizer/t5.rs`)

### 実装方式

TS版と異なり、Rust版は **PureUnigram のみ** (native fallback なし):

```
get_t5_tokenizer()
  └─ JSON パース → PureUnigram → NovelAIT5Tokenizer::from_pure_unigram()
```

### T5 前処理 (`src/tokenizer/preprocess.rs:preprocess_t5`)

CLIP と異なり、**最小限の前処理のみ**:

```rust
// 1. ブラケット除去: [] と {} を削除
// 2. ウェイト構文除去: "2::content::" → "content"
```

**行わないこと** (CLIP との違い):
- HTMLエンティティデコードしない
- 空白正規化しない
- 小文字化しない

### PureUnigram 実装

#### コンストラクタ

```rust
pub fn new(vocab_entries: Vec<(String, f64)>, unk_id: u32) -> Self
```

- `vocab_entries`: `(piece, log_score)` のペアベクタ (JSON の `model.vocab`)
- `unk_id`: 未知トークンのID (`model.unk_id`)
- `unk_score`: `min(scores) - 10` (SentencePiece の `kUnkPenalty`)
- `max_piece_length`: ボキャブラリ中の最長ピース長 (コードポイント単位)
- NaN スコアはスキップして DP テーブル汚染を防止

#### エンコード手順

1. **NFKC 正規化**: `unicode_normalization` クレートの `.nfkc()` (Precompiled normalizer の近似)
2. **WhitespaceSplit**: 空白文字で分割
3. **Metaspace**: 各ピースに `▁` (U+2581) を前置
4. **Viterbi**: 各ピースに対して最適分割を実行

#### Viterbi アルゴリズム

最高スコアの分割を動的計画法で求める:

```
入力: "▁hello" (コードポイント配列)
best[0] = { score: 0, prev: 0 }

for i = 1 to len:
  for l = 1 to min(maxPieceLength, i):
    substr = text[byte_offsets[i-l]..byte_offsets[i]]  // 事前計算バイトオフセット
    if vocab.has(substr):
      candidate = best[i-l].score + vocab.get(substr)
      if candidate > best[i].score:
        best[i] = { score: candidate, prev: i-l }
  if best[i].score === -Infinity:
    // 未知文字: 1文字ずつ unk として処理
    best[i] = { score: best[i-1].score + unkScore, prev: i-1 }

// バックトラックでピース列を復元
// ピース → piece_to_id マップでトークンIDに変換
```

バイトオフセットを事前計算し、`&str` スライスで効率的にサブストリングを取得。

#### count_tokens_only 最適化

トークンカウントのみが必要な場合、`viterbi_count` でバックトラック時にカウントのみ行い、
ピース Vec の構築を省略。

---

## トークンカウント

### count_tokens()

```rust
pub fn count_tokens(&self, text: &str) -> usize
```

- EOS トークン (`</s>`, ID=1) を**含む**カウントを返す
- NovelAI 公式サイトの表示と一致
- 空テキスト → `1` (EOSのみ)

### validate_token_count() (`src/tokenizer/cache.rs`)

```rust
pub async fn validate_token_count(text: &str) -> Result<usize>
```

- `count_tokens()` の結果が `MAX_TOKENS (512)` を超えた場合 `NovelAIError::TokenValidation` エラー

### バリデーションでのトークンカウント

`src/schemas/validation.rs` の `validate_token_counts()`:
- ポジティブプロンプト: ベースプロンプト + 全キャラクタープロンプトの合計
- ネガティブプロンプト: ベースネガティブ + 全キャラクターネガティブの合計
- 各合計が `MAX_TOKENS (512)` 以下であること
- トークナイザー障害時: バリデーションをスキップ (TS版と同じ挙動)

---

## エラー型

| 型 | 用途 |
|----|------|
| `NovelAIError::Tokenizer(String)` | トークナイザーの初期化・ダウンロード失敗 |
| `NovelAIError::TokenValidation { token_count, max_tokens }` | トークン数超過 |

---

## TS版との差分

| 項目 | TS版 | Rust版 |
|------|------|--------|
| T5バックエンド | native (tokenizers npm) → pure JS fallback | PureUnigram のみ |
| BPEキャッシュ | 手動 Map + LRU | `lru::LruCache` + Mutex |
| シングルトン | Promise キャッシュ | `tokio::sync::OnceCell` |
| デコンプレッション | `zlib.inflateRawSync` / `inflateSync` | `flate2::DeflateDecoder` / `ZlibDecoder` + `.take()` |
| BPEランクキー | 文字列タプル `(String, String)` | 同 (TS版はセパレータ結合だったがバグ修正済み) |
| count最適化 | なし | `count_tokens_only` / `viterbi_count` で Vec 省略 |
