# 計画: #537 Core Service ユースケース層の共通ヘルパー関数抽出とファイル分割

## Context

Issue #525 Phase 1（テストビルダーパターン導入）完了後の継続タスク。ユースケース層全体に散在する重複パターン（エラーハンドリング、権限チェック）を共通ヘルパーに抽出し、大ファイルを分割して保守性を向上させる。

## 対象

- Phase 2（Issue の定義）: 共通ヘルパー関数の抽出
- Phase 3（Issue の定義）: 大ファイルの分割（decision.rs, lifecycle.rs, task.rs）

## 対象外

- テストビルダーパターン（#525 Phase 1 で完了済み）
- 新規機能の追加
- エラーメッセージ体系の再設計（将来の改善タスクとして検討可能）
- `Result<T, InfraError>` → `Result<T, CoreError>` の `map_err` のみの箇所（`Option` を含まないパターン）

## 設計判断

### 拡張トレイト vs 自由関数

**選択: 拡張トレイト（`FindResultExt`）**

`Result<Option<T>, InfraError>` → `Result<T, CoreError>` の変換に拡張トレイトを使用する。

```rust
// Before (3行)
let step = self.step_repo.find_by_id(&step_id, &tenant_id).await
    .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?
    .ok_or_else(|| CoreError::NotFound("ステップが見つかりません".to_string()))?;

// After (1行)
let step = self.step_repo.find_by_id(&step_id, &tenant_id).await
    .or_not_found("ステップ")?;
```

理由:
- メソッドチェーンが自然に読める（Rust の `ResultExt` パターンに準拠）
- `anyhow::Context` や `color_eyre::WrapErr` と同じ設計手法
- 自由関数（`find_or_not_found(repo.find(...).await, "ステップ")`）より呼び出し側が簡潔

トレードオフ:
- `use crate::usecase::helpers::FindResultExt;` のインポートが各ファイルで必要
- `or_not_found` がカバーしない特殊ケース（`Internal` で NotFound を返す `get_task` の内部整合性チェック）は手動のまま

### エラーメッセージの統一

`or_not_found("ステップ")` で以下を生成:
- Internal: `"ステップの取得に失敗: {原因}"`
- NotFound: `"ステップが見つかりません"`

現在のコードではメッセージの揺れがある（"取得に失敗" vs "取得エラー"）。ヘルパー導入により自然に統一される。
ただし、既存のメッセージと完全一致しない箇所がある（例: task.rs の "タスクが見つかりません"）。これらは `or_not_found("タスク")` でカバーでき、内部エラーメッセージは "タスクの取得に失敗" に変わるが、Internal エラーはログ用であり外部影響はない。

### ファイル分割のモジュール構造

Rust 2018+ のファイルベース命名規則（プロジェクト既存パターン: `workflow.rs` + `workflow/`）に従う。

```
# Before
command/decision.rs (1882行)
command/lifecycle.rs (1289行)

# After
command/decision.rs        (mod宣言 + re-export)
command/decision/approve.rs
command/decision/reject.rs
command/decision/request_changes.rs

command/lifecycle.rs       (mod宣言 + re-export)
command/lifecycle/create.rs
command/lifecycle/submit.rs
command/lifecycle/resubmit.rs
```

テストヘルパーは `command.rs` に残し、分割後のサブモジュールから `super::super::super::test_helpers` で参照する。

### task.rs の方針

task.rs（796行）は production code が ~225行、test code が ~570行。Phase 2 ヘルパー適用後も 500行超が残るが、production code は十分小さい。

方針: ヘルパー適用のみ行い、ファイル分割はしない。理由:
- production code は分割不要な規模（~210行）
- 超過分は test code であり、テストを別ファイルに分ける利点が少ない
- Issue の目標「各150行以下」は decision/lifecycle の分割後サブモジュール目標であり、task.rs に同基準を適用すると不自然に細分化する

---

## 実装計画

TDD（Red → Green → Refactor）で MVP を積み上げる。

### Phase 1: ヘルパーモジュール作成

新規ファイル `backend/apps/core-service/src/usecase/helpers.rs` を作成する。

#### 確認事項
- [x] 型: `InfraError` のインポートパス → `ringiflow_infra::InfraError`（error.rs L34 で `#[from]` 使用確認済み）
- [x] 型: `CoreError` の定義 → `error.rs` L14-39, 6バリアント (NotFound/BadRequest/Forbidden/Conflict/Database/Internal)
- [x] 型: `WorkflowStep::assigned_to()` → `domain/src/workflow/step.rs` L242, `pub fn assigned_to(&self) -> Option<&UserId>`
- [x] パターン: 既存の拡張トレイトパターン → Grep 結果 0件。プロジェクト内に前例なし、新規導入
- [x] パターン: `pub(crate)` の使用箇所 → `workflow.rs` L91 `pub(crate) fn collect_user_ids_from_workflow` で使用確認済み

#### テストリスト

