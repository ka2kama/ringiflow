# #184 テストフィクスチャ集約

## Context

`backend/crates/infra/tests/` 内の6つのテストファイルで、テストデータ生成コードが大量に重複している。特に以下のパターンが顕著:

- シードデータ UUID の繰り返し（`"00000000-0000-0000-0000-000000000001".parse().unwrap()`）: 40箇所以上
- `WorkflowInstance::new(NewWorkflowInstance { ... })`: 約18箇所
- `WorkflowStep::new(NewWorkflowStep { ... })`: 約10箇所
- `DateTime::from_timestamp(1_700_000_000, 0).unwrap()`: 約23箇所

新しいリポジトリテストを追加するたびにコピペが必要で、保守性が低い。

## 方針

`tests/common/mod.rs` に共通フィクスチャを集約する。Rust の統合テスト規約に従い、`tests/common/mod.rs` 形式で配置する（`tests/common.rs` だとテストクレートとして扱われるため）。

## 対象ファイル

### 新規作成

- `backend/crates/infra/tests/common/mod.rs` — 共通フィクスチャモジュール

### 変更

- `backend/crates/infra/tests/workflow_instance_repository_test.rs` — 共通モジュール利用に変更
- `backend/crates/infra/tests/workflow_step_repository_test.rs` — 共通モジュール利用に変更
- `backend/crates/infra/tests/user_repository_test.rs` — `setup_test_data()` を共通化
- `backend/crates/infra/tests/display_id_counter_repository_test.rs` — 定数を共通化
- `backend/crates/infra/tests/workflow_definition_repository_test.rs` — 定数を共通化

### 対象外

- `backend/crates/infra/tests/session_test.rs` — Redis テスト（DB テストと異なる構造のため別スコープ）

## 共通モジュールの設計

### `tests/common/mod.rs` に提供するもの

#### 1. シードデータ定数関数

マイグレーションで作成されるシードデータの ID を関数で提供する:

```rust
/// シードデータのテナント ID
pub fn seed_tenant_id() -> TenantId {
    TenantId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap())
}

/// シードデータのユーザー ID
pub fn seed_user_id() -> UserId {
    UserId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap())
}

/// シードデータのワークフロー定義 ID
pub fn seed_definition_id() -> WorkflowDefinitionId {
    WorkflowDefinitionId::from_uuid("00000000-0000-0000-0000-000000000001".parse().unwrap())
}

/// テスト用の固定日時
pub fn test_now() -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000, 0).unwrap()
}
```

#### 2. エンティティ生成ヘルパー

デフォルト値で WorkflowInstance / WorkflowStep を作成するヘルパー:

```rust
/// デフォルト値で WorkflowInstance を作成
pub fn create_test_instance(display_number: i64) -> WorkflowInstance {
    WorkflowInstance::new(NewWorkflowInstance {
        id: WorkflowInstanceId::new(),
        tenant_id: seed_tenant_id(),
        definition_id: seed_definition_id(),
        definition_version: Version::initial(),
        display_number: DisplayNumber::new(display_number).unwrap(),
        title: "テスト申請".to_string(),
        form_data: json!({}),
        initiated_by: seed_user_id(),
        now: test_now(),
    })
}

/// デフォルト値で WorkflowStep を作成
pub fn create_test_step(instance_id: &WorkflowInstanceId, display_number: i64) -> WorkflowStep {
    WorkflowStep::new(NewWorkflowStep {
        id: WorkflowStepId::new(),
        instance_id: instance_id.clone(),
        display_number: DisplayNumber::new(display_number).unwrap(),
        step_id: "step1".to_string(),
        step_name: "承認".to_string(),
        step_type: "approval".to_string(),
        assigned_to: Some(seed_user_id()),
        now: test_now(),
    })
}
```

#### 3. DB セットアップヘルパー（user_repository_test から移動）

```rust
/// テスト用のテナントとユーザーを DB に作成
pub async fn setup_test_data(pool: &PgPool) -> (TenantId, UserId) { ... }

/// ロールをユーザーに割り当て
pub async fn assign_role(pool: &PgPool, user_id: &UserId) { ... }
```

### 設計判断

- **ビルダーパターンは不採用**: エンティティ生成は引数1〜2個の関数で十分。ビルダーパターンは過度な抽象化
- **関数の戻り値を使う形式**: 各テストが必要な値をカスタマイズできるよう、関数は最小限のパラメータを受け取り、生成したエンティティを返す
- **`title` や `step_name` のカスタマイズが必要なテスト**: 直接 `NewWorkflowInstance { ... }` を書く（ヘルパーに全パラメータを渡すのは本末転倒）
- **session_test.rs は対象外**: Redis テストは DB テストと構造が異なり、共通化のメリットが薄い

## 実装手順

### Phase 1: common モジュール作成 + workflow_instance_repository_test.rs 適用

1. `tests/common/mod.rs` を作成（定数関数 + エンティティ生成ヘルパー）
2. `workflow_instance_repository_test.rs` を共通モジュール利用に書き換え
3. テスト実行で動作確認

### Phase 2: 残りのテストファイル適用

1. `workflow_step_repository_test.rs` を書き換え
2. `workflow_definition_repository_test.rs` を書き換え
3. `display_id_counter_repository_test.rs` を書き換え
4. `user_repository_test.rs` を書き換え（`setup_test_data` / `assign_role` を common に移動）
5. 全テスト実行

## 検証

```bash
cd backend && cargo test -p ringiflow-infra --test workflow_instance_repository_test
cd backend && cargo test -p ringiflow-infra --test workflow_step_repository_test
cd backend && cargo test -p ringiflow-infra --test workflow_definition_repository_test
cd backend && cargo test -p ringiflow-infra --test display_id_counter_repository_test
cd backend && cargo test -p ringiflow-infra --test user_repository_test
just check
```

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | 6テストファイル中5ファイルを対象、session_test.rs は理由付きで除外 |
| 2 | 曖昧さ排除 | OK | 共通モジュールの関数シグネチャを具体的に記載 |
| 3 | 設計判断の完結性 | OK | ビルダー不採用、session_test 除外の判断理由を記載 |
| 4 | スコープ境界 | OK | 対象: DB テストのフィクスチャ集約、対象外: session_test、テストロジックの変更 |
| 5 | 技術的前提 | OK | Rust の `tests/common/mod.rs` 規約を確認（`common.rs` だとテストクレート扱い） |
| 6 | 既存ドキュメント整合 | OK | Issue #184 の実装案と合致 |
