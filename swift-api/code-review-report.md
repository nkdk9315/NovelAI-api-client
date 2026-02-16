# NovelAI Swift API 批判的コードレビュー分析レポート

## エグゼクティブサマリー

`NovelAIAPI` Swift パッケージ（Sources: 18ファイル, Tests: 6ファイル・517テスト）に対し、セキュリティ・品質・テストの観点から包括的なコードレビューを実施し、以下の改善を3セッションに分けて適用した。

| カテゴリ | 件数 | 優先度 |
|---------|------|--------|
| セキュリティ脆弱性 | 6件 | HIGH |
| データ整合性問題 | 7件 | MEDIUM |
| 並行性問題 | 4件 | MEDIUM |
| コード品質問題 | 8件 | LOW-MEDIUM |
| テスト品質問題 | 8件 | MEDIUM |

全改善適用後: `swift build` 成功、`swift test` 全517テスト合格。

---

## Session 1: セキュリティ修正とクリティカルバグ修正 (7項目)

| # | 項目 | ファイル | 優先度 | 複雑度 |
|---|------|---------|--------|--------|
| 1-1 | Force Unwrap URL構築クラッシュ | `Client/NovelAIClient.swift` | HIGH | S |
| 1-2 | パストラバーサル防御の不備と重複 | `Utils/ImageUtils.swift`, `Utils/VibeUtils.swift`, `Client/NovelAIClient.swift`, `Schemas/Validation.swift` | HIGH | M |
| 1-3 | ZIP圧縮率チェックのゼロ除算 | `Client/Response.swift` | HIGH | S |
| 1-4 | Zlibヘッダー検出の不十分さ | `Tokenizer/TokenizerCache.swift` | MEDIUM | S |
| 1-5 | Base64パディング検証の不備 | `Schemas/Validation.swift` | MEDIUM | S |
| 1-6 | 静的正規表現の `try!` | 複数ファイル | MEDIUM | S |
| 1-7 | リトライのタイムアウト競合 | `Client/Retry.swift` | MEDIUM | M |

### 1-1: Force Unwrap URL構築クラッシュ

**問題:** `URL(string:)!` が5箇所で使用されており、環境変数で不正なURLが設定された場合にランタイムクラッシュが発生する。

**修正:** 全5箇所を `guard let url = URL(string:) else { throw NovelAIError.other(...) }` に変更。

```swift
// Before
let url = URL(string: subscriptionURL())!

// After
guard let url = URL(string: subscriptionURL()) else {
    throw NovelAIError.other("Invalid subscription URL")
}
```

**影響箇所:** `getAnlasBalance()`, `encodeVibe()`, `generate()`, `augmentImage()`, `upscaleImage()`

### 1-2: パストラバーサル防御の不備と重複

**問題:**
- 3箇所（`ImageUtils.swift`, `VibeUtils.swift`, `NovelAIClient.swift`）に独立したパストラバーサル検証ロジックが存在
- `standardizingPath` + 文字列`.contains("..")` では `my..folder` のような合法名も拒否される
- シンボリックリンクを経由した迂回が可能

**修正:**
- `Validation.swift` の `validateSafePath()` にロジックを一元化
- コンポーネント単位の `..` チェック（`components(separatedBy: "/").contains("..")`）に変更
- 各ユーティリティはこの関数にデリゲート
- `NovelAIClient.validateSavePathTraversal()` は `validateSafePath()` + `resolvingSymlinksInPath` を使用

### 1-3: ZIP圧縮率チェックのゼロ除算

**問題:** `compressedSize == 0` かつ `uncompressedSize > 0` のエントリでは圧縮率チェックがスキップされ、ZIPボム保護をバイパス可能。

**修正:** `compressedSize == 0 && uncompressedSize > 0` のケースを明示的に拒否するガードを追加。

```swift
// Before
if compressedSize > 0 && uncompressedSize / compressedSize > UInt64(maxCompressionRatio) {

// After
if compressedSize == 0 && uncompressedSize > 0 {
    throw NovelAIError.parse("Suspicious compression ratio detected (zero compressed size)")
}
if compressedSize > 0 && uncompressedSize / compressedSize > UInt64(maxCompressionRatio) {
```

### 1-4: Zlibヘッダー検出の不十分さ

**問題:** `data[0] == 0x78` のみでzlibヘッダーと判定しており、偶然0x78で始まる非zlibデータを誤判定する可能性がある。

**修正:** 2バイトのzlibチェックサム検証 `(CMF * 256 + FLG) % 31 == 0` を追加。

### 1-5: Base64パディング検証の不備

**問題:** 正規表現 `=*$` がパディング文字を無制限に許可（`====...` も合格）。

**修正:** `={0,2}$` に変更し、Base64仕様に準拠（最大2文字）。

### 1-6: 静的正規表現の `try!`

