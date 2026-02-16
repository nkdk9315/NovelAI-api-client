# トークナイザー内部実装

`Sources/NovelAIAPI/Tokenizer/` ディレクトリのアルゴリズム詳細。移植時の参考用。

## 概要

2種類のトークナイザーを実装:

| トークナイザー | アルゴリズム | 用途 | 実装方式 |
|---------------|------------|------|---------|
| CLIP | BPE (Byte Pair Encoding) | 生トークン数カウント | 純Swift (`CLIPTokenizer.swift`) |
| T5 | Unigram (SentencePiece) | プロンプトバリデーション (上限512) | 純Swift (`T5Tokenizer.swift`) |

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
- 保存先: `~/.cache/tokenizers/` (ホームディレクトリ相対)
- ファイル名: `<tokenizer_name>_v<version>.json` (URLから自動生成)
- TTL: 7日間

**`TokenizerCacheManager` actor (メモリキャッシュ)**:
- `TokenizerCacheManager.shared` (シングルトン actor)
- actor の直列化で並行リクエストによる重複ダウンロードを自動防止
- TypeScript 版の「Promise をキャッシュ」パターンは不要 — actor が同等の機能を提供
- 失敗時は次回リトライ可能 (キャッシュ値を nil に戻す)

```swift
public actor TokenizerCacheManager {
    public static let shared = TokenizerCacheManager()

    private var clipTokenizer: NovelAIClipTokenizer?
    private var t5Tokenizer: NovelAIT5Tokenizer?

    public func getClipTokenizer() async throws -> NovelAIClipTokenizer
    public func getT5Tokenizer() async throws -> NovelAIT5Tokenizer
    public func validateTokenCount(_ text: String) async throws -> Int
    public func clearCache()
}
```

### デコンプレッション

サーバーレスポンスは deflate 圧縮。`Compression` フレームワーク (Foundation) で解凍:

1. `decompressRawDeflate()` を試行 (raw deflate, `COMPRESSION_ZLIB`)
2. 失敗時: `decompressZlib()` を試行 (zlib wrapped deflate, 2バイトヘッダー検証)
3. 両方失敗: `NovelAIError.tokenizer` スロー
4. 解凍サイズ上限: 50MB (`MAX_RESPONSE_SIZE_TOKENIZER`) でデコンプレッションボム防止

---

## CLIP BPE アルゴリズム

### bytesToUnicode マッピング

バイト値 (0-255) を Unicode 文字にマッピングする関数。GPT-2 スタイルのBPEで使用:

- `!` (33) 〜 `~` (126), `¡` (161) 〜 `¬` (172), `®` (174) 〜 `ÿ` (255) → そのまま
- それ以外のバイト → 256 以降の Unicode 文字にマッピング

### ボキャブラリ構築

1. 初期ボキャブラリ: `buildInitialVocab()` (ASCII + Latin拡張、約250文字)
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

- サイズ上限: 10,000件
- 実装: `NSLock` + `Dictionary` でスレッドセーフ
- アクセス時に delete → set で末尾に移動 (挿入順序ベース)
- 満杯時: 最古のエントリを削除

```swift
class NovelAIClipTokenizer: @unchecked Sendable {
    private let cacheLock = NSLock()
    private var bpeCache: [String: String]  // max 10,000 entries
}
```

### CLIP 前処理

```swift
// 1. HTMLエンティティを2回デコード (カスタム decodeHTMLEntities())
text = decodeHTMLEntities(decodeHTMLEntities(text)).trimmingCharacters(in: .whitespaces)
// 2. 空白正規化 + 小文字化
text = text.replacingOccurrences(of: "\\s+", with: " ", options: .regularExpression)
    .trimmingCharacters(in: .whitespaces).lowercased()
```

TypeScript 版との違い:
- `he` ライブラリの代わりにカスタム `decodeHTMLEntities()` を使用
- HTML エンティティのルックアップテーブルを内蔵

### エンコード手順

