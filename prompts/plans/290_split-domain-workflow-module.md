# 計画: domain/workflow.rs のサブモジュール分割

## Context

Issue #290「Refactor oversized files (500+ lines)」の最初の対応として、`backend/crates/domain/src/workflow.rs`（1665行）を分割する。

このファイルには 3 つのドメインエンティティ（WorkflowDefinition, WorkflowInstance, WorkflowStep）が同居しており、エンティティ間の結合は ID 型の参照のみ。状態遷移ロジックは各エンティティ内で完結しているため、分割しても凝集度は下がらない。

Issue 作成時から workflow.rs は +370 行増加（1295→1665）しており、割れ窓効果で肥大化が加速している。

## 対象・対象外

- 対象: `workflow.rs` の分割、ADR-039 の作成
- 対象外: 他の 500 行超過ファイル（別 PR で対応）、消費者側のインポート変更（不要）

## 設計判断

### モジュール構造: `workflow.rs` + `workflow/` ディレクトリ

```
domain/src/
├── lib.rs               # 変更なし（pub mod workflow;）
├── workflow.rs          # 親モジュール: doc + mod 宣言 + pub use re-export（~50行）
└── workflow/
    ├── definition.rs    # WorkflowDefinition（プロダクション ~205行 + テスト ~70行）
    ├── instance.rs      # WorkflowInstance（プロダクション ~365行 + テスト ~300行）
    └── step.rs          # WorkflowStep（プロダクション ~367行 + テスト ~240行）
```

- `mod.rs` は使わない（`.claude/rules/rust.md` のルール）
- `handler.rs` + `handler/` と同じ Rust 2018+ パターン

### Re-export 戦略: `mod`（非pub）+ `pub use *`

```rust
// workflow.rs（親モジュール）
mod definition;
mod instance;
mod step;

pub use definition::*;
pub use instance::*;
pub use step::*;
```

選択理由:
- 消費者のインポートパス `workflow::TypeName` が変わらない（API 互換性維持）
- サブモジュールの内部構造を隠蔽（`workflow::definition::TypeName` は外部に露出しない）
- handler.rs は `pub mod` + 明示的 `pub use` だが、あちらはサブモジュールが独立機能。workflow は単一概念の内部分割のため `mod` + glob re-export が適切

### サブモジュール間の型参照

```
definition.rs ←(WorkflowDefinitionId)─ instance.rs ←(WorkflowInstanceId)─ step.rs
```

- `instance.rs`: `use super::definition::WorkflowDefinitionId;`
- `step.rs`: `use super::instance::WorkflowInstanceId;`
- Rust の可視性ルールにより、sibling の private module の public item には `super::` 経由でアクセス可能

### テスト配置: 各サブモジュール内

各 `.rs` に `#[cfg(test)] mod tests` を配置。共有フィクスチャ `now()` は各テストモジュールで重複定義する（1行定義のため共有モジュール化のオーバーヘッドが大きい）。

## 実装計画

### Phase 1: ADR-039 作成

`docs/70_ADR/039_ワークフローモジュールの分割方針.md` を作成。

確認事項: なし（ADR-038 のフォーマットを踏襲）

テストリスト: なし（ドキュメントのみ）

### Phase 2: definition.rs の切り出し

確認事項:
- パターン: `handler.rs` の re-export パターン → `backend/apps/core-service/src/handler.rs`

手順:
1. `workflow/` ディレクトリを作成
2. `workflow/definition.rs` を作成（型 + impl + テストを移動）
3. `workflow.rs` に `mod definition; pub use definition::*;` を追加
4. `workflow.rs` から Definition セクション（51-255行）とテスト `mod workflow_definition`（1592-1664行）を削除

テストリスト:
- [ ] 既存の WorkflowDefinition 4 テストが通る
- [ ] 他エンティティのテストに回帰がない

### Phase 3: instance.rs の切り出し

確認事項:
- 型: `WorkflowDefinitionId` の参照 → `super::definition::WorkflowDefinitionId`

手順:
1. `workflow/instance.rs` を作成（型 + impl + テストを移動）
2. `workflow.rs` に `mod instance; pub use instance::*;` を追加
3. `workflow.rs` から Instance セクション（257-621行）とテスト `test_instance` フィクスチャ + `mod workflow_instance`（1008-1341行）を削除

テストリスト:
- [ ] 既存の WorkflowInstance 20 テストが通る
- [ ] 他エンティティのテストに回帰がない

### Phase 4: step.rs の切り出し + 最終整理

確認事項:
- 型: `WorkflowInstanceId` の参照 → `super::instance::WorkflowInstanceId`

手順:
1. `workflow/step.rs` を作成（型 + impl + テストを移動）
2. `workflow.rs` に `mod step; pub use step::*;` を追加
3. `workflow.rs` から Step セクション + 残りのテスト全体を削除
4. `workflow.rs` を最終形に整理（doc + mod 宣言 + re-export のみ、不要なインポート削除）
5. `just check-all` で全テスト通過を確認

テストリスト:
- [ ] 既存の WorkflowStep 12 テストが通る
- [ ] `just check-all` が通る
- [ ] `just check-file-size` で新たな閾値超過がない
- [ ] doctest が通る

## 検証方法

```bash
just check-all          # lint + test + API test
just check-file-size    # 閾値確認
```

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | サブモジュール間の型参照パスが未定義 | 未定義 | `super::definition::WorkflowDefinitionId` 等を明示 |
| 2回目 | `mod`（非pub）vs `pub mod` の判断が曖昧 | 曖昧 | handler.rs との違いを分析し、`mod` + `pub use *` に決定 |
| 3回目 | テスト込み行数が閾値超過の可能性 | 技術的前提 | structural-review.md はテスト除外カウント。プロダクションのみなら各ファイル 205-367行で閾値以内 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | Definition(51-255), Instance(257-621), Step(623-989), Tests(995-1665), Doc(1-49) を全て配分済み |
| 2 | 曖昧さ排除 | OK | 各ファイルのインポート、re-export 方法、テスト配置を具体的に記載 |
| 3 | 設計判断の完結性 | OK | mod vs pub mod、glob vs 明示的 re-export、テスト配置の判断を理由付きで記載 |
| 4 | スコープ境界 | OK | 対象: workflow.rs のみ。対象外: 消費者側変更、他ファイル |
| 5 | 技術的前提 | OK | Rust 2018+ モジュール解決、sibling module の可視性ルール確認済み |
| 6 | 既存ドキュメント整合 | OK | structural-review.md, rust.md, handler.rs パターンと整合 |

## 主要ファイル

| ファイル | 役割 |
|---------|------|
| `backend/crates/domain/src/workflow.rs` | 分割元（1665行 → ~50行の親モジュール） |
| `backend/crates/domain/src/lib.rs` | 変更なし（`pub mod workflow;`） |
| `backend/apps/core-service/src/handler.rs` | 参照パターン（`handler.rs` + `handler/`） |
| `docs/70_ADR/038_未使用依存検出ツールの選定.md` | ADR フォーマット参照 |
