# 計画: workflow/command.rs (3501行) の分割

Issue: #493

## Context

`/assess` 診断（2026-02-13）で検出。Phase 2-3 の機能追加（多段階承認・差し戻し・コメント）により `command.rs` が 3501 行に膨張した。ADR-043 の 500 行閾値（プロダクションコード対象）に対し、プロダクションコード ~1097 行で約 2 倍超過。

純粋なリファクタリング（振る舞い変更なし）として、ドメイン責務に基づく 3 分割を行う。

## 対象と対象外

**対象:**
- `backend/apps/core-service/src/usecase/workflow/command.rs` (3501 行)

**対象外:**
- Issue に記載された他の大型ファイル（Detail.elm, auth.rs, New.elm, instance.rs）は別 Story で対応

## 設計判断

### 分割単位: ドメイン責務に基づく 3 分割

| モジュール | 責務 | メソッド |
|-----------|------|---------|
| `lifecycle.rs` | ワークフローの作成・申請・再申請 | create_workflow, submit_workflow, resubmit_workflow + display_number variants |
| `decision.rs` | 承認者のステップ判断 | approve_step, reject_step, request_changes_step + display_number variants |
| `comment.rs` | コラボレーション | post_comment, is_participant |

**選択理由:**

- 3 グループは明確に異なるドメイン責務（ライフサイクル / 承認判断 / コラボレーション）
- 各ファイルのプロダクションコード: lifecycle ~363行, decision ~489行, comment ~86行（全て 500 行以内）
- 2 分割（lifecycle+decision / comment）だと lifecycle+decision が ~792 行で閾値超過
- 4 分割（approve / reject+request_changes / ...）は、3 メソッドが同一の「承認者の判断」責務であり過分割

### display_number メソッドの配置: コアメソッドと同居

各 `*_by_display_number` メソッドはコアメソッドの thin wrapper（ID 解決 → 委譲）であり、コアメソッドと変更タイミングが一致するため同居させる。

### テストヘルパーの配置: 親 command.rs の `#[cfg(test)]` モジュール

| ヘルパー | 使用グループ |
|---------|------------|
| `single_approval_definition_json()` | decision + lifecycle |
| `two_step_approval_definition_json()` | decision のみ（setup 内部） |
| `setup_two_step_approval()` | decision のみ |

`single_approval_definition_json` が複数グループで使用されるため、親モジュールに共有テストヘルパーとして配置する。

## ファイル構造

### Before

```
usecase/
├── workflow.rs              # 親: struct, types, new()
└── workflow/
    ├── command.rs           # 全コマンド操作 (3501行)
    └── query.rs             # 全クエリ操作
```

### After

```
usecase/
├── workflow.rs              # 変更なし
└── workflow/
    ├── command.rs           # 親: mod 宣言 + 共有テストヘルパー (~60行)
    ├── command/
    │   ├── lifecycle.rs     # 作成・申請・再申請 (~363 prod + ~801 test ≈ ~1164行)
    │   ├── decision.rs      # 承認・却下・差し戻し (~489 prod + ~1308 test ≈ ~1797行)
    │   └── comment.rs       # コメント (~86 prod + ~223 test ≈ ~309行)
    └── query.rs             # 変更なし
```

### 各ファイルの詳細

**`command.rs`（親）:**
```rust
//! ワークフローユースケースの状態変更操作

mod comment;
mod decision;
mod lifecycle;

#[cfg(test)]
pub(super) mod test_helpers {
    // single_approval_definition_json()
    // two_step_approval_definition_json()
    // setup_two_step_approval()
}
```

**`command/lifecycle.rs`:**
- imports: `super::super::{CreateWorkflowInput, SubmitWorkflowInput, ResubmitWorkflowInput, StepApprover, WorkflowUseCaseImpl, WorkflowWithSteps}` + domain types
- impl WorkflowUseCaseImpl:
  - `create_workflow` (L50-98)
  - `submit_workflow` (L121-233)
  - `resubmit_workflow` (L680-807)
  - `submit_workflow_by_display_number` (L827-847)
  - `resubmit_workflow_by_display_number` (L983-1004)
- テスト: create(2) + submit(4) + resubmit(5) = 11 テスト

**`command/decision.rs`:**
- imports: `super::super::{ApproveRejectInput, WorkflowUseCaseImpl, WorkflowWithSteps}` + domain types
- impl WorkflowUseCaseImpl:
  - `approve_step` (L256-397)
  - `reject_step` (L417-527)
  - `request_changes_step` (L547-657)
  - `approve_step_by_display_number` (L867-897)
  - `reject_step_by_display_number` (L917-948)
  - `request_changes_step_by_display_number` (L950-981)
- テスト: approve(6) + reject(6) + request_changes(5) = 17 テスト

**`command/comment.rs`:**
- imports: `super::super::{PostCommentInput, WorkflowUseCaseImpl}` + domain types
- impl WorkflowUseCaseImpl:
  - `post_comment` (L1027-1072)
  - `is_participant` (L1077-1096, private)
