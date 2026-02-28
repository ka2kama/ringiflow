# 計画: #834 instrumentation.sh を rust-script に移行

## Context

`scripts/check/instrumentation.sh`（137行）は `sed`/`grep` で Rust ソースコードを解析して `#[tracing::instrument]` の漏れを検出するスクリプト。Shell の正規表現ベースの解析はジェネリクス・ライフタイム・複数行シグネチャに対応できず、`syn` クレートの AST 解析で正確性を向上させたい。

Epic #841 の高優先度タスク。先行実績 `.github/scripts/match-rules.rs` の rust-script パターンを踏襲する。

## 設計判断

### 1. ファイル配置: `scripts/check/instrumentation.rs`

現在のパスと同じ `scripts/check/` に配置。ADR-056 のスコープ拡大（同 PR 内）により `scripts/` でも ADR-015 基準該当時に rust-script を許容する。

### 2. ファイル検出: `git ls-files` via `std::process::Command`

代替案 `globset` クレートは `**` の解釈が異なり振る舞い互換性が損なわれる。`git ls-files` は VCS 管理外ファイルを自動除外でき、現行スクリプトと完全一致する。

### 3. 行番号取得: `proc-macro2` の `span-locations` feature

`syn::Span::start().line` で行番号を取得。`proc-macro2 = { features = ["span-locations"] }` を明示的に依存に追加。

### 4. 属性検出: `Attribute::path().segments` でパスセグメントを比較

`#[tracing::instrument(...)]` と `#[instrument(...)]`（短縮形）の両方をサポート。

### 5. trait/impl 区別: `syn::Item` variant で自然に分離

`Item::Trait` のメソッドは無視、`Item::Impl` 内の `ImplItem::Fn` のみ検査。Shell の `is_impl_method`（`{` vs `;` 前方スキャン）に相当するヒューリスティクスが不要になる。

### 6. CI: `code-quality` ジョブに Rust toolchain + rust-script を追加

`claude-rules-check.yaml` の既存パターンを踏襲。sccache + Cargo キャッシュで初回ビルド時間を最小化。

## 変更対象ファイル

| ファイル | 操作 |
|---------|------|
| `scripts/check/instrumentation.rs` | 新規作成 |
| `scripts/check/instrumentation.sh` | 削除 |
| `justfile` (L429-430) | `rust-script` 呼び出しに変更 |
| `.github/workflows/ci.yaml` (L715-739) | Rust toolchain + rust-script セットアップ追加 |
| `docs/05_ADR/056_CIスクリプトの言語選定方針.md` | スコープ拡大記載 |
| `docs/06_ナレッジベース/devtools/rust-script.md` | 使用箇所追記 |
| `.claude/rules/observability.md` | パス参照を `.rs` に更新 |

対象外: `scripts/check/parallel.sh`（`just check-instrumentation` を呼ぶだけで変更不要）

## rust-script ヘッダー

```rust
#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! syn = { version = "2", features = ["full", "parsing"] }
//! proc-macro2 = { version = "1", features = ["span-locations"] }
//! tempfile = "3"
//! ```
```

## コア型

```rust
struct InstrumentationError {
    file: String,
    line: usize,
    kind: TargetKind,
    fn_name: String,
}

enum TargetKind {
    Handler,
    RepositoryImpl,
}
```

出力フォーマット（現行と一致）:
- 成功: `✅ すべてのハンドラ・リポジトリに計装が設定されています`
- 失敗: `❌ 計装漏れが見つかりました (N 件):\n  - file:line: ハンドラ fn_name に #[tracing::instrument] がありません`

## コア関数

```rust
const EXCLUDE_FUNCTIONS: &[&str] = &["health_check"];

fn git_ls_files(pattern: &str) -> Vec<String>;
fn has_instrument_attr(attrs: &[syn::Attribute]) -> bool;
fn is_excluded(fn_name: &str) -> bool;
fn check_handler_file(path: &str, content: &str) -> Vec<InstrumentationError>;
fn check_repository_file(path: &str, content: &str) -> Vec<InstrumentationError>;
fn run() -> i32;
fn main() { std::process::exit(run()); }
```

### `check_handler_file` ロジック

`syn::parse_file` → `Item::Fn` を走査 → `pub` + `async` のみ対象 → 除外リスト確認 → `has_instrument_attr` で属性チェック。

### `check_repository_file` ロジック

`syn::parse_file` → `Item::Impl` を走査 → `ImplItem::Fn` で `async` のみ対象（`pub` チェック不要：trait impl ではメソッドの可視性を再宣言しない）→ 除外リスト確認 → `has_instrument_attr` で属性チェック。`Item::Trait` は完全に無視。

