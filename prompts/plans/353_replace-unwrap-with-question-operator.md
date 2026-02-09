# #353 Replace unwrap() with ? operator in module-level doctests

## Context

PR #349 のレビューで、モジュールレベル doctest での `unwrap()` 使用が指摘された。doctest は「使い方のお手本」であり、プロダクションコードと同じ品質基準でエラーハンドリングすべき。加えて、doctest での `?` 演算子使用方針がルールファイルに明文化されていないため、ルール更新もあわせて行う。

## 対象

### 対象

| ファイル | 箇所数 | unwrap() の対象 |
|---------|-------|----------------|
| `backend/crates/domain/src/user.rs` | 3 | `DisplayNumber::new`, `Email::new`, `UserName::new` |
| `backend/crates/domain/src/workflow.rs` | 1 | `WorkflowName::new` |
| `backend/crates/domain/src/tenant.rs` | 1 | `Uuid::parse_str` |
| `backend/crates/domain/src/value_objects.rs` | 2 | `DisplayNumber::new`（2箇所） |

### 対象外

- `#[cfg(test)]` 内のテストコード（テストでの `unwrap()` は許容）
- `.claude/rules/rust.md` 内のテストコード例（行 75: `Email::new("user@example.com").unwrap()`）— テストの例なので `unwrap()` は適切

## 設計判断

### エラー型: `Box<dyn std::error::Error>`

doctest の `fn main()` 戻り値に `Box<dyn std::error::Error>` を使用する。

選択肢:
1. `Box<dyn std::error::Error>` — 異なるエラー型が混在しても対応可能。Rust 公式ドキュメントの標準パターン
2. `DomainError` — 型が明確だが、`tenant.rs` の `uuid::Error` に対応できない
3. `anyhow::Error` — 外部クレート依存。doctest の標準パターンから外れる

判断: 1 を採用。Rust の doctest で最も一般的で、`DomainError`（user.rs 等）と `uuid::Error`（tenant.rs）の両方に対応できる。

### 隠し行（`#` プレフィックス）

`fn main() -> Result<...>` のラッパーは `#` で隠す。doctest の本質は使い方の例示であり、ボイラープレートは非表示にする。

```rust
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let num = DisplayNumber::new(42)?;
//! # Ok(())
//! # }
//! ```
```

## Phase 1: ルールファイル更新（`.claude/rules/rust.md`）

### 確認事項: なし（既知のパターンのみ）

### 変更内容

行 364-388 のドキュメントコメントセクションに、doctest でのエラーハンドリング方針を追加する。

追加する内容:
- doctest は「使い方のお手本」→ `?` 演算子を使う
- `fn main() -> Result<...>` ラッパーパターンの説明
- `#[cfg(test)]` 内の `unwrap()` は許容する旨の明記

具体的な追加位置: 行 370（使用例の項目）の後、コード例の前に方針を追記し、コード例に `# fn main()` ラッパーを反映する。

## Phase 2: doctest の修正（7箇所）

### 確認事項: なし（既知のパターンのみ）

各 doctest に `# fn main() -> Result<(), Box<dyn std::error::Error>>` ラッパーを追加し、`.unwrap()` を `?` に置換する。

### 2-1: `user.rs`（行 20-39）

変更前:
```rust
//! let user = User::new(
//!    UserId::new(),
//!    TenantId::new(),
//!    DisplayNumber::new(1).unwrap(),
//!    Email::new("user@example.com").unwrap(),
//!    UserName::new("山田太郎").unwrap(),
//!    chrono::Utc::now(),
//! );
```

変更後:
```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let user = User::new(
//!    UserId::new(),
//!    TenantId::new(),
//!    DisplayNumber::new(1)?,
//!    Email::new("user@example.com")?,
//!    UserName::new("山田太郎")?,
//!    chrono::Utc::now(),
//! );
//!
//! // ステータス確認
//! assert!(user.is_active());
//! # Ok(())
//! # }
```

### 2-2: `workflow.rs`（行 13-32）

変更: `WorkflowName::new("汎用申請").unwrap()` → `WorkflowName::new("汎用申請")?` + ラッパー追加

### 2-3: `tenant.rs`（行 31-44）

変更: `Uuid::parse_str("...").unwrap()` → `Uuid::parse_str("...")?` + ラッパー追加

### 2-4: `value_objects.rs` DisplayNumber（行 153-160）

変更: `DisplayNumber::new(42).unwrap()` → `DisplayNumber::new(42)?` + ラッパー追加

### 2-5: `value_objects.rs` DisplayId（行 282-287）

変更: `DisplayNumber::new(42).unwrap()` → `DisplayNumber::new(42)?` + ラッパー追加

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `rust.md` 行 374-388 の例で `?` を使っているのに `fn main() -> Result` ラッパーがない。ルールの例自体が不正確 | 不完全なパス | Phase 1 でルールのコード例も `fn main()` ラッパーを含めるよう修正する |
| 2回目 | `rust.md` 行 75 のテストコード例に `unwrap()` がある。これは `#[cfg(test)]` 内の例なので適切だが、doctest 方針追記時に「テストでは `unwrap()` 許容」を明記すべき | 曖昧 | Phase 1 のルール追記にテストコードの `unwrap()` 許容を明記 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全 doctest 内の unwrap() が計画に含まれている | OK | Explore agent で domain src 全体を探索済み。7箇所すべて計画に含む |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | エラー型、隠し行の扱い、テストコードの除外がすべて明示的 |
| 3 | 設計判断の完結性 | エラー型の選択に判断が記載されている | OK | `Box<dyn Error>` の選定理由と代替案を記載 |
| 4 | スコープ境界 | 対象と対象外が明記されている | OK | テストコードと rust.md テスト例を対象外として明記 |
| 5 | 技術的前提 | doctest の `#` 隠し行と `fn main()` ラッパーが正しい | OK | Rust 公式ドキュメントの標準パターン |
| 6 | 既存ドキュメント整合 | CLAUDE.md の型安全性方針と矛盾なし | OK | 「安易な unwrap を避け、エラーを適切に型で扱う」に合致 |

## 検証方法

```bash
just check  # doctest を含む全テスト + lint
```