1. 前処理 (HTMLデコード、小文字化、空白正規化)
2. 正規表現でプリトークン化
3. 各トークンを UTF-8 バイト列に変換
4. `bytesToUnicode` で Unicode 文字列に変換
5. BPE アルゴリズム適用
6. スペース分割 → ボキャブラリ辞書でトークンIDに変換

---

## T5 Unigram アルゴリズム

### 純Swift 実装 (PureUnigram)

TypeScript 版では native `tokenizers` パッケージ → PureJS フォールバックの2段構成だったが、Swift 版は **純Swift `PureUnigram` のみ**。native バインディングは不要。

```swift
class PureUnigram {
    let vocab: [String: Double]     // piece → logScore
    let pieceToId: [String: Int]    // piece → token ID
    let unkId: Int                  // 未知トークン ID
    let unkScore: Double            // min(scores) - 10
    let maxPieceLength: Int         // 最長ピース長 (コードポイント単位)
}
```

### T5 前処理 (`preprocessT5`)

CLIP と異なり、**最小限の前処理のみ**:

```swift
// Preprocess.swift
func preprocessT5(_ text: String) -> String {
    // 1. ブラケット除去: [] と {} を削除
    // 2. ウェイト構文除去: "2::content::" → "content"
}
```

**行わないこと** (CLIP との違い):
- HTMLエンティティデコードしない
- 空白正規化しない
- 小文字化しない

### PureUnigram エンコード手順

1. **NFKC 正規化**: `String.precomposedStringWithCompatibilityMapping` (TypeScript の `text.normalize('NFKC')` に相当)
2. **WhitespaceSplit**: 空白文字で分割
3. **Metaspace**: 各ピースに `▁` (U+2581) を前置
4. **Viterbi**: 各ピースに対して最適分割を実行

### Viterbi アルゴリズム

最高スコアの分割を動的計画法で求める:

```
入力: "▁hello" (コードポイント配列 — Unicode.Scalar 単位)
best[0] = { score: 0, prev: -1 }

for i = 1 to len:
  for l = 1 to min(maxPieceLength, i):
    substr = chars[i-l..i]
    if vocab.has(substr):
      candidate = best[i-l].score + vocab[substr]
      if candidate > best[i].score:
        best[i] = { score: candidate, prev: i-l }
  if best[i].score == -Double.infinity:
    // 未知文字: 1文字ずつ unk として処理
    best[i] = { score: best[i-1].score + unkScore, prev: i-1 }

// バックトラックでピース列を復元
// ピース → pieceToId マップでトークンIDに変換
```

Unicode.Scalar 単位のイテレーション (`text.unicodeScalars`) でBMP外の文字 (絵文字等) を正しく処理。

---

## トークンカウント

### countTokens()

```swift
// NovelAIT5Tokenizer
func countTokens(_ text: String) -> Int
```

- EOS トークン (`</s>`, ID=1) を**含む**カウントを返す
- NovelAI 公式サイトの表示と一致
- 空テキスト → `1` (EOSのみ)

### validateTokenCount()

```swift
// TokenizerCacheManager
public func validateTokenCount(_ text: String) async throws -> Int
```

- `countTokens()` の結果が `MAX_TOKENS (512)` を超えた場合 `NovelAIError.tokenValidation` スロー
- トークナイザー障害時: バリデーションをスキップ (警告ログのみ)

### バリデーションでのトークンカウント

ポジティブプロンプト・ネガティブプロンプトそれぞれについて:
- ベースプロンプト + 全キャラクタープロンプトの合計が `MAX_TOKENS (512)` 以下であること
- トークナイザー障害時: バリデーションをスキップ (警告ログのみ)

---

## エラー型

| ケース | 用途 |
|--------|------|
| `NovelAIError.tokenizer(String)` | トークナイザーの初期化・ダウンロード失敗 |
| `NovelAIError.tokenValidation(String)` | トークン数超過 |

---

## テスト・デバッグ

直接実行でトークンカウントを確認可能:

```bash
swift run ExampleTokenizer "your prompt here"
```

キャッシュクリア:
```swift
await TokenizerCacheManager.shared.clearCache()
```

ディスクキャッシュは `~/.cache/tokenizers/` を手動削除。