- テスト: comment(4) = 4 テスト（共有ヘルパー不使用）

### import パスの変更

現在の `command.rs` では `super::` で `workflow.rs` の型を参照。サブモジュール化後は `super::super::` に変わる。

```rust
// Before (command.rs)
use super::{WorkflowUseCaseImpl, CreateWorkflowInput, ...};

// After (command/lifecycle.rs)
use super::super::{WorkflowUseCaseImpl, CreateWorkflowInput, ...};
```

テストの import:
```rust
// Before
use super::super::{ResubmitWorkflowInput, StepApprover};
use super::*;

// After (lifecycle テスト)
use super::super::test_helpers::*;        // 共有テストヘルパー
use super::super::super::{ResubmitWorkflowInput, StepApprover};
```

## 実装フェーズ

### Phase 1: 親 command.rs の変換 + テストヘルパー分離

command.rs を親モジュールに変換し、テストヘルパーを配置。同時に 3 つの空サブモジュールファイルを作成する。

#### 確認事項
- [x] パターン: handler/workflow.rs の mod 宣言パターン → `mod command; mod query;` + `pub use`
- [x] パターン: 現在の command.rs の import パス → `super::` で workflow.rs の型を参照
- [x] パターン: テストの import → `super::super::` + `super::*`

#### テストリスト

ユニットテスト（該当なし）: この Phase はファイル構造の変換のみ

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

### Phase 2: lifecycle.rs の作成

create_workflow, submit_workflow, resubmit_workflow + display_number variants を移動。対応テストも移動。

#### 確認事項

確認事項: なし（Phase 1 で確認済みのパターンを踏襲）

#### テストリスト

ユニットテスト:
- [ ] 移動した 11 テストが全て通過すること（`cargo test --package ringiflow-core-service lifecycle`）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

### Phase 3: decision.rs の作成

approve_step, reject_step, request_changes_step + display_number variants を移動。対応テストも移動。

#### 確認事項

確認事項: なし（Phase 2 と同一パターン）

#### テストリスト

ユニットテスト:
- [ ] 移動した 17 テストが全て通過すること（`cargo test --package ringiflow-core-service decision`）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

### Phase 4: comment.rs の作成 + 最終検証

post_comment, is_participant を移動。対応テストも移動。元の command.rs のプロダクションコードが空になることを確認。

#### 確認事項

確認事項: なし（Phase 2 と同一パターン）

#### テストリスト

ユニットテスト:
- [ ] 移動した 4 テストが全て通過すること（`cargo test --package ringiflow-core-service comment`）
- [ ] 全テスト通過: `cargo test --package ringiflow-core-service`

ハンドラテスト（該当なし）

API テスト: `just check-all` で API テスト + E2E テストを含む全体検証

E2E テスト: 上記に含む

## 検証

1. 各 Phase 完了時: `cargo test --package ringiflow-core-service` でユニットテスト通過
2. 全 Phase 完了後: `just check-all`（lint + test + API test + E2E test）
3. ファイルサイズ確認: 各サブモジュールのプロダクションコードが 500 行以内

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | テストヘルパーの共有方法が未定義。`single_approval_definition_json` が lifecycle と decision の両方で使用される | 未定義 | 親 command.rs の `#[cfg(test)] pub(super) mod test_helpers` に共有ヘルパーを配置する方式を採用 |
| 2回目 | import パスの変更（`super::` → `super::super::`）が明示されていない | 曖昧 | 各ファイルの import パス変更を具体的に記載 |
| 3回目 | テストの import パスが 3 段階（`super::super::super::`）になり冗長 | 品質の向上 | テスト内で `use crate::usecase::workflow::*` に変更することで簡潔にできるか検討 → Rust の `#[cfg(test)]` モジュールでは `crate::` パスが使えるため、テスト内は `crate::` パスで統一する方が可読性が高い |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | command.rs の全メソッド（12 public + 1 private）が割り当てられている | OK | lifecycle(5) + decision(6) + comment(2) = 13 メソッド = 全メソッド |
| 2 | 曖昧さ排除 | 各メソッドの移動先が一意に確定している | OK | メソッド名と行番号で明示 |
| 3 | 設計判断の完結性 | 分割粒度、display_number 配置、テストヘルパー配置が決定済み | OK | 3 つの設計判断を理由付きで記載 |
| 4 | スコープ境界 | 対象（command.rs）と対象外（他の大型ファイル）が明記 | OK | 「対象と対象外」セクションで明記 |
| 5 | 技術的前提 | Rust のモジュールシステムの制約が考慮されている | OK | `workflow.rs` + `workflow/` パターンの既存実績を確認。`#[cfg(test)]` の可視性も確認 |
| 6 | 既存ドキュメント整合 | ADR-043 の分割戦略と整合している | OK | 「責務分析 → 責務に基づく分割」パターンに該当。閾値判定もプロダクションコードのみで計算 |
