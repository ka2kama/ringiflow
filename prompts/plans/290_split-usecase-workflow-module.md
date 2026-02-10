# 計画: usecase/workflow.rs の Query/Command 分割

## Context

Issue #290「Refactor oversized files (500+ lines)」の2番目の対応として、`backend/apps/core-service/src/usecase/workflow.rs`（1938行）を分割する。

プロダクションコードは約792行（500行閾値超過）、テストコードは約1145行。14個の pub メソッドが単一ファイルに同居している。前回の domain/workflow.rs 分割（PR #363）で確立したパターンを応用するが、ユースケース層は単一構造体の impl ブロック分割という異なるアプローチが必要。

## 対象・対象外

- 対象: `usecase/workflow.rs` の分割
- 対象外: 他の 500 行超過ファイル（別 PR）、ADR の追加作成（ADR-039 の方針を踏襲）、消費者側のインポート変更（不要）

## 設計判断

### 分割軸: Query/Command（CQRS ライク）

選択肢:
1. Query/Command 分割 — メソッドの責務（読み取り vs 状態変更）で分ける
2. エンティティ分割 — 操作対象エンティティ（Definition, Instance, Step）で分ける

選択: Query/Command 分割

理由:
- ユースケースの操作はエンティティ境界を横断する（`approve_step` は Step と Instance の両方を更新）
- 既にコード内に `// ===== GET 系メソッド =====` `// ===== 承認/却下系メソッド =====` のセクション分けがあり、自然な分割線
- エンティティ分割だと approve/reject が Step と Instance の両方に属し、配置が曖昧になる

### display_number メソッドの配置

display_number バリアントは ID 解決 → コアメソッド委譲の薄いアダプタ。コアメソッドと同じモジュールに配置する:
- `get_workflow_by_display_number` → query.rs（読み取り）
- `submit/approve/reject_by_display_number` → command.rs（状態変更）

### モジュール構造

```
usecase/
├── workflow.rs          # 親モジュール: doc + 型定義 + struct + new() (~130 行)
└── workflow/
    ├── query.rs         # 読み取りメソッド 5 個 (~155 行 prod, テストなし)
    └── command.rs       # 状態変更メソッド 7 個 (~530 行 prod, ~785 行 tests)
```

command.rs のプロダクションコード (~530 行) は閾値をやや超過するが、7メソッドはすべて状態遷移という同一責務であり、さらに分割すると人工的な境界になるため許容する。

### 型定義・構造体の配置: 親モジュールに残す

型定義（`WorkflowWithSteps`, `CreateWorkflowInput`, `SubmitWorkflowInput`, `ApproveRejectInput`）、ヘルパー関数（`collect_user_ids_from_workflow`）、構造体定義（`WorkflowUseCaseImpl` + `new()` + `resolve_user_names()`）はすべて親モジュールに残す。

理由:
- 型はユースケース全体の公開 API であり、特定のサブモジュールに属さない
- `collect_user_ids_from_workflow` は handler から `crate::usecase::workflow::collect_user_ids_from_workflow` で参照されており、パスを維持する必要がある

### Re-export 戦略: 不要

domain 分割と異なり、`pub use *` は不要。子モジュールは新しい型を公開せず、親の構造体に impl ブロックを追加するのみ。メソッドは構造体を通じてアクセスされるため、モジュールの可視性は影響しない。

```rust
// workflow.rs
mod command;
mod query;
// pub use は不要
```

### Rust の可視性: 子モジュールから親の private フィールドへのアクセス

`command.rs` と `query.rs` は親モジュール（`workflow.rs`）で定義された `WorkflowUseCaseImpl` の private フィールド（`definition_repo`, `instance_repo` 等）にアクセスする必要がある。Rust の可視性ルールにより、子モジュールは親モジュールの private アイテムにアクセス可能。

### テスト配置: command.rs に集約

全 11 テストケースと 5 個の Mock リポジトリ（計 ~1122 行）を command.rs に配置する。

理由:
- 全テストが command メソッド（create, submit, approve, reject）を対象としている
- query メソッドのテストは現時点で存在しない
- Mock リポジトリは command テスト内でのみ使用されている

## 実装計画

### Phase 1: query.rs の切り出し

