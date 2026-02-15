# CODE_REVIEW.md 修正プラン — 2セッション構成

## Context

`CODE_REVIEW.md` の50件を2セッションで修正する。各セッション内はPhase分けし、モジュール単位でエージェントを並列起動する。

## 構成概要

```
Session 1: 非破壊的修正 (46件)
  Phase 1 [直列] → 基盤変更 (Cargo.toml, error.rs, 共通関数作成)
  Phase 2 [並列] → モジュール単位で6エージェント同時実行
  Phase 3 [直列] → cargo test + 統合確認

Session 2: 破壊的型変更 (4件)
  Phase 1 [直列] → types.rs の型定義変更
  Phase 2 [並列] → 波及先を2エージェントで修正
  Phase 3 [直列] → cargo test + 統合確認
```

---

## Session 1: 非破壊的修正 (46件)

**PR**: `fix: security hardening, bug fixes, code deduplication, and enum modernization`
**新規依存**: `secrecy = "0.8"`, `strum = "0.26"`, `strum_macros = "0.26"`

### Phase 1: 基盤変更（直列・メインエージェント）

他のエージェントが依存する変更を先に実施する。

| 作業 | 対象ファイル | 内容 |
|------|-------------|------|
| 依存追加 | `Cargo.toml` | `secrecy`, `strum`, `strum_macros` 追加 |
| エラー型統合 | `src/error.rs` | #21: `Range` を `Validation` に統合、全 `Range(...)` を置換 |
| 共通パス検証関数 | `src/utils/mod.rs` | #8/#39: `pub(crate) fn validate_safe_path()` をセグメント単位チェックで作成 |

### Phase 2: モジュール並列修正（6エージェント同時）

Phase 1完了後、以下を並列実行。各エージェントは担当ファイルの全問題を一括修正する。

#### Agent 1: `src/constants.rs`
| # | 問題 | 修正内容 |
|---|------|----------|
| 6 | SSRF env var URL注入 | URLスキーム・ドメイン検証 |
| 36 | URL関数毎回アロケーション | `OnceLock` でキャッシュ |
| 37 | MAX_SEED マジックナンバー | `u32::MAX` に置換 |
| 28 | enum/serde/as_str三重管理 | `strum` derive で統一 |
| 41 | model_key_from_str重複 | `FromStr` impl で削除 |
| 42 | enum FromStr/Hash未実装 | strum で自動導出 |

#### Agent 2: `src/anlas.rs`
| # | 問題 | 修正内容 |
|---|------|----------|
| 4 | Inpaintゼロ次元 | `.max(grid)` でクランプ |
| 17 | i64→u64キャスト | `InpaintCorrectionResult` を `u64` に |
| 22 | Opus無料判定の非対称性 | 補正後次元で統一 |
| 23 | エラーがOk(...)で返る | `Err(...)` で返却 |

#### Agent 3: `src/utils/` (image.rs, mask.rs, charref.rs, vibe.rs)
| # | 問題 | ファイル | 修正内容 |
|---|------|----------|----------|
| 2 | デコンプレッション爆弾 (3箇所) | image.rs, charref.rs, mask.rs | `load_image_safe()` で次元チェック |
| 3 | Base64メモリ枯渇 | image.rs | デコード前にbase64文字列長を検証 |
| 38 | get_image_dimensionsフルデコード | image.rs | `Reader::into_dimensions()` |
| 40 | PNGエンコード3重複 | mask.rs, charref.rs | `encode_to_png()` 共通関数 |
| 16 | Vibeファイル読込無制限 | vibe.rs | `metadata().len()` で事前チェック |
| 8/39 | sanitize_file_path重複除去 | image.rs, vibe.rs | Phase 1で作成した共通関数に置換 |

#### Agent 4: `src/tokenizer/` (cache.rs, clip.rs, t5.rs)
| # | 問題 | ファイル | 修正内容 |
|---|------|----------|----------|
| 9/26 | Thundering herd + RwLock in async | cache.rs | `tokio::sync::OnceCell` or `tokio::sync::RwLock` に置換 |
| 10 | validate_cache_path dead code | cache.rs | `#[allow(dead_code)]` 除去、read/writeに接続 |
| 11 | HTTPステータス未検証 | cache.rs | `is_success()` チェック追加 |
| 12 | レスポンスサイズ後チェック | cache.rs | `Content-Length` ヘッダ先行チェック |
| 13 | JSON unwrap()パニック | cache.rs | `.ok_or_else()` で安全にパース |
| 15 | デコンプレッション出力無制限 | cache.rs | `take()` でサイズ制限 |
| 48 | HTTPクライアント再作成 | cache.rs | `OnceLock<reqwest::Client>` でキャッシュ |
| 27 | Mutex/RwLockポイズニング | cache.rs, clip.rs | `.unwrap_or_else(\|e\| e.into_inner())` |
| 32 | NaN Viterbi汚染 | t5.rs | `min_score` → `f64::INFINITY` + NaN除外 |
| 44 | BPEセパレータ衝突リスク | clip.rs | タプルキーに変更 |
| 46 | Viterbiアロケーション | t5.rs | バイトオフセット事前計算 |
| 47 | count_tokens不要Vec構築 | t5.rs | カウント専用パス追加 |

