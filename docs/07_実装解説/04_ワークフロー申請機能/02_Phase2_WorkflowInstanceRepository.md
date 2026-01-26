# Phase 2: WorkflowInstanceRepository

## 概要

ワークフローインスタンス（申請案件）の永続化を担当するリポジトリを実装した。
インスタンスの保存、検索、テナント一覧、申請者検索の機能を提供する。

### 対応 Issue

[#35 ワークフロー申請機能](https://github.com/ka2kama/ringiflow/issues/35) - Phase 2

---

## 設計書との対応

本 Phase は以下の設計書セクションに対応する。

| 設計書セクション | 対応内容 |
|----------------|---------|
| [データベース設計 > workflow_instances](../../03_詳細設計書/02_データベース設計.md) | テーブル定義 |
| [実装ロードマップ > Phase 1: MVP](../../03_詳細設計書/00_実装ロードマップ.md#phase-1-mvp) | Phase 1 の位置づけ |

---

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`backend/crates/domain/src/workflow.rs`](../../../backend/crates/domain/src/workflow.rs) | WorkflowInstance エンティティ |
| [`backend/crates/infra/src/repository/workflow_instance_repository.rs`](../../../backend/crates/infra/src/repository/workflow_instance_repository.rs) | WorkflowInstanceRepository トレイト + PostgreSQL 実装 |
| [`backend/crates/infra/tests/workflow_instance_repository_test.rs`](../../../backend/crates/infra/tests/workflow_instance_repository_test.rs) | 統合テスト |

---

## 実装内容

### WorkflowInstance エンティティ（[`workflow.rs`](../../../backend/crates/domain/src/workflow.rs)）

**値オブジェクト:**

| 型 | 説明 |
|----|------|
| `WorkflowInstanceId` | UUID v7 ベースのインスタンス ID |
| `WorkflowInstanceStatus` | Draft / Pending / InProgress / Approved / Rejected / Cancelled |

**主要メソッド:**

```rust
// 新規作成
WorkflowInstance::new(
    tenant_id, definition_id, definition_version,
    title, form_data, initiated_by
) -> Self

// DB から復元
WorkflowInstance::from_db(...) -> Self

// 状態遷移（不変、新インスタンスを返す）
instance.submitted() -> Result<Self, DomainError>  // Draft → Pending
instance.approved() -> Self                         // → Approved
instance.rejected() -> Self                         // → Rejected
instance.cancelled() -> Result<Self, DomainError>   // → Cancelled
instance.with_current_step(step_id) -> Self         // → InProgress

// 検証
instance.can_edit() -> Result<(), DomainError>
```

### WorkflowInstanceRepository トレイト（[`workflow_instance_repository.rs`](../../../backend/crates/infra/src/repository/workflow_instance_repository.rs)）

```rust
#[async_trait]
pub trait WorkflowInstanceRepository: Send + Sync {
    // 保存（新規作成または更新）
    async fn save(&self, instance: &WorkflowInstance)
        -> Result<(), InfraError>;

    // ID で検索（テナント分離）
    async fn find_by_id(&self, id: &WorkflowInstanceId, tenant_id: &TenantId)
        -> Result<Option<WorkflowInstance>, InfraError>;

    // テナント内の一覧
    async fn find_by_tenant(&self, tenant_id: &TenantId)
        -> Result<Vec<WorkflowInstance>, InfraError>;

    // 申請者による検索
    async fn find_by_initiated_by(&self, tenant_id: &TenantId, user_id: &UserId)
        -> Result<Vec<WorkflowInstance>, InfraError>;
}
```

### PostgreSQL 実装

**UPSERT による保存:**

```rust
async fn save(&self, instance: &WorkflowInstance) -> Result<(), InfraError> {
    sqlx::query!(
        r#"
        INSERT INTO workflow_instances (
            id, tenant_id, definition_id, definition_version,
            title, form_data, status, current_step_id,
            initiated_by, submitted_at, completed_at,
            created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        ON CONFLICT (id) DO UPDATE SET
            title = EXCLUDED.title,
            form_data = EXCLUDED.form_data,
            status = EXCLUDED.status,
            current_step_id = EXCLUDED.current_step_id,
            submitted_at = EXCLUDED.submitted_at,
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

---

## テスト

### 統合テスト（[`workflow_instance_repository_test.rs`](../../../backend/crates/infra/tests/workflow_instance_repository_test.rs)）

| テスト | 目的 |
|-------|------|
| `test_save_で新規インスタンスを作成できる` | 新規保存 |
| `test_find_by_id_でインスタンスを取得できる` | ID 検索の正常系 |
| `test_find_by_id_存在しない場合はnoneを返す` | ID 検索の異常系 |
| `test_find_by_tenant_テナント内の一覧を取得できる` | テナント一覧 |
| `test_find_by_tenant_別テナントのインスタンスは取得できない` | テナント分離 |
| `test_find_by_initiated_by_申請者によるインスタンスを取得できる` | 申請者検索 |
| `test_save_で既存インスタンスを更新できる` | 更新（UPSERT） |
| `test_トレイトはsendとsyncを実装している` | トレイト検証 |

### 実行方法

```bash
cd backend && cargo test --test workflow_instance_repository_test
```

---

## 関連ドキュメント

- [データベース設計](../../03_詳細設計書/02_データベース設計.md)
- [.claude/rules/repository.md](../../../.claude/rules/repository.md) - リポジトリ実装ルール

---

## 設計解説

### 1. UPSERT パターンの採用

**場所:** [`workflow_instance_repository.rs:117-155`](../../../backend/crates/infra/src/repository/workflow_instance_repository.rs#L117-L155)

**コード例:**

```rust
sqlx::query!(
    r#"
    INSERT INTO workflow_instances (...)
    VALUES (...)
    ON CONFLICT (id) DO UPDATE SET
        title = EXCLUDED.title,
        ...
    "#,
    // ...
)
```

**なぜこの設計か:**

ワークフローインスタンスは、以下のライフサイクルを持つ：

1. Draft として作成
2. 何度か編集
3. Submit して Pending に遷移
4. Approved/Rejected で完了

同じエンティティを繰り返し保存するため、`save` メソッドで UPSERT を使用することで：

- 新規作成と更新を同じインターフェースで扱える
- 呼び出し側が INSERT/UPDATE を意識する必要がない
- ドメインモデルの不変性（状態遷移ごとに新インスタンス）と相性が良い

**代替案:**

1. `insert` と `update` を分ける
   - ❌ 呼び出し側が存在チェックを行う必要がある
   - ❌ コードが冗長になる

2. `update` のみで、初回は事前に `insert` を呼ぶ
   - ❌ 2 回のクエリが必要
   - ❌ トランザクション管理が複雑

3. 現在の方式（UPSERT）
   - ✅ 1 回のクエリで完結
   - ✅ シンプルなインターフェース
   - ✅ ドメインモデルと整合

### 2. Phase 1 の失敗から学んだ実装プロセス

**場所:** 実装全体の進め方

**なぜこの設計か:**

Phase 1 で以下のミスを経験：

1. sqlx::test に migrations パラメータを指定していなかった
2. SQLx クエリキャッシュを更新していなかった
3. tests/ に配置すべきテストを src/ に配置していた
4. --all-targets を指定せずに cargo sqlx prepare を実行していた

これらを防ぐため、Phase 2 では以下の手順を厳守：

```bash
# 1. 既存パターンを確認
ls backend/crates/infra/tests/
grep -r "sqlx::test" backend/crates/infra/tests/

# 2. テストを tests/ に作成
# 3. #[sqlx::test(migrations = "../../migrations")] を使用
# 4. just sqlx-prepare でキャッシュ更新
# 5. just pre-commit で全体チェック
```

結果：**Phase 2 は一発で CI をパス**

**代替案:**

1. Phase 1 と同じミスを繰り返す
   - ❌ CI で失敗し、時間を浪費

2. 手動でルールを覚える
   - ❌ 忘れやすい、新メンバーが学習コスト高い

3. 現在の方式（文書化 + 標準化コマンド）
   - ✅ `.claude/rules/repository.md` にルール明記
   - ✅ `justfile` に `sqlx-prepare`, `pre-commit` 追加
   - ✅ AI エージェントが自動的に従う

### 3. 申請者による検索の追加

**場所:** [`workflow_instance_repository.rs:256-295`](../../../backend/crates/infra/src/repository/workflow_instance_repository.rs#L256-L295)

**コード例:**

```rust
async fn find_by_initiated_by(
    &self,
    tenant_id: &TenantId,
    user_id: &UserId,
) -> Result<Vec<WorkflowInstance>, InfraError> {
    let rows = sqlx::query!(
        r#"
        SELECT ...
        FROM workflow_instances
        WHERE tenant_id = $1 AND initiated_by = $2
        ORDER BY created_at DESC
        "#,
        tenant_id.as_uuid(),
        user_id.as_uuid()
    )
    // ...
}
```

**なぜこの設計か:**

ユーザーが「自分が申請したワークフロー」を一覧表示する機能を実装するため。

テナント分離を維持しつつ、`initiated_by` でフィルタすることで：

- ユーザーは自分の申請のみを見られる
- インデックス（`workflow_instances_initiated_by_idx`）により高速検索
- テナント ID も条件に含めることで、セキュリティを強化

**代替案:**

1. `find_by_tenant` で全取得後、アプリケーション側でフィルタ
   - ❌ 不要なデータを取得し、パフォーマンス低下

2. tenant_id を条件に含めない
   - ❌ セキュリティリスク（他テナントのデータ取得可能性）

3. 現在の方式（DB 側でフィルタ + tenant_id 条件）
   - ✅ 高速（インデックス活用）
   - ✅ セキュア（多層防御）
   - ✅ シンプル（クエリ 1 回）

### 4. Version 型の as_i32() 使用

**場所:** [`workflow_instance_repository.rs:139`](../../../backend/crates/infra/src/repository/workflow_instance_repository.rs#L139)

**コード例:**

```rust
instance.definition_version().as_i32(),
```

**なぜこの設計か:**

`Version` 型は内部で `u32` を保持するが、PostgreSQL の INTEGER 型は `i32` に対応する。`as_i32()` メソッドで安全に変換する。

**代替案:**

1. Version を i32 で保持する
   - ❌ ドメインモデルが DB の制約に影響される

2. Version を u32 のまま、キャスト時にエラーハンドリング
   - ❌ バージョン番号が i32::MAX を超えることは現実的にない
   - ❌ 冗長

3. 現在の方式（as_i32() メソッド）
   - ✅ ドメインモデルは u32 のまま
   - ✅ DB 層で明示的に変換
   - ✅ 型安全性を維持