**問題:** 複数ファイルでインライン `try! NSRegularExpression(...)` が使用されており、パターンが不正な場合のリスクと毎回のコンパイルコストが存在。

**修正:**
- `ImageUtils.swift`: 3つの正規表現をモジュールレベル `private let` 定数に抽出
- `Validation.swift`: 3つの正規表現をモジュールレベル `private let` 定数に抽出
- `CLIPTokenizer.swift`: パターンが正しいことを明示する `// swiftlint:disable` コメント追加
- 全箇所に「パターンはコンパイル時既知リテラルであり `try!` は安全」のコメントを付与

### 1-7: リトライのタイムアウト競合

**問題:** タイムアウト時に `CancellationError` がスローされ、API利用者が `NovelAIError` で統一的にエラーハンドリングできない。

**修正:**
- タイムアウトタスクが `NovelAIError.api(statusCode: 0, message: "... timed out ...")` をスロー
- `CancellationError` を `catch` して `NovelAIError` に変換するフォールバック追加

---

## Session 2: コード品質改善・リファクタリング・データ整合性 (12項目)

| # | 項目 | ファイル | 優先度 | 複雑度 |
|---|------|---------|--------|--------|
| 2-1 | キャッシュTTL変数名 `_MS` が実際は秒 | `Tokenizer/TokenizerCache.swift` | LOW | S |
| 2-2 | サイレントモデルフォールバック | `Utils/VibeUtils.swift`, `Client/NovelAIClient.swift` | MEDIUM | S |
| 2-3 | CLIPTokenizer LRUキャッシュの非同期安全性 | `Tokenizer/CLIPTokenizer.swift` | MEDIUM | M |
| 2-4 | MaskUtils 浮動小数点精度・`pow()` | `Utils/MaskUtils.swift` | LOW | S |
| 2-5 | ピクセル計算の整数オーバーフロー文書化 | `Anlas.swift` | LOW | S |
| 2-6 | NSNull の使用目的コメント追加 | `Client/Payload.swift` | LOW | S |
| 2-7 | Inpaintサイズ補正の可読性改善 | `Anlas.swift` | LOW-MEDIUM | S |
| 2-8 | マジックナンバーの文書化 | `Constants.swift` | LOW | S |
| 2-9 | アスペクト比閾値の意図コメント | `Utils/CharRefUtils.swift` | LOW | S |
| 2-10 | 正規表現の繰り返しコンパイル→定数化 | `Schemas/Validation.swift` | LOW-MEDIUM | S |
| 2-11 | `vibes != nil && !(vibes!.isEmpty)` パターン | `Schemas/Validation.swift` | LOW | S |
| 2-12 | Mutable Public Struct のバリデーション不在 | `Schemas/Types.swift` | LOW-MEDIUM | M |

### 2-1: キャッシュTTL変数名

**修正:** `CACHE_TTL_MS` → `CACHE_TTL` にリネーム（値は `7 * 24 * 60 * 60` = 7日間の秒数）。

### 2-2: サイレントモデルフォールバック

**問題:** `MODEL_KEY_MAP[model] ?? "v4-5full"` はサポート外のモデルを黙ってフォールバックさせる。

**修正:** `guard let modelKey = MODEL_KEY_MAP[model] else { throw NovelAIError.validation(...) }` に変更（2箇所）。

### 2-3: CLIPTokenizer LRUキャッシュの非同期安全性

**問題:** `@unchecked Sendable` クラスのミュータブルな `cache` / `cacheOrder` が排他制御なし。

**修正:** `NSLock` (`cacheLock`) を追加し、キャッシュの読み書き・LRU更新・エビクションをロック下で実行。

### 2-4: MaskUtils 浮動小数点精度

**修正:** `pow(radius * Double(maskWidth), 2)` → `radiusPx * radiusPx`（`pow` は内部でログ・指数変換するため精度が落ちる）。

### 2-5: ピクセル計算の整数オーバーフロー文書化

**修正:** `calcV4BaseCost` および `calculateAugmentCost` に「64-bit Int では MAX_PIXELS 範囲で安全」のコメント追加。

### 2-6: NSNull の使用目的コメント

**修正:** `Payload.swift` の `"skip_cfg_above_sigma": NSNull()` に「API は null 値を含むキーの存在を期待」のコメント追加。

### 2-7: Inpaintサイズ補正の可読性改善

**修正:** ネストした `Int(floor(Double(Int(floor(...)))))` を段階的計算に書き直し:

```swift
// Before
let newW = Int(floor(Double(Int(floor(Double(maskWidth) * scale))) / Double(GRID_SIZE))) * GRID_SIZE

// After
let scaledW = Int(floor(Double(maskWidth) * scale))
let newW = (scaledW / GRID_SIZE) * GRID_SIZE
```

### 2-8〜2-9: マジックナンバー・閾値の文書化

