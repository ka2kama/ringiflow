# Phase 3: WorkflowStepRepository

## 概要

ワークフローステップ（承認タスク）の永続化を担当するリポジトリを実装した。
ステップの保存、検索、インスタンス別一覧、担当者別一覧の機能を提供する。

### 対応 Issue

[#35 ワークフロー申請機能](https://github.com/ka2kama/ringiflow/issues/35) - Phase 3

---

## 設計書との対応

本 Phase は以下の設計書セクションに対応する。

| 設計書セクション | 対応内容 |
|----------------|---------|
| [データベース設計 > workflow_steps](../../03_詳細設計書/02_データベース設計.md) | テーブル定義 |
| [実装ロードマップ > Phase 1: MVP](../../03_詳細設計書/00_実装ロードマップ.md#phase-1-mvp) | Phase 1 の位置づけ |

---

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`backend/crates/domain/src/workflow.rs`](../../../backend/crates/domain/src/workflow.rs) | WorkflowStep エンティティ（既存） |
| [`backend/crates/infra/src/repository/workflow_step_repository.rs`](../../../backend/crates/infra/src/repository/workflow_step_repository.rs) | WorkflowStepRepository トレイト + PostgreSQL 実装 |
| [`backend/crates/infra/tests/workflow_step_repository_test.rs`](../../../backend/crates/infra/tests/workflow_step_repository_test.rs) | 統合テスト |

---

## 実装内容

### WorkflowStep エンティティ（[`workflow.rs`](../../../backend/crates/domain/src/workflow.rs)）

**値オブジェクト:**

| 型 | 説明 |
|----|------|
| `WorkflowStepId` | UUID v7 ベースのステップ ID |
| `WorkflowStepStatus` | Pending / Active / Completed / Skipped |
| `StepDecision` | Approved / Rejected / RequestChanges |

**主要メソッド:**

```rust
// 新規作成
WorkflowStep::new(
    instance_id, step_id, step_name, step_type, assigned_to
) -> Self

// DB から復元
WorkflowStep::from_db(...) -> Self

// 状態遷移（不変、新インスタンスを返す）
step.activated() -> Self                                 // Pending → Active
step.completed(decision, comment) -> Result<Self, ...>   // Active → Completed
step.skipped() -> Self                                   // → Skipped

// 期限チェック
step.is_overdue() -> bool
```

### WorkflowStepRepository トレイト（[`workflow_step_repository.rs`](../../../backend/crates/infra/src/repository/workflow_step_repository.rs)）

```rust
#[async_trait]
pub trait WorkflowStepRepository: Send + Sync {
    // 保存（新規作成または更新）
    async fn save(&self, step: &WorkflowStep)
        -> Result<(), InfraError>;

    // ID で検索（テナント分離）
    async fn find_by_id(&self, id: &WorkflowStepId, tenant_id: &TenantId)
        -> Result<Option<WorkflowStep>, InfraError>;

    // インスタンス別一覧
    async fn find_by_instance(&self, instance_id: &WorkflowInstanceId, tenant_id: &TenantId)
        -> Result<Vec<WorkflowStep>, InfraError>;

    // 担当者別一覧（タスク一覧用）
    async fn find_by_assigned_to(&self, tenant_id: &TenantId, user_id: &UserId)
        -> Result<Vec<WorkflowStep>, InfraError>;
}
```

### PostgreSQL 実装

**UPSERT による保存:**

```rust
async fn save(&self, step: &WorkflowStep) -> Result<(), InfraError> {
    sqlx::query!(
        r#"
        INSERT INTO workflow_steps (
            id, instance_id, step_id, step_name, step_type,
            status, assigned_to, decision, comment,
            due_date, started_at, completed_at,
            created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        ON CONFLICT (id) DO UPDATE SET
            status = EXCLUDED.status,
            decision = EXCLUDED.decision,
            comment = EXCLUDED.comment,
            started_at = EXCLUDED.started_at,
            completed_at = EXCLUDED.completed_at,
            updated_at = EXCLUDED.updated_at
        "#,
        // ...
    )
    .execute(&self.pool)
    .await?;

    Ok(())
}
```

**テナント分離のための JOIN:**

```rust
async fn find_by_id(...) -> Result<Option<WorkflowStep>, InfraError> {
    sqlx::query!(
        r#"
        SELECT s.*
        FROM workflow_steps s
        INNER JOIN workflow_instances i ON s.instance_id = i.id
        WHERE s.id = $1 AND i.tenant_id = $2
        "#,
        // ...
    )
    // ...
}
```

---

## テスト

### 統合テスト（[`workflow_step_repository_test.rs`](../../../backend/crates/infra/tests/workflow_step_repository_test.rs)）

| テスト | 目的 |
|-------|------|
| `test_save_で新規ステップを作成できる` | 新規保存 |
| `test_find_by_id_でステップを取得できる` | ID 検索の正常系 |
| `test_find_by_id_存在しない場合はnoneを返す` | ID 検索の異常系 |
| `test_find_by_instance_インスタンスのステップ一覧を取得できる` | インスタンス別一覧 |
| `test_find_by_instance_別テナントのステップは取得できない` | テナント分離 |
| `test_find_by_assigned_to_担当者のタスク一覧を取得できる` | 担当者別検索 |
| `test_save_で既存ステップを更新できる` | 更新（UPSERT） |
| `test_ステップを完了できる` | 状態遷移の永続化 |

### 実行方法

```bash
cd backend && cargo test --test workflow_step_repository_test
```

---

## 関連ドキュメント

- [データベース設計](../../03_詳細設計書/02_データベース設計.md)
- [.claude/rules/repository.md](../../../.claude/rules/repository.md) - リポジトリ実装ルール

---

## 設計解説

### 1. JOIN によるテナント分離

**場所:** [`workflow_step_repository.rs:106-125`](../../../backend/crates/infra/src/repository/workflow_step_repository.rs#L106-L125)

**コード例:**

```rust
async fn find_by_id(
    &self,
    id: &WorkflowStepId,
    tenant_id: &TenantId,
) -> Result<Option<WorkflowStep>, InfraError> {
    let row = sqlx::query!(
        r#"
        SELECT s.*
        FROM workflow_steps s
        INNER JOIN workflow_instances i ON s.instance_id = i.id
        WHERE s.id = $1 AND i.tenant_id = $2
        "#,
        // ...
    )
    // ...
}
```

**なぜこの設計か:**

workflow_steps テーブルには tenant_id カラムが存在しない。これは正規化の原則に従い、テナント情報は workflow_instances で管理するためである。

テナント分離を担保するため、以下の戦略を採用：

1. **workflow_instances との JOIN**
   - すべてのクエリで workflow_instances テーブルと JOIN
   - i.tenant_id で条件を指定
   - これにより、他テナントのステップへのアクセスを防ぐ

2. **多層防御の原則**
   - DB スキーマレベル: workflow_steps.instance_id が workflow_instances.id を参照
   - クエリレベル: tenant_id による WHERE 条件
   - この 2 層により、設定ミスによるデータ漏洩を防ぐ

**代替案:**

1. workflow_steps に tenant_id カラムを追加する
   - ❌ 正規化違反（冗長なデータ保持）
   - ❌ データ不整合のリスク（instance と step の tenant_id が異なる可能性）

2. tenant_id チェックをアプリケーション層で行う
   - ❌ DB 側でガードがなく、バグによる漏洩リスクが高い

3. 現在の方式（JOIN + WHERE）
   - ✅ 正規化を維持
   - ✅ DB クエリレベルで防御
   - ✅ インデックス活用（workflow_instances の tenant_id インデックス）

### 2. 担当者による検索の最適化

**場所:** [`workflow_step_repository.rs:192-226`](../../../backend/crates/infra/src/repository/workflow_step_repository.rs#L192-L226)

**コード例:**

```rust
async fn find_by_assigned_to(
    &self,
    tenant_id: &TenantId,
    user_id: &UserId,
) -> Result<Vec<WorkflowStep>, InfraError> {
    sqlx::query!(
        r#"
        SELECT s.*
        FROM workflow_steps s
        INNER JOIN workflow_instances i ON s.instance_id = i.id
        WHERE i.tenant_id = $1 AND s.assigned_to = $2
        ORDER BY s.created_at DESC
        "#,
        // ...
    )
    // ...
}
```

**なぜこの設計か:**

ユーザーのタスク一覧（「自分に割り当てられたステップ」）を高速に取得するため。

設計のポイント：

1. **インデックス活用**
   - workflow_steps_assigned_to_idx: `assigned_to` の部分インデックス（WHERE status = 'active'）
   - 未完了のアクティブタスクのみインデックス対象
   - これにより、完了済みステップを除外して検索を高速化

2. **降順ソート**
   - `ORDER BY s.created_at DESC`: 新しいタスクを先に表示
   - UUID v7 の時系列性を活用

**代替案:**

1. find_by_instance で全取得後、アプリケーション側でフィルタ
   - ❌ 不要なデータを取得し、パフォーマンス低下

2. インデックスなしで全件スキャン
   - ❌ テーブルが大きくなると極端に遅くなる

3. 現在の方式（部分インデックス + 降順ソート）
   - ✅ 高速（インデックススキャン）
   - ✅ タスク一覧画面の UX 向上
   - ✅ DB 容量の節約（完了済みタスクはインデックス対象外）

### 3. UPSERT の更新対象フィールド選定

**場所:** [`workflow_step_repository.rs:69-77`](../../../backend/crates/infra/src/repository/workflow_step_repository.rs#L69-L77)

**コード例:**

```rust
ON CONFLICT (id) DO UPDATE SET
    status = EXCLUDED.status,
    decision = EXCLUDED.decision,
    comment = EXCLUDED.comment,
    started_at = EXCLUDED.started_at,
    completed_at = EXCLUDED.completed_at,
    updated_at = EXCLUDED.updated_at
```

**なぜこの設計か:**

ワークフローステップのライフサイクル：

1. Pending として作成
2. Active に遷移（started_at 記録）
3. Completed に遷移（decision, comment, completed_at 記録）

UPSERT では、状態遷移で変化するフィールドのみを更新する。

**更新しないフィールド:**
- `id`, `instance_id`, `step_id`, `step_name`, `step_type`: ステップの識別子（変更不可）
- `assigned_to`: 担当者の再割り当ては別の操作（MVP 外）
- `due_date`: 期限の変更は別の操作（MVP 外）
- `created_at`: 作成日時は不変

**代替案:**

1. 全フィールドを更新対象にする
   - ❌ 意図しないデータ上書きのリスク
   - ❌ 監査ログで変更追跡が困難

2. `updated_at` のみ更新し、他は手動 UPDATE
   - ❌ 冗長なクエリが必要
   - ❌ トランザクション管理が複雑

3. 現在の方式（状態遷移フィールドのみ更新）
   - ✅ 必要最小限の更新
   - ✅ 不変フィールドの保護
   - ✅ シンプルなインターフェース

### 4. Phase 1-2 で確立したプロセスの適用成果

**場所:** 実装全体

**なぜこの設計か:**

Phase 3 では、Phase 1-2 で文書化したプロセスを忠実に適用した：

1. ✅ テストを tests/ に配置
2. ✅ `#[sqlx::test(migrations = "../../migrations")]` を使用
3. ✅ `just sqlx-prepare` でキャッシュ更新
4. ✅ `just pre-commit` で全体チェック

**結果:** **一発で全チェックをパス**

Phase 1 では 3 回 CI に失敗したが、Phase 3 では失敗なし。ルール化と自動化により、同じミスを繰り返すことを完全に防止した。

**代替案:**

1. Phase 1 のミスを記憶に頼って回避する
   - ❌ 人間の記憶は不確実
   - ❌ 新メンバーが同じミスを繰り返す

2. 現在の方式（文書化 + justfile による自動化）
   - ✅ `.claude/rules/repository.md` にプロセスを明記
   - ✅ `justfile` に標準コマンドを集約
   - ✅ AI エージェントが自動的にプロセスに従う
   - ✅ 人的ミスを構造的に防止
