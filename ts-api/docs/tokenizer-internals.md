# トークナイザー内部実装

`src/tokenizer.ts` (約790行) のアルゴリズム詳細。移植時の参考用。

## 概要

2種類のトークナイザーを実装:

| トークナイザー | アルゴリズム | 用途 | 実装方式 |
|---------------|------------|------|---------|
| CLIP | BPE (Byte Pair Encoding) | 生トークン数カウント | 純JavaScript |
| T5 | Unigram (SentencePiece) | プロンプトバリデーション (上限512) | native → pure JS fallback |

画像生成 API のプロンプトバリデーションには T5 トークナイザーが使用される。

---

## データソース

### トークナイザー定義ファイル

| トークナイザー | URL | 形式 |
|---------------|-----|------|
| CLIP | `https://novelai.net/tokenizer/compressed/clip_tokenizer.def?v=2&static=true` | deflate圧縮 → JSON |
| T5 | `https://novelai.net/tokenizer/compressed/t5_tokenizer.def?v=2&static=true` | deflate圧縮 → JSON |

### キャッシュ戦略

**ディスクキャッシュ**:
- 保存先: `.cache/tokenizers/` (プロジェクトルート相対)
- ファイル名: `<tokenizer_name>_v<version>.json` (URLから自動生成)
- TTL: 7日間 (`CACHE_TTL_MS = 604,800,000ms`)
- パストラバーサル防御: `validateCachePath()` で `.cache/tokenizers/` 外への書き込みを拒否

**メモリキャッシュ (シングルトン)**:
- `cachedClipTokenizerPromise` / `cachedT5TokenizerPromise`
- **Promise をキャッシュ**: 並行リクエストによる重複ダウンロードを防止
- 失敗時は Promise キャッシュをクリアし、次回リトライ可能

### デコンプレッション

サーバーレスポンスは deflate 圧縮:
1. `zlib.inflateRawSync()` を試行 (raw deflate)
2. 失敗時: `zlib.inflateSync()` を試行 (zlib wrapped deflate)
3. 両方失敗: `TokenizerError` スロー

---

## CLIP BPE アルゴリズム

### bytesToUnicode マッピング

バイト値 (0-255) を Unicode 文字にマッピングする関数。GPT-2 スタイルのBPEで使用:

- `!` (33) 〜 `~` (126), `¡` (161) 〜 `¬` (172), `®` (174) 〜 `ÿ` (255) → そのまま
- それ以外のバイト → 256 以降の Unicode 文字にマッピング

### ボキャブラリ構築

1. 初期ボキャブラリ: `INITIAL_VOCAB` (ASCII + Latin拡張、約250文字)
2. 各文字 + `</w>` サフィックスを追加
3. マージペア (定義ファイルの2〜48895行目) の結合結果を追加
4. `<|startoftext|>`, `<|endoftext|>` を追加

### 正規表現プリトークン化

```regex
/<\|startoftext\|>|<\|endoftext\|>|'s|'t|'re|'ve|'m|'ll|'d|[\p{L}]+|[\p{N}]|[^\s\p{L}\p{N}]+/gu
```

テキストをこの正規表現でマッチングし、各トークンに対してBPEを適用。

### BPE マージアルゴリズム

1. トークンを文字に分割。最後の文字に `</w>` を付与
2. 全隣接ペアを取得
3. マージランクが最小のペアを結合
4. 1文字になるかマージ不可になるまで繰り返す
5. 結果をスペース区切り文字列としてキャッシュ

### LRU キャッシュ

- サイズ上限: 10,000件 (`BPE_CACHE_MAX_SIZE`)
- 実装: `Map` の挿入順序を利用 (アクセス時に delete → set で末尾に移動)
- 満杯時: `Map.keys().next()` で最古のエントリを削除

### CLIP 前処理

```typescript
// 1. HTMLエンティティを2回デコード
text = he.decode(he.decode(text)).trim();
// 2. 空白正規化 + 小文字化
text = text.replace(/\s+/g, ' ').trim().toLowerCase();
```

### エンコード手順

1. 前処理 (HTMLデコード、小文字化、空白正規化)
2. 正規表現でプリトークン化
3. 各トークンを UTF-8 バイト列に変換
4. `bytesToUnicode` で Unicode 文字列に変換
5. BPE アルゴリズム適用
6. スペース分割 → ボキャブラリ辞書でトークンIDに変換

---

## T5 Unigram アルゴリズム

### native vs pure JS fallback