ユニットテスト:
- [ ] `or_not_found`: `Ok(Some(value))` → `Ok(value)` を返す
- [ ] `or_not_found`: `Ok(None)` → `Err(CoreError::NotFound)` を返す（エラーメッセージにエンティティ名を含む）
- [ ] `or_not_found`: `Err(InfraError)` → `Err(CoreError::Internal)` を返す（エラーメッセージにエンティティ名と原因を含む）
- [ ] `check_step_assigned_to`: 担当者一致 → `Ok(())`
- [ ] `check_step_assigned_to`: 担当者不一致 → `Err(CoreError::Forbidden)`
- [ ] `check_step_assigned_to`: `assigned_to` が `None` → `Err(CoreError::Forbidden)`

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

#### 実装内容

```rust
// usecase/helpers.rs
use ringiflow_domain::{user::UserId, workflow::WorkflowStep};
use ringiflow_infra::InfraError;
use crate::error::CoreError;

/// リポジトリの `Result<Option<T>, InfraError>` を `Result<T, CoreError>` に変換する拡張トレイト
///
/// `find_by_id` 等の `Option` を返すリポジトリメソッドの結果を、
/// `CoreError::NotFound` または `CoreError::Internal` に変換する。
///
/// → ナレッジベース: [FindResultExt パターン](../../docs/06_ナレッジベース/backend/FindResultExt.md)
pub(crate) trait FindResultExt<T> {
    fn or_not_found(self, entity_name: &str) -> Result<T, CoreError>;
}

impl<T> FindResultExt<T> for Result<Option<T>, InfraError> {
    fn or_not_found(self, entity_name: &str) -> Result<T, CoreError> {
        self.map_err(|e| CoreError::Internal(format!("{}の取得に失敗: {}", entity_name, e)))?
            .ok_or_else(|| CoreError::NotFound(format!("{}が見つかりません", entity_name)))
    }
}

/// ステップの担当者をチェックする
pub(crate) fn check_step_assigned_to(
    step: &WorkflowStep,
    user_id: &UserId,
    action: &str,
) -> Result<(), CoreError> {
    if step.assigned_to() != Some(user_id) {
        return Err(CoreError::Forbidden(
            format!("このステップを{}する権限がありません", action),
        ));
    }
    Ok(())
}
```

### Phase 2: ヘルパー適用（既存コードのリファクタリング）

既存の全ユースケースファイルに `FindResultExt::or_not_found` と `check_step_assigned_to` を適用する。新しいテストは書かない（既存テストが回帰テストとして機能する）。

#### 確認事項
- [ ] 各ファイルの `map_err + ok_or_else` パターンの全箇所を Grep で特定
- [ ] `check_step_assigned_to` に置換可能な権限チェック箇所を特定
- [ ] `or_not_found` でカバーできない特殊ケース（`CoreError::Internal` で NotFound 相当を返す箇所）を把握

#### テストリスト

ユニットテスト（既存テストで回帰確認）:
- [ ] `cd backend && cargo test -p core-service` 全件 pass

ハンドラテスト（該当なし — ヘルパー適用はユースケース層のみ）
API テスト（該当なし）
E2E テスト（該当なし）

#### 適用対象ファイルと変換箇所

| ファイル | `or_not_found` 適用 | `check_step_assigned_to` 適用 | 特殊ケース（手動のまま） |
|---------|------|------|------|
| `decision.rs` | ~10箇所 | 3箇所（承認/却下/差し戻し） | なし |
| `lifecycle.rs` | ~7箇所 | なし（`initiated_by` チェックは対象外） | なし |
| `task.rs` | ~4箇所 | 2箇所（タスク詳細） | `get_task` L160-163: Internal で NotFound 相当 |
| `comment.rs` | ~2箇所 | なし（`is_participant` は対象外） | なし |
| `query.rs` | ~4箇所 | なし | なし |
| `dashboard.rs` | なし（`Option` を返さない） | なし | なし |

注: `dashboard.rs` は `Result<Vec<T>, InfraError>` パターンのみで、`Option` を含まないため `or_not_found` の対象外。

task.rs の `check_step_assigned_to` 適用時、エラーメッセージが変わる:
- Before: `"このタスクにアクセスする権限がありません"`
- After: `check_step_assigned_to(&step, &user_id, "アクセス")` → `"このステップをアクセスする権限がありません"`

これは不自然。task.rs の権限チェックは「タスクへのアクセス」であり「ステップの操作」ではない。task.rs のチェックは手動のままにする。

**修正: task.rs の `check_step_assigned_to` は適用しない**（エラーメッセージのドメインが異なるため）。

### Phase 3: decision.rs の分割

`decision.rs`（1882行）を機能別に3ファイルに分割する。

#### 確認事項
- [ ] Rust のモジュール分割時の re-export パターン → `workflow.rs` + `workflow/` の既存パターンを参照
- [ ] テストヘルパー（`command.rs::test_helpers`）の可視性 → 分割後サブモジュールから `super::super::super::test_helpers` でアクセス可能か
- [ ] `decision.rs` 内の `impl WorkflowUseCaseImpl` ブロックが分割可能か（Rust は複数ファイルに impl ブロックを分散可能）

#### テストリスト

