# doctest の unwrap() を ? 演算子に置換

## 概要

モジュールレベルおよび型レベルの doctest で使用されていた `unwrap()` を `?` 演算子に置換し、doctest でのエラーハンドリング方針を `.claude/rules/rust.md` に明文化した。

## 背景と目的

PR #349 のレビューで、doctest 内の `unwrap()` 使用が指摘された。doctest はドキュメントの一部であり「使い方のお手本」として機能するため、プロダクションコードと同じ品質基準でエラーハンドリングすべきである。

## 実施内容

1. `.claude/rules/rust.md` のドキュメントコメントセクションに doctest エラーハンドリング方針を追加
   - `?` 演算子 + `fn main() -> Result<(), Box<dyn std::error::Error>>` ラッパーパターン
   - doctest vs テストコードでの `unwrap()` 使い分け表
2. domain クレートの doctest 7 箇所で `unwrap()` → `?` に置換
   - `user.rs`（3箇所）、`workflow.rs`（1箇所）、`tenant.rs`（1箇所）、`value_objects.rs`（2箇所）

## 設計上の判断

### エラー型の選択: `Box<dyn std::error::Error>`

| 選択肢 | 判断 | 理由 |
|--------|------|------|
| `Box<dyn std::error::Error>` | 採用 | Rust 標準の doctest パターン。異なるエラー型（`DomainError`, `uuid::Error`）が混在しても対応可能 |
| `DomainError` | 不採用 | `tenant.rs` の `uuid::Error` に対応できない |
| `anyhow::Error` | 不採用 | 外部クレート依存。doctest の標準パターンから外れる |

## 判断ログ

特筆すべき判断なし。

## 成果物

- コミット: `#353 Replace unwrap() with ? operator in doctests`
- PR: #354（Draft）
- 変更ファイル:
  - `.claude/rules/rust.md` — doctest ポリシー追加
  - `backend/crates/domain/src/user.rs` — doctest 修正
  - `backend/crates/domain/src/workflow.rs` — doctest 修正
  - `backend/crates/domain/src/tenant.rs` — doctest 修正
  - `backend/crates/domain/src/value_objects.rs` — doctest 修正

## 議論の経緯

### doctest 方針の明文化

ユーザーから、doctest のコード修正だけでなく方針自体をルールとして明記する必要があるとの指摘があった。これを受け、`rust.md` にポリシーを追加するスコープに拡大した。

## 学んだこと

- Rust の doctest で `?` 演算子を使うには `# fn main() -> Result<(), Box<dyn std::error::Error>>` ラッパーが必要
- `#` プレフィックスで doctest のボイラープレートを非表示にできる

## 次のステップ

- PR レビュー・マージ
