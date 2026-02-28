# #923 Rust 関数内コーディングスタイルの標準化

## Context

現在の `rust.md` は「何を使うか」（推奨クレート、型システム活用等）を規定しているが、「同じロジックをどう書くか」の判断基準が不足している。Rust は同一ロジックに複数の書き方（`match` vs `if let` vs `let-else`、イテレータ vs `for` ループ等）が可能なため、プロジェクト合意として明文化する。

3 層アプローチ: (1) clippy lint 自動強制 → (2) `rust.md` 判断基準 → (3) 既存パターン踏襲。

## 対象

- `backend/Cargo.toml` — `[workspace.lints.clippy]` 追加
- 6 member crate `Cargo.toml` — `[lints] workspace = true` 追加
- `justfile` L274 — `-- -D warnings` 削除
- `.github/workflows/ci.yaml` L121 — `-- -D warnings` 削除
- `.claude/rules/rust.md` — 「関数内スタイル」セクション追加
- `docs/05_ADR/057_workspace-lintsによるlint管理の標準化.md` — 新規
- 既存 Rust ソースファイル — 新 lint 違反の修正

## 対象外

- `unwrap_used` / `expect_used`（restriction lint、違反 1200+ 箇所、別 Issue）
- `too_many_lines` 閾値変更（既に `check-fn-size` で管理）
- rustfmt 設定変更（既に充実）
- docstring 関連 lint（`missing_errors_doc` 等、別フェーズ）

---

## Phase 1: `[workspace.lints]` インフラ整備 + ADR

lint 管理を CLI フラグから Cargo.toml 宣言的設定に移行する。

### 設計判断

`[workspace.lints.clippy]` で `all = "deny"` を設定し、CLI の `-- -D warnings` を削除する。`all` は `clippy::all` グループ（デフォルト lint）に対応し、従来の `-D warnings` と同等。宣言的管理に一本化することで二重管理を排除する。

```toml
# backend/Cargo.toml に追加
[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
```

`priority = -1` は個別 lint の override（デフォルト priority 0）より低い優先度で適用されるため、後で個別 lint を `allow` で上書きできる。

### 確認事項
- パターン: `[workspace.lints]` 構文 → Cargo 公式ドキュメント + Grep 既存使用なし（確認済み）
- パターン: CI clippy コマンド → `.github/workflows/ci.yaml` L121: `cargo clippy --all-targets --all-features -- -D warnings`
- パターン: justfile clippy コマンド → `justfile` L274: `cargo clippy {{ _cargo_q }} --all-targets --all-features -- -D warnings`

### 変更ファイル
1. `docs/05_ADR/057_workspace-lintsによるlint管理の標準化.md`（新規）
2. `backend/Cargo.toml` — `[workspace.lints.clippy]` セクション追加
3. `backend/apps/bff/Cargo.toml` — `[lints] workspace = true` 追加
4. `backend/apps/core-service/Cargo.toml` — `[lints] workspace = true` 追加
5. `backend/apps/auth-service/Cargo.toml` — `[lints] workspace = true` 追加
6. `backend/crates/domain/Cargo.toml` — `[lints] workspace = true` 追加
7. `backend/crates/infra/Cargo.toml` — `[lints] workspace = true` 追加
8. `backend/crates/shared/Cargo.toml` — `[lints] workspace = true` 追加
9. `justfile` L274 — `-- -D warnings` 削除
10. `.github/workflows/ci.yaml` L121 — `-- -D warnings` 削除

### 操作パス
該当なし（操作パスが存在しない）

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証:
- [ ] `just lint-rust` pass（`-D warnings` なしで `all = "deny"` が機能する）
- [ ] `just check` pass

---

## Phase 2: Pedantic lint の有効化と既存コード修正

関数内スタイルを自動強制する pedantic lint を有効化する。

### 有効化する lint

| lint | 効果 | レベル |
|------|------|--------|
| `manual_let_else` | `if let` + early return → `let-else` を推奨 | deny |
| `cloned_instead_of_copied` | Copy 型で `.cloned()` → `.copied()` | deny |
| `semicolon_if_nothing_returned` | unit 返却関数の末尾セミコロン一貫性 | deny |
| `redundant_else` | early return 後の不要な `else` を検出 | deny |
| `implicit_clone` | 暗黙の clone を明示化 | deny |

### 見送る lint

| lint | 見送り理由 |
|------|-----------|
| `needless_pass_by_value` | axum handler の extractor（`Json<T>`, `State<T>` 等）で偽陽性多数 |
| `module_name_repetitions` | 命名は人間判断、偽陽性が多い |
| `must_use_candidate` | 大量の `#[must_use]` 追加が必要 |
| `missing_const_for_fn` | nursery lint、Rust 1.86 で既知の問題あり |