ユニットテスト（既存テストの移動確認）:
- [ ] approve 関連テスト全件 pass（approve.rs に移動）
- [ ] reject 関連テスト全件 pass（reject.rs に移動）
- [ ] request_changes 関連テスト全件 pass（request_changes.rs に移動）
- [ ] display_number 対応メソッドのテスト全件 pass

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

#### ファイル構造

```
command/decision.rs (新: mod宣言のみ, ~10行)
  mod approve;
  mod reject;
  mod request_changes;

command/decision/approve.rs (~450行)
  - approve_step()
  - approve_step_by_display_number()
  - #[cfg(test)] mod tests (approve 関連テスト)

command/decision/reject.rs (~400行)
  - reject_step()
  - reject_step_by_display_number()
  - #[cfg(test)] mod tests (reject 関連テスト)

command/decision/request_changes.rs (~350行)
  - request_changes_step()
  - request_changes_step_by_display_number()
  - #[cfg(test)] mod tests (request_changes 関連テスト)
```

各サブモジュールの import 構造:

```rust
// decision/approve.rs
use ringiflow_domain::{...};
use ringiflow_infra::InfraError;
use crate::error::CoreError;
use crate::usecase::helpers::FindResultExt;
use crate::usecase::workflow::{ApproveRejectInput, WorkflowUseCaseImpl, WorkflowWithSteps};

impl WorkflowUseCaseImpl {
    pub async fn approve_step(...) -> Result<WorkflowWithSteps, CoreError> { ... }
    pub async fn approve_step_by_display_number(...) -> Result<WorkflowWithSteps, CoreError> { ... }
}

#[cfg(test)]
mod tests {
    use super::super::super::test_helpers::{...}; // command::test_helpers
    // ...
}
```

### Phase 4: lifecycle.rs の分割

`lifecycle.rs`（1289行）を機能別に3ファイルに分割する。

#### 確認事項
- [ ] Phase 3 と同じモジュール分割パターンが適用可能か確認
- [ ] `lifecycle.rs` のテストヘルパー使用状況を確認

#### テストリスト

ユニットテスト（既存テストの移動確認）:
- [ ] create 関連テスト全件 pass（create.rs に移動）
- [ ] submit 関連テスト全件 pass（submit.rs に移動）
- [ ] resubmit 関連テスト全件 pass（resubmit.rs に移動）

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

#### ファイル構造

```
command/lifecycle.rs (新: mod宣言のみ, ~10行)
  mod create;
  mod submit;
  mod resubmit;

command/lifecycle/create.rs (~200行)
  - create_workflow()
  - #[cfg(test)] mod tests

command/lifecycle/submit.rs (~350行)
  - submit_workflow()
  - submit_workflow_by_display_number()
  - #[cfg(test)] mod tests

command/lifecycle/resubmit.rs (~400行)
  - resubmit_workflow()
  - resubmit_workflow_by_display_number()
  - #[cfg(test)] mod tests
```

### Phase 5: task.rs のヘルパー適用確認と行数測定

Phase 2 でヘルパーを適用済みの task.rs の最終行数を測定し、Issue の完了条件との整合を確認する。

#### 確認事項
- [ ] Phase 2 適用後の task.rs の行数を `wc -l` で計測
- [ ] production code と test code の行数比率を確認

#### テストリスト

ユニットテスト（全体回帰確認）:
- [ ] `just check-all` pass

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | task.rs の `check_step_assigned_to` はドメインが異なる（「タスク」vs「ステップ」） | 競合・エッジケース | task.rs では手動チェックを維持する方針に変更 |
| 2回目 | dashboard.rs は `Result<Vec<T>, InfraError>` のみで `Option` を含まず `or_not_found` 対象外 | 未定義 | 適用対象から dashboard.rs を除外、対象外の理由を明記 |
| 3回目 | `get_task` L160-163 の Internal + NotFound パターンは `or_not_found` でカバー不可 | 競合・エッジケース | 特殊ケースとして手動のまま維持する方針を明記 |
| 4回目 | task.rs の 500行超過は test code が原因。分割は production code に対して不自然 | シンプルさ | Phase 5 で行数を測定し判断する方針に変更。ファイル分割は必須としない |
| 5回目 | ギャップなし | — | — |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 探索で発見された全対象が計画に含まれている | OK | 6ファイルの全クローンパターンを探索結果と突合。dashboard.rs が対象外の理由を明記 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase の対象ファイル・適用箇所・特殊ケースを具体的に列挙済み |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | 拡張トレイト vs 自由関数、task.rs の分割方針、エラーメッセージ統一を判断済み |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象・対象外セクションで明記。`map_err` のみの箇所を対象外に含む |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | Rust 拡張トレイトの慣例、モジュール可視性（`pub(super)` → `super::super::super`）を確認 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | Issue #537 の完了条件、#525 Phase 1 のセッションログと照合 |

## 検証方法

1. 各 Phase 完了時: `cd backend && cargo test -p core-service` で全テスト pass
2. 全 Phase 完了後: `just check-all` で全体回帰確認
3. 効果測定: `wc -l` で各ファイルの行数を計測し、500行以下を確認
4. クローン確認: jscpd でクローン率が削減されていることを確認