```
getT5Tokenizer()
  ├─ tryLoadNativeTokenizer() → import('tokenizers')
  │   ├─ 成功 → Tokenizer.fromString() → NovelAIT5Tokenizer.createFromNative()
  │   └─ 失敗 → nativeTokenizerUnavailable = true
  │
  └─ フォールバック
      └─ JSON パース → PureJSUnigram → NovelAIT5Tokenizer.createFromPureJS()
```

- `tokenizers` パッケージは optional dependency
- macOS ARM (Apple Silicon) では `tokenizers-darwin-arm64` が未公開のため、native が使えない場合がある
- 一度失敗すると `nativeTokenizerUnavailable` フラグで以降の試行をスキップ

### T5 前処理 (`preprocessT5`)

CLIP と異なり、**最小限の前処理のみ**:

```typescript
// 1. ブラケット除去: [] と {} を削除
text = text.replace(/[[\]{}]/g, "");

// 2. ウェイト構文除去: "2::content::" → "content"
text = text.replace(/(-?\d+\.?\d*)?::((?:(?!::)[\s\S])+)(?:::)/g, '$2');
```

**行わないこと** (CLIP との違い):
- HTMLエンティティデコードしない
- 空白正規化しない
- 小文字化しない

### PureJSUnigram 実装

#### コンストラクタ

```typescript
constructor(vocabEntries: [string, number][], unkId: number)
```

- `vocabEntries`: `[piece, logScore]` のペア配列 (JSON の `model.vocab`)
- `unkId`: 未知トークンのID (`model.unk_id`)
- `unkScore`: `min(scores) - 10` (SentencePiece の `kUnkPenalty`)
- `maxPieceLength`: ボキャブラリ中の最長ピース長 (コードポイント単位)

#### エンコード手順

1. **NFKC 正規化**: `text.normalize('NFKC')` (Precompiled normalizer の近似)
2. **WhitespaceSplit**: 空白文字で分割
3. **Metaspace**: 各ピースに `▁` (U+2581) を前置
4. **Viterbi**: 各ピースに対して最適分割を実行

#### Viterbi アルゴリズム

最高スコアの分割を動的計画法で求める:

```
入力: "▁hello" (コードポイント配列)
best[0] = { score: 0, prev: -1 }

for i = 1 to len:
  for l = 1 to min(maxPieceLength, i):
    substr = chars[i-l..i]
    if vocab.has(substr):
      candidate = best[i-l].score + vocab.get(substr)
      if candidate > best[i].score:
        best[i] = { score: candidate, prev: i-l }
  if best[i].score === -Infinity:
    // 未知文字: 1文字ずつ unk として処理
    best[i] = { score: best[i-1].score + unkScore, prev: i-1 }

// バックトラックでピース列を復元
// ピース → pieceToId マップでトークンIDに変換
```

コードポイント単位のイテレーション (`Array.from(text)`) でサロゲートペア (絵文字等) を正しく処理。

---

## トークンカウント

### countTokens()

```typescript
async countTokens(text: string): Promise<number>
```

- EOS トークン (`</s>`, ID=1) を**含む**カウントを返す
- NovelAI 公式サイトの表示と一致
- 空テキスト → `1` (EOSのみ)

### validateTokenCount()

```typescript
async validateTokenCount(text: string): Promise<number>
```

- `countTokens()` の結果が `MAX_TOKENS (512)` を超えた場合 `TokenValidationError` スロー
- スキーマバリデーション内では `schemas.ts:validateTokenCounts()` から間接的に呼ばれる

### バリデーションでのトークンカウント

`schemas.ts` の `validateTokenCounts()`:
- ポジティブプロンプト: ベースプロンプト + 全キャラクタープロンプトの合計
- ネガティブプロンプト: ベースネガティブ + 全キャラクターネガティブの合計
- 各合計が `MAX_TOKENS (512)` 以下であること
- トークナイザー障害時: バリデーションをスキップ (警告ログのみ)

---

## エラークラス

| クラス | 用途 |
|--------|------|
| `TokenizerError` | トークナイザーの初期化・ダウンロード失敗 |
| `TokenValidationError` | トークン数超過 (`tokenCount`, `maxTokens` プロパティ付き) |

---

## テスト・デバッグ

直接実行でトークンカウントを確認可能:

```bash
pnpm exec tsx src/tokenizer.ts "your prompt here"
```

キャッシュクリア:
```typescript
import { clearTokenizerCache } from './src/tokenizer';
clearTokenizerCache();  // メモリキャッシュ + native フラグをリセット
```

ディスクキャッシュは `.cache/tokenizers/` を手動削除。