## Phase 1: instrumentation.rs の実装と既存スクリプトの置換

### 確認事項

- 型: `syn::Item`, `syn::ItemFn`, `syn::ItemImpl`, `syn::ImplItem::Fn`, `syn::Attribute` → docs.rs/syn/2/
- パターン: match-rules.rs のスクリプト構造 → `.github/scripts/match-rules.rs`
- ライブラリ: `syn::parse_file` → docs.rs/syn/2/syn/fn.parse_file.html
- ライブラリ: `proc_macro2::Span::start().line` → docs.rs/proc-macro2/1/ （`span-locations` feature）
- ライブラリ: `Attribute::path().segments` → docs.rs/syn/2/syn/struct.Attribute.html

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 全ファイルに計装あり → 成功メッセージ + exit 0 | 正常系 | ユニット |
| 2 | ハンドラに計装漏れ → エラーメッセージ + exit 1 | 準正常系 | ユニット |
| 3 | リポジトリ impl に計装漏れ → エラーメッセージ + exit 1 | 準正常系 | ユニット |
| 4 | trait 定義の async fn は無視される | 正常系 | ユニット |
| 5 | health_check は除外される | 正常系 | ユニット |

### テストリスト

ユニットテスト:

`has_instrument_attr`:
- [ ] `tracing::instrument属性がある場合にtrueを返す`
- [ ] `instrument属性の短縮形でもtrueを返す`
- [ ] `他の属性のみの場合にfalseを返す`
- [ ] `属性なしの場合にfalseを返す`

`is_excluded`:
- [ ] `health_checkは除外される`
- [ ] `通常の関数名は除外されない`

`check_handler_file`:
- [ ] `pub_async_fnに計装ありでエラーなし`
- [ ] `pub_async_fnに計装なしでエラーあり`
- [ ] `health_checkは計装なしでもエラーなし`
- [ ] `非pubのasync_fnはチェック対象外`
- [ ] `pubだが非asyncのfnはチェック対象外`
- [ ] `複数の関数で漏れのある関数のみエラー`

`check_repository_file`:
- [ ] `impl内のasync_fnに計装ありでエラーなし`
- [ ] `impl内のasync_fnに計装なしでエラーあり`
- [ ] `trait定義のasync_fnはチェック対象外`
- [ ] `impl内の非async_fnはチェック対象外`
- [ ] `traitとimplが混在する場合にimplのみチェック`
- [ ] `除外関数はimplメソッドでもスキップ`

出力フォーマット:
- [ ] `ハンドラのエラーメッセージフォーマットが正しい`
- [ ] `リポジトリのエラーメッセージフォーマットが正しい`

ハンドラテスト: 該当なし
API テスト: 該当なし
E2E テスト: 該当なし

## 検証

1. `rust-script --test ./scripts/check/instrumentation.rs` → 全ユニットテスト通過
2. `rust-script ./scripts/check/instrumentation.rs` → 現在のコードベースで exit 0（シェルスクリプトと同じ結果）
3. `just check-instrumentation` → justfile 経由で動作確認
4. `just check-all` → 全体テスト通過

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `globset` の `**` 解釈が `git ls-files` と異なる | 既存手段の見落とし | ファイル検出は `git ls-files` via `Command` に決定 |
| 2回目 | CI の `code-quality` ジョブに Rust ツールチェインがない | 技術的前提 | `claude-rules-check.yaml` のパターンで Rust + rust-script セットアップ追加 |
| 3回目 | `proc-macro2` の `span-locations` を有効にしないと行番号が取れない | 未定義 | 依存に `proc-macro2 = { features = ["span-locations"] }` を追加 |
| 4回目 | trait impl のメソッドには `pub` がない | 不完全なパス | `check_repository_file` では `pub` チェックを行わない設計に |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | instrumentation.rs 新規、instrumentation.sh 削除、justfile、ci.yaml、ADR-056、ナレッジベース、observability.md — 全て記載。parallel.sh は変更不要を確認 |
| 2 | 曖昧さ排除 | OK | 全関数シグネチャとロジック詳細を記載。syn API の具体的なアクセスパスを確認 |
| 3 | 設計判断の完結性 | OK | ファイル検出方式、行番号取得、属性検出、trait/impl 区別、CI セットアップ — 6 件の判断を記載 |
| 4 | スコープ境界 | OK | 対象 7 ファイル、対象外（parallel.sh、他スクリプト移行）を明示 |
| 5 | 技術的前提 | OK | git ls-files セマンティクス、span-locations feature、CI ジョブ構成を調査済み |
| 6 | 既存ドキュメント整合 | OK | ADR-015（移行基準該当）、ADR-056（スコープ拡大で整合）を確認 |