確認事項:
- 型: `WorkflowUseCaseImpl` のフィールドと可視性 → `usecase/workflow.rs:94-101`
- パターン: domain 分割の sibling import → `domain/src/workflow/instance.rs` の `use super::`

手順:
1. `usecase/workflow/` ディレクトリを作成
2. `usecase/workflow/query.rs` を作成
   - `use super::WorkflowUseCaseImpl;` + 必要な domain 型
   - 5 メソッドの impl ブロックを移動
3. `workflow.rs` に `mod query;` を追加
4. `workflow.rs` から query メソッド（498-613行）と `get_workflow_by_display_number`（615-653行）を削除
5. `cargo test --package ringiflow-core-service` で全テスト通過を確認

テストリスト:
- [ ] 既存の 11 テストが通る（query テストはないが回帰確認）
- [ ] `cargo check --package ringiflow-core-service` で warning なし

### Phase 2: command.rs の切り出し + テスト移動

確認事項: なし（Phase 1 で確認済みのパターンを踏襲）

手順:
1. `usecase/workflow/command.rs` を作成
   - `use super::` で親の型をインポート
   - 7 メソッドの impl ブロックを移動（create, submit, approve, reject + 3 display_number variants）
   - `#[cfg(test)] mod tests { ... }` を移動（Mock リポジトリ 5 個 + テスト 11 個）
2. `workflow.rs` に `mod command;` を追加
3. `workflow.rs` から command メソッド（131-496行）、display_number command メソッド（655-791行）、テストコード（794-1938行）を削除
4. `workflow.rs` の不要なインポートを整理
5. `just check-all` で全テスト通過を確認

テストリスト:
- [ ] 既存の 11 テストが通る
- [ ] `just check-all` が通る
- [ ] `just check-file-size` で workflow.rs が閾値超過リストから消えている

## 分割後の行数見積もり

| ファイル | プロダクション行数 | テスト行数 |
|---------|-------------------|-----------|
| workflow.rs（親） | ~130 行 | なし |
| workflow/query.rs | ~155 行 | なし |
| workflow/command.rs | ~530 行 | ~1122 行 |

## 検証方法

```bash
just check-all          # lint + test + API test
just check-file-size    # 閾値確認（workflow.rs が消えていること）
```

## 主要ファイル

| ファイル | 役割 |
|---------|------|
| `backend/apps/core-service/src/usecase/workflow.rs` | 分割元（1938行 → ~130行の親モジュール） |
| `backend/apps/core-service/src/usecase.rs` | 親モジュール（`pub mod workflow;` + re-export、変更なし） |
| `backend/apps/core-service/src/handler/workflow.rs` | 消費者（`collect_user_ids_from_workflow` の参照パス維持を確認） |
| `backend/crates/domain/src/workflow.rs` | 前回の分割パターン参照 |

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | display_number メソッドの配置が未決定 | 曖昧 | コアメソッドと同じモジュールに配置する方針を追加 |
| 2回目 | command.rs (~530行) が閾値超過の可能性 | 技術的前提 | 全メソッドが同一責務で分割は人工的。閾値はソフトなので許容と判断 |
| 3回目 | `pub use *` が必要か未検討 | 未定義 | impl ブロック追加のみで新規型の公開なし → re-export 不要と確定 |
| 4回目 | 子モジュールから親の private フィールドへのアクセス可否 | 技術的前提 | Rust の可視性ルール確認: 子モジュールは親の private アイテムにアクセス可能 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | 14メソッド全て（query 5 + command 7 + struct 2）と全テスト（11テスト + 5 Mock）の配置先を明示 |
| 2 | 曖昧さ排除 | OK | 各メソッドの配置先、テスト配置、re-export 戦略を具体的に記載 |
| 3 | 設計判断の完結性 | OK | Query/Command vs Entity、display_number 配置、re-export 不要、テスト集約の判断を理由付きで記載 |
| 4 | スコープ境界 | OK | 対象: workflow.rs のみ。対象外: ADR追加、他ファイル、消費者側変更 |
| 5 | 技術的前提 | OK | Rust の可視性ルール（子→親 private フィールド）、impl ブロック分割の挙動を確認 |
| 6 | 既存ドキュメント整合 | OK | ADR-039（分割方針）、structural-review.md（500行閾値、テスト除外）、domain 分割計画と整合 |