#### Agent 5: `src/schemas/` (validation.rs, types.rs, builder.rs)
| # | 問題 | ファイル | 修正内容 |
|---|------|----------|----------|
| 8/39 | パス走査重複除去 | validation.rs | Phase 1の共通関数に置換 |
| 24 | Vibe強度範囲未検証 | validation.rs | 個別値の0.0〜1.0チェック追加 |
| 25 | VibeItem::FilePath走査未検証 | validation.rs | `validate_safe_path()` 呼出追加 |
| 35 | pubフィールドでバリデーション回避 | validation.rs | `pub(crate)` 化検討 |
| 43 | 0.0..=1.0チェック11重複 | validation.rs | `validate_unit_range()` ヘルパー |
| 50 | FilePath(String) → PathBuf | types.rs | `PathBuf` に変更 |

#### Agent 6: `src/client/` (mod.rs, retry.rs, response.rs)
| # | 問題 | ファイル | 修正内容 |
|---|------|----------|----------|
| 1 | truncate_text UTF-8パニック | retry.rs | `char_indices()` でバイト境界回避 |
| 5 | ZIP爆弾 | response.rs | `take()` で実際の展開サイズを制限 |
| 7 | APIキー平文保持 | mod.rs | `secrecy::SecretString` に変更 |
| 12 | レスポンスサイズ後チェック | response.rs | `Content-Length` ヘッダ先行チェック |
| 14 | MsgPack無制限ネスト | response.rs | バッファサイズを制限してからパース |
| 29 | generate() 130行 | mod.rs | サブ関数に分割 |
| 30/34 | 毎操作2回の余分HTTP + 競合 | mod.rs | 残高チェックをオプション化 |
| 31 | body_str.clone()毎リトライ | retry.rs | `&str` 参照渡しに変更 |
| 33 | HTTP 502/503リトライなし | retry.rs | リトライ対象ステータス追加 |
| 49 | User-Agent未設定 | retry.rs | ヘッダ追加 |

### Phase 3: 統合確認（直列・メインエージェント）

```bash
cargo test
cargo clippy
```

コンパイルエラー・テスト失敗があれば修正。

---

## Session 2: 破壊的型変更 (4件)

**PR**: `refactor: introduce data enums for type-safe configuration`
**前提**: Session 1がマージ済み

### Phase 1: 型定義変更（直列・メインエージェント）

`src/schemas/types.rs` を直接編集：

| # | 問題 | 修正内容 |
|---|------|----------|
| 18 | 並列Vibeベクタ → VibeConfig | `VibeConfig` 構造体導入、`vibes: Option<Vec<VibeConfig>>` |
| 19 | GenerateAction Option地獄 | データ付きenumに変更（Img2Img/Infillが専用フィールド保持） |
| 20 | save_path相互排他 → SaveTarget | `SaveTarget` enum導入 |

### Phase 2: 波及先修正（2エージェント並列）

#### Agent A: schemas/ + tests/
| ファイル | 修正内容 |
|----------|----------|
| `src/schemas/validation.rs` | 新型に合わせてバリデーション書き換え |
| `src/schemas/builder.rs` | ビルダーメソッドを新型に対応 |
| `tests/schemas_test.rs` | テスト更新 |

#### Agent B: client/ + utils/ + tests/
| ファイル | 修正内容 |
|----------|----------|
| `src/client/mod.rs` | generate()を新型に対応 |
| `src/client/payload.rs` | ペイロード構築を新型に対応 |
| `src/utils/vibe.rs` | VibeConfig対応 |
| `tests/client_test.rs` | テスト更新 |
| `tests/utils_test.rs` | テスト更新 |

#### Agent C: tokenizer/ (独立)
| # | 問題 | ファイル | 修正内容 |
|---|------|----------|----------|
| 45 | BPE内部ループ過剰アロケーション | `src/tokenizer/clip.rs` | 文字列処理最適化 |

### Phase 3: 統合確認

```bash
cargo test
cargo clippy
```

---

## まとめ

| Session | 件数 | エージェント数 | リスク | 所要見積 |
|---------|------|---------------|--------|----------|
| 1 | 46件 | Phase1: 1 + Phase2: 6 | **中** | エージェント並列で大部分完了 |
| 2 | 4件 | Phase1: 1 + Phase2: 3 | **高** | 型変更の波及が広範 |

**新規クレート依存** (Session 1でまとめて追加):
- `secrecy = "0.8"` — APIキーゼロ化
- `strum = "0.26"` + `strum_macros = "0.26"` — enum文字列自動導出
