# #902 workflow command usecase のコード重複を解消する

## Context

`workflow/command/` 配下の 6 ファイル（合計 3,958 行）に大量のコード重複が存在する。jscpd で約 200 clones が検出されており、保守コストの増大と修正漏れのリスクが問題。

完了基準:
- jscpd clone 数が現状から 50% 以上削減
- 共通パターンが適切に抽象化されている
- 既存テストがすべて通過する

## 設計判断

### アプローチ: ハイブリッド（ヘルパー関数 + フロー統合）

選択肢:
1. ヘルパー関数のみ（ボトムアップ） — 共通ブロックを関数に抽出。理解しやすいが reject/request_changes のフロー全体の重複には対処しにくい
2. Strategy パターン（トップダウン） — 共通フローをテンプレート化。最大限の削減だが過度な抽象化。approve の特殊ロジック（次ステップ判定）が嵌まらない
3. ハイブリッド（採用） — reject/request_changes はフロー統合、他はヘルパー関数抽出

理由: reject/request_changes は 95% 同一で enum パラメータ化が自然。submit/resubmit は前提条件が異なり、共通部分のみ関数化が適切。approve は独自ロジック（次ステップ判定）があり独立を維持。

### ヘルパーの配置

新規ファイル `command/helpers.rs` に `WorkflowUseCaseImpl` の `pub(super)` メソッドとして配置。理由: 既存の `usecase/helpers.rs` はユースケース横断の汎用ヘルパー。workflow command 固有のヘルパーは command モジュール内に閉じる。

### reject/request_changes の統合方式

enum `StepTerminationType { Reject, RequestChanges }` を導入し、共通の `terminate_step()` メソッドに統合。変動点（ドメインメソッド、インスタンス遷移、イベント）を enum の match で分岐。

理由: クロージャ/トレイトより enum が Rust として自然で、変動点が 3-4 箇所と少ないため。

### テスト戦略

リファクタリング（振る舞い変更なし）のため、既存テストが安全ネット。新規ヘルパー関数の個別テストは追加しない（既存テストで網羅的にカバーされるため）。テストコード自体の重複解消は Phase 4 で SUT ビルダーを導入して対応。

## 対象と対象外

対象:
- `decision/approve.rs`, `decision/reject.rs`, `decision/request_changes.rs`
- `lifecycle/submit.rs`, `lifecycle/resubmit.rs`
- `command.rs`（test_helpers 拡充）
- 上記ファイルのテストコード

対象外:
- `comment.rs` — 独立したフローで重複が少ない（374行、トランザクション操作なし）
- `lifecycle/create.rs` — 独立したフローで重複が少ない（242行）
- `task.rs`, `dashboard.rs` — Issue #902 のスコープ外（構造的パターン重複であり jscpd 検出閾値以下）
- ドメイン層の変更 — 不要（リファクタリングはユースケース層のみ）

---

## Phase 1: Persistence ヘルパー関数の抽出

全コマンドで繰り返される persistence ボイラープレートをヘルパー関数に抽出する。

### 新規ファイル

`backend/apps/core-service/src/usecase/workflow/command/helpers.rs`

### 抽出するヘルパー

```rust
impl WorkflowUseCaseImpl {
    /// トランザクション開始
    pub(super) async fn begin_tx(&self) -> Result<TxContext, CoreError>;

    /// トランザクションコミット
    pub(super) async fn commit_tx(&self, tx: TxContext) -> Result<(), CoreError>;

    /// ステップの version check 付き更新
    pub(super) async fn save_step(
        &self, tx: &mut TxContext, step: &WorkflowStep,
        expected_version: Version, tenant_id: &TenantId,
    ) -> Result<(), CoreError>;

    /// インスタンスの version check 付き更新
    pub(super) async fn save_instance(
        &self, tx: &mut TxContext, instance: &WorkflowInstance,
        expected_version: Version, tenant_id: &TenantId,
    ) -> Result<(), CoreError>;

    /// インスタンスに紐づくステップ一覧取得
    pub(super) async fn fetch_instance_steps(
        &self, instance_id: &WorkflowInstanceId, tenant_id: &TenantId,
    ) -> Result<Vec<WorkflowStep>, CoreError>;
}
```