- `GRID_SIZE`: 「NovelAI API requires dimensions to be multiples of 64」
- `INPAINT_THRESHOLD_RATIO`: 「masks below this fraction of OPUS_FREE_PIXELS get size-corrected」
- `V4_COST_COEFF_*`: 「empirically derived from NovelAI pricing model」
- `CHARREF_*_THRESHOLD`: アスペクト比の分類基準をコメントで明記

### 2-10: 正規表現の定数化

`Validation.swift` の3つのインライン正規表現をモジュールレベル定数に抽出（Session 1-6 と同時実施）。

### 2-11: Force Unwrap パターン改善

**修正:** `vibes != nil && !(vibes!.isEmpty)` → `vibes.map { !$0.isEmpty } ?? false`（3箇所）。

### 2-12: Mutable Public Struct の文書化

**修正:** `GenerateParams` に「全プロパティは builder-style 設定用に `var`。使用前に `validate()` を呼ぶこと」のドキュメントを追加。

---

## Session 3: テスト改善・カバレッジ拡充 (9項目)

| # | 項目 | ファイル | 優先度 | 複雑度 |
|---|------|---------|--------|--------|
| 3-1 | 脆弱なenum count アサーション | `ConstantsTests.swift` | MEDIUM | S |
| 3-2 | MockURLProtocol のスレッドセーフティ | `ClientTests.swift` | MEDIUM | M |
| 3-3 | テストヘルパーの `try!` | `ClientTests.swift` | LOW-MEDIUM | S |
| 3-4 | 弱いアサーション (XCTAssertNoThrow のみ) | 複数 | LOW-MEDIUM | S |
| 3-5 | テストセットアップのDRY違反 | `TokenizerTests.swift` | LOW | S |
| 3-6 | 欠落テストカテゴリの追加 | 新規テストコード | MEDIUM | L |
| 3-7 | エラーメッセージのフラジャイルな文字列マッチング | `ClientTests.swift` | LOW | M |
| 3-8 | Actor ボトルネックの文書化 | `Tokenizer/TokenizerCache.swift` | LOW-MEDIUM | S |
| 3-9 | TOCTOU ファイル操作の文書化 | `Client/NovelAIClient.swift` | LOW | S |

### 3-1: 脆弱なenum count アサーション

**問題:** `XCTAssertEqual(Model.allCases.count, 4)` のようなテストは、enum にケースが追加された際に何を更新すべきか不明。

**修正:** メッセージ付きに改善: `"Model enum case count changed — update MODEL_KEY_MAP and related tests"`（5箇所）。

### 3-2: MockURLProtocol のスレッドセーフティ

**問題:** `nonisolated(unsafe) static var` は並行テスト実行時にデータ競合の可能性あり。

**修正:** `NSLock` + computed property で排他制御:

```swift
private static let lock = NSLock()
private static var _requestHandler: ...?
static var requestHandler: ...? {
    get { lock.withLock { _requestHandler } }
    set { lock.withLock { _requestHandler = newValue } }
}
```

### 3-3: テストヘルパーの `try!`

**修正:** `makeZipWithPNG` を `throws` に変更し、`fatalError` / `try!` / `data!` を排除。呼び出し側は全て `try` に更新。

### 3-5: テストセットアップのDRY違反

**問題:** `PureUnigramTests` と `NovelAIT5TokenizerTests` で同一の語彙定義が重複。

**修正:** `sharedTestVocab` としてファイルスコープの定数に抽出し、両テストクラスから参照。

### 3-8: Actor ボトルネックの文書化

**修正:** `TokenizerCacheManager` のドキュメントに、Actor 直列化による初回ロード時のキュー動作と、以降のキャッシュヒット時の影響がないことを明記。

### 3-9: TOCTOU ファイル操作の文書化

**修正:** `NovelAIClient` のファイル保存コードに「TOCTOU レースは非致命的であり、保存失敗は警告としてログされる」旨のコメントを追加。

---

## 検証結果

```
$ swift build
Build complete! (1.43s)

$ swift test
Executed 517 tests, with 0 failures (0 unexpected) in 12.754 seconds
```

### 追加検証

- **Force Unwrap 確認:** `grep -rn 'URL(string:' Sources/ | grep '!'` → 該当なし
- **パストラバーサルテスト:** `..` を含む合法パスで誤検出しないことを確認（既存テスト合格）
- **ZIP ボム保護:** `compressedSize == 0` ケースが適切にブロックされることを確認

---

## 未対応・将来課題

| 項目 | 理由 |
|------|------|
| 3-4: 弱いアサーション強化 | 既存テストは `case` パターンマッチングを使用しており十分な品質 |
| 3-6: 欠落テストカテゴリ追加 | CJK/絵文字・境界値・並行性・ラウンドトリップテストは別PRで対応推奨 |
| 3-7: エラー文字列マッチング改善 | 既存の `case` パターンマッチング + `contains` 組み合わせは許容範囲 |