### 確認事項
- ライブラリ: 各 lint の仕様 → `cargo clippy` 実行で違反数を確認
- パターン: `.cloned()` 28 箇所中、Copy 型に対する使用がどれだけあるか → 実行時に確認
- パターン: `semicolon_if_nothing_returned` の違反数 → 実行時に確認

### 実装手順
1. lint を `[workspace.lints.clippy]` に `"warn"` で追加
2. `cargo clippy` で違反数を確認
3. `cargo clippy --fix --allow-dirty` で自動修正可能なものを修正
4. 手動修正が必要なものを対処
5. 全 lint を `"deny"` に昇格

```toml
# backend/Cargo.toml [workspace.lints.clippy] に追加
manual_let_else = "deny"
cloned_instead_of_copied = "deny"
semicolon_if_nothing_returned = "deny"
redundant_else = "deny"
implicit_clone = "deny"
```

### 変更ファイル
1. `backend/Cargo.toml` — pedantic lint 追加
2. 既存 Rust ソースファイル — lint 違反の修正

### 操作パス
該当なし（操作パスが存在しない）

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証:
- [ ] `cargo clippy --all-targets --all-features` が warning/error なし
- [ ] `just check` pass
- [ ] 追加した `#[allow]` が最小限（各箇所に根拠コメント付き）

---

## Phase 3: `rust.md` に「関数内スタイル」セクション追加

clippy でカバーできない判断ポイントを文書化する。

### 確認事項
- パターン: `rust.md` の構造と文体 → 確認済み
- パターン: 既存コードの制御フロー・イテレータパターン → 探索済み

### 追加する内容（`rust.md` の「コーディング規約」セクション配下）

**1. 制御フロー**
- 早期リターン / ガード節を推奨。ネスト 2 段以上は平坦化
- `let-else`: `Option` / `Result` の早期脱出に使用（clippy `manual_let_else` が自動検出）
- `match` vs `if let`: enum の全バリアント → `match`、1 パターン抽出 → `if let`
- `if let` + `&&`（let chains）: 複合条件の簡潔表現

**2. イテレータ vs for ループ**
- 変換・フィルタ・集約 → イテレータチェーン
- 副作用が主目的（DB 操作、ログ、可変状態の蓄積）→ `for` ループ
- チェーン 4 段以上 → 中間 `let` 束縛かヘルパー関数に分割
- `collect` は必要になるまで遅延（型推論が困難な場合は早期 collect）

**3. 変数束縛**
- 3 段以上のメソッドチェーンが不明瞭なら中間 `let` 導入
- 構造体フィールド抽出にはデストラクチャリング活用
- 意図を明確にする名前があれば一度使い変数でも `let` 束縛

**4. クロージャ**
- 単一メソッド呼び出し → メソッド参照 `Type::method`（clippy が検出）
- 複数操作 → クロージャ `|x| x.method().another()`
- 3 行以上 → ヘルパー関数に抽出

**5. impl ブロックの構成順序**
- コンストラクタ → ゲッター → ビジネスロジック → ビルダー/変換
- trait impl は別 `impl` ブロック
- rustfmt `reorder_impl_items = true` が整列

### 変更ファイル
1. `.claude/rules/rust.md` — 「関数内スタイル」セクション追加

### 操作パス
該当なし（操作パスが存在しない）

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証:
- [ ] ガイドラインが既存コードの実態と一致（矛盾なし）
- [ ] `just check` pass（ルールファイル lint 含む）

---

## Phase 4: 最終検証

### 確認事項
- Phase 2 の `#[allow]` 箇所を Grep で確認し最小限であること
- 既存の `#[allow(clippy::too_many_arguments)]` 5 箇所に影響なし

### 検証
- [ ] `just check-all` pass
- [ ] 新 lint 導入前後の差分が合理的

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `unwrap_used` が 1200+ 箇所でスコープ外 | スコープ境界 | 対象外に明記 |
| 2回目 | `clippy::all` は pedantic を含まない | 技術的前提 | Phase 2 で個別に `"deny"` 指定 |
| 3回目 | CI と justfile 両方から `-D warnings` 削除必要 | 不完全なパス | Phase 1 変更ファイルに両方追加 |
| 4回目 | `needless_pass_by_value` は axum handler で偽陽性 | 競合・エッジケース | 見送り lint に分類 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | 完了基準 4 項目すべてに Phase が対応 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | lint 名・レベル・変更ファイルが具体的 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | CLI vs Cargo.toml、各 lint の採用/見送り理由 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象外セクションあり |
| 5 | 技術的前提 | 前提が考慮されている | OK | `clippy::all` と pedantic の関係、priority の動作 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | `rust.md` 既存内容と新セクションに矛盾なし |