### 適用先

- approve.rs: begin_tx, save_step(×2), save_instance, commit_tx, fetch_instance_steps
- reject.rs: begin_tx, save_step(×N), save_instance, commit_tx, fetch_instance_steps
- request_changes.rs: 同上
- submit.rs: begin_tx, save_instance, commit_tx
- resubmit.rs: 同上

### 確認事項

- 型: `TxContext` の定義 → `ringiflow_infra::TxContext`
- 型: `InfraError::Conflict` のバリアント構造 → `ringiflow_infra::InfraError`
- パターン: 既存の `update_with_version_check` 呼び出しパターン → 各コマンドファイル

### 操作パス

該当なし（リファクタリングのみ、振る舞い変更なし）

### テストリスト

ユニットテスト（該当なし — ヘルパーは既存テストで間接的にカバー）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証: `just check` で既存テスト全通過を確認

---

## Phase 2: reject/request_changes のフロー統合

最大の重複源（95% 同一コード）を統合する。

### 新規ファイル

`backend/apps/core-service/src/usecase/workflow/command/decision/common.rs`

### 設計

```rust
/// ステップ終了操作の種別
pub(super) enum StepTerminationType {
    Reject,
    RequestChanges,
}

impl WorkflowUseCaseImpl {
    /// reject/request_changes の共通フロー
    ///
    /// 1. ステップ取得 → 権限チェック → 楽観的ロック
    /// 2. ドメイン操作（種別で分岐）
    /// 3. Pending ステップを Skipped に遷移
    /// 4. インスタンス遷移（種別で分岐）
    /// 5. トランザクション保存
    /// 6. イベントログ（種別で分岐）
    pub(super) async fn terminate_step(
        &self,
        input: ApproveRejectInput,
        step_id: WorkflowStepId,
        tenant_id: TenantId,
        user_id: UserId,
        termination: StepTerminationType,
    ) -> Result<WorkflowWithSteps, CoreError>;
}
```

変動点（enum match で分岐）:
1. ドメインメソッド: `step.reject()` / `step.request_changes()`
2. インスタンス遷移: `instance.complete_with_rejection()` / `instance.complete_with_request_changes()`
3. 権限チェックアクション名: "却下" / "差し戻し"
4. イベント: `STEP_REJECTED` / `STEP_CHANGES_REQUESTED`, ログメッセージ

### 変更ファイル

- `decision.rs`: `mod common;` 追加
- `reject.rs`: 本体を `self.terminate_step(..., StepTerminationType::Reject)` に委譲
- `request_changes.rs`: 本体を `self.terminate_step(..., StepTerminationType::RequestChanges)` に委譲
- `_by_display_number` メソッドはそのまま維持（既に本体メソッドに委譲している）

### 確認事項

- 型: `WorkflowStep` のドメインメソッド `reject()`, `request_changes()` のシグネチャ → ドメイン層
- 型: `WorkflowInstance` の `complete_with_rejection()`, `complete_with_request_changes()` のシグネチャ → ドメイン層
- パターン: `WorkflowStepStatus::Pending` のフィルタリングパターン → reject.rs line 80

### 操作パス

該当なし（リファクタリングのみ）

### テストリスト

ユニットテスト（該当なし — 既存テストでカバー）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証: `just check` で既存テスト全通過を確認。reject_step, request_changes_step の全テストケースが引き続き通過すること。

---

## Phase 3: submit/resubmit の共通部分抽出

approvers 検証とステップ作成ループを共通関数に抽出する。

### 配置

`backend/apps/core-service/src/usecase/workflow/command/lifecycle/common.rs`

### 抽出するヘルパー

```rust
/// approvers と定義のステップの整合性を検証する
pub(super) fn validate_approvers(
    approvers: &[StepApprover],
    approval_step_defs: &[ApprovalStepDef],
) -> Result<(), CoreError>;

impl WorkflowUseCaseImpl {
    /// 定義と approvers に基づいて承認ステップを作成する
    ///
    /// 最初のステップのみ Active、残りは Pending。
    pub(super) async fn create_approval_steps(
        &self,
        instance_id: &WorkflowInstanceId,
        tenant_id: &TenantId,
        approval_step_defs: &[ApprovalStepDef],
        approvers: &[StepApprover],
        now: DateTime<Utc>,
    ) -> Result<Vec<WorkflowStep>, CoreError>;
}
```

### 変更ファイル

- `lifecycle.rs`: `mod common;` 追加
- `submit.rs`: `validate_approvers` + `create_approval_steps` を使用
- `resubmit.rs`: 同上

### 確認事項

- 型: `ApprovalStepDef` の定義場所と構造 → ドメイン層の `extract_approval_steps()` 戻り値
- 型: `DisplayIdEntityType::WorkflowStep` → `ringiflow_domain::value_objects`
- パターン: `counter_repo.next_display_number` の呼び出しパターン → submit.rs line 104

### 操作パス

該当なし（リファクタリングのみ）

### テストリスト

ユニットテスト（該当なし — 既存テストでカバー）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証: `just check` で既存テスト全通過を確認

---

## Phase 4: テストコードの重複解消

テストの SUT 構築と共通データセットアップの重複を解消する。

### 変更ファイル

`command.rs` の `test_helpers` モジュールを拡充。

### 抽出するヘルパー

```rust
#[cfg(test)]
pub(super) mod test_helpers {
    // 既存: single_approval_definition_json, two_step_approval_definition_json, setup_two_step_approval

    // 新規: SUT ビルダー
    pub struct WorkflowCommandTestContext {
        pub definition_repo: MockWorkflowDefinitionRepository,
        pub instance_repo: MockWorkflowInstanceRepository,
        pub step_repo: MockWorkflowStepRepository,
        pub now: DateTime<Utc>,
    }

    impl WorkflowCommandTestContext {
        pub fn new() -> Self;
        pub fn build_sut(&self) -> WorkflowUseCaseImpl;

        /// InProgress インスタンス + Active ステップを作成して登録
        pub async fn setup_active_step(&self, ...) -> (WorkflowInstance, WorkflowStep);

        /// 1段階承認の定義 + インスタンス + ステップを一括作成
        pub async fn setup_single_approval(&self, ...) -> SingleApprovalSetup;
    }
}
```

### 適用対象

approve.rs, reject.rs, request_changes.rs, submit.rs, resubmit.rs の各テストモジュール。

### 確認事項

- パターン: 各テストファイルの共通セットアップパターン → approve.rs tests, reject.rs tests
- 型: `MockTransactionManager`, `MockUserRepository` 等のコンストラクタ → `ringiflow_infra::mock`

### 操作パス

該当なし

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証: `just check` で既存テスト全通過を確認

---

## Phase 5: approve.rs への Phase 1 ヘルパー適用と最終整理

Phase 1 で作成したヘルパーを approve.rs にも適用し、全体を整理する。

### 変更ファイル

- `approve.rs`: Phase 1 ヘルパーを使って persistence ボイラープレートを置換
- 全ファイル: 不要になった import の整理

### 確認事項

- approve.rs の save_step 呼び出しが 2 箇所（承認ステップ + 次ステップ activation）あることを確認

### 操作パス

該当なし

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証: `just check-all` で全テスト通過を最終確認

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | comment.rs と create.rs の除外理由が未明記 | スコープ境界 | 対象外セクションに除外理由を追記 |
| 2回目 | approve.rs のヘルパー適用が Phase 1 と混在 | 不完全なパス | approve.rs は Phase 5 で独立して適用（Phase 2-3 の変更と分離） |
| 3回目 | テスト戦略の判断根拠が不明確 | 曖昧 | 設計判断セクションにテスト戦略を追加 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | 6 対象ファイルすべてがいずれかの Phase でカバー。重複クラスター 1-5 すべてに対応 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各ヘルパーのシグネチャ、配置先、変動点を具体化済み |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | アプローチ選定、ヘルパー配置、統合方式、テスト戦略の各判断に理由を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | comment.rs, create.rs, task.rs, dashboard.rs を除外理由付きで記載 |
| 5 | 技術的前提 | 前提が考慮されている | OK | Rust の impl ブロック分散、pub(super) visibility、enum dispatch を考慮 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | 既存の helpers.rs パターン、command.rs の test_helpers パターンに準拠 |
