# Phase 1: WorkflowDefinitionRepository

## 概要

ワークフロー定義（テンプレート）の永続化を担当するリポジトリを実装した。
公開されている定義の取得、ID による検索機能を提供する。

### 対応 Issue

[#35 ワークフロー申請機能](https://github.com/ka2kama/ringiflow/issues/35) - Phase 1

---

## 設計書との対応

本 Phase は以下の設計書セクションに対応する。

| 設計書セクション | 対応内容 |
|----------------|---------|
| [データベース設計 > workflow_definitions](../../03_詳細設計書/02_データベース設計.md) | テーブル定義 |
| [実装ロードマップ > Phase 1: MVP](../../03_詳細設計書/00_実装ロードマップ.md#phase-1-mvp) | Phase 1 の位置づけ |

---

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`backend/crates/domain/src/workflow.rs`](../../../backend/crates/domain/src/workflow.rs) | WorkflowDefinition エンティティ |
| [`backend/crates/infra/src/repository/workflow_definition_repository.rs`](../../../backend/crates/infra/src/repository/workflow_definition_repository.rs) | WorkflowDefinitionRepository トレイト + PostgreSQL 実装 |
| [`backend/crates/infra/tests/workflow_definition_repository_test.rs`](../../../backend/crates/infra/tests/workflow_definition_repository_test.rs) | 統合テスト |

---

## 実装内容

### WorkflowDefinition エンティティ（[`workflow.rs`](../../../backend/crates/domain/src/workflow.rs)）

**値オブジェクト:**

| 型 | 説明 |
|----|------|
| `WorkflowDefinitionId` | UUID v7 ベースの定義 ID |
| `WorkflowDefinitionStatus` | Draft / Published / Archived |

**主要メソッド:**

```rust
// 新規作成
WorkflowDefinition::new(
    tenant_id, name, description, definition, created_by
) -> Self

// DB から復元
WorkflowDefinition::from_db(...) -> Self

// 状態変更（不変、新インスタンスを返す）
definition.published() -> Result<Self, DomainError>
definition.archived() -> Self

// 検証
definition.can_publish() -> Result<(), DomainError>
```

### WorkflowDefinitionRepository トレイト（[`workflow_definition_repository.rs`](../../../backend/crates/infra/src/repository/workflow_definition_repository.rs)）

```rust
#[async_trait]
pub trait WorkflowDefinitionRepository: Send + Sync {
    // テナント内の公開定義一覧
    async fn find_published_by_tenant(&self, tenant_id: &TenantId)
        -> Result<Vec<WorkflowDefinition>, InfraError>;

    // ID で検索（テナント分離）
    async fn find_by_id(&self, id: &WorkflowDefinitionId, tenant_id: &TenantId)
        -> Result<Option<WorkflowDefinition>, InfraError>;
}
```

### PostgreSQL 実装

**tenant_id による分離:**

```rust
async fn find_published_by_tenant(&self, tenant_id: &TenantId)
    -> Result<Vec<WorkflowDefinition>, InfraError>
{
    let rows = sqlx::query!(
        r#"
        SELECT id, tenant_id, name, description, version, definition,
               status, created_by, created_at, updated_at
        FROM workflow_definitions
        WHERE tenant_id = $1 AND status = 'published'
        ORDER BY created_at DESC
        "#,
        tenant_id.as_uuid()
    )
    .fetch_all(&self.pool)
    .await?;

    // 行からエンティティへの変換
    // ...
}
```

---

## テスト

### 統合テスト（[`workflow_definition_repository_test.rs`](../../../backend/crates/infra/tests/workflow_definition_repository_test.rs)）

| テスト | 目的 |
|-------|------|
| `test_find_published_by_tenant_returns_published_definitions` | 公開定義の取得 |
| `test_find_published_by_tenant_filters_by_tenant` | テナント分離の検証 |
| `test_find_by_id_returns_definition_when_exists` | ID 検索の正常系 |
| `test_find_by_id_returns_none_when_not_exists` | ID 検索の異常系 |

### 実行方法

```bash
cd backend && cargo test --test workflow_definition_repository_test
```

---

## 関連ドキュメント

- [データベース設計](../../03_詳細設計書/02_データベース設計.md)
- [.claude/rules/repository.md](../../../.claude/rules/repository.md) - リポジトリ実装ルール

---

## 設計解説

### 1. sqlx::test のマイグレーション指定

**場所:** [`tests/workflow_definition_repository_test.rs:19`](../../../backend/crates/infra/tests/workflow_definition_repository_test.rs#L19)

**コード例:**

```rust
#[sqlx::test(migrations = "../../migrations")]
async fn test_find_published_by_tenant_returns_published_definitions(pool: PgPool) {
    // ...
}
```

**なぜこの設計か:**

ワークスペース構成では、sqlx::test がデフォルトで探すマイグレーションパス（プロジェクトルートの `migrations/`）が機能しない。テストファイルからの相対パスを明示的に指定する必要がある。

**代替案:**

1. マイグレーションをクレートごとに配置する
   - ❌ マイグレーション管理が分散し、一貫性を保ちにくい

2. 環境変数でパスを指定する
   - ❌ 各開発者が環境変数を設定する手間が増える

3. 現在の方式（相対パス指定）
   - ✅ テストコード内で完結し、明示的

### 2. テストの配置（tests/ vs src/）

**場所:** テスト全体の配置戦略

**なぜこの設計か:**

DB 接続が必要なテストを `tests/` に配置することで：

1. **CI の最適化**: ユニットテストジョブで DB を起動する必要がない
2. **明確な分離**: 単体テスト（src/）と統合テスト（tests/）の責務が明確
3. **実行速度**: cargo test（DB 不要）と cargo test --test（DB 必要）を分離可能

**代替案:**

1. すべてのテストを src/ に配置
   - ❌ ユニットテストジョブでも DB が必要になり、CI が遅くなる

2. 現在の方式（tests/ に配置）
   - ✅ ユニットテストと統合テストを分離でき、CI を最適化できる

### 3. クエリキャッシュの管理

**場所:** SQLx オフラインモード対応

**コード例:**

```bash
just sqlx-prepare
# → cd backend && cargo sqlx prepare --workspace -- --all-targets
```

**なぜこの設計か:**

CI で `SQLX_OFFLINE=true` を設定し、DB 接続なしでビルド可能にするため、事前にクエリのメタデータをキャッシュする必要がある。`--all-targets` を指定することで、tests/ 内のクエリもキャッシュされる。

**代替案:**

1. CI で DB を起動してビルドする
   - ❌ CI の実行時間が長くなる
   - ❌ DB の起動失敗でビルドが失敗する可能性

2. オフラインモードを使用しない
   - ❌ ネットワーク環境によってビルドが失敗する

3. 現在の方式（オフラインモード + キャッシュ）
   - ✅ CI が高速で安定
   - ✅ ネットワーク環境に依存しない

### 4. tenant_id による分離の徹底

**場所:** [`workflow_definition_repository.rs:83-102`](../../../backend/crates/infra/src/repository/workflow_definition_repository.rs#L83-L102)

**コード例:**

```rust
async fn find_published_by_tenant(&self, tenant_id: &TenantId)
    -> Result<Vec<WorkflowDefinition>, InfraError>
{
    let rows = sqlx::query!(
        r#"
        SELECT ...
        FROM workflow_definitions
        WHERE tenant_id = $1 AND status = 'published'
        "#,
        tenant_id.as_uuid()
    )
    // ...
}
```

**なぜこの設計か:**

マルチテナント SaaS では、テナント間のデータ漏洩を防ぐため、すべてのクエリに `tenant_id` 条件を含める必要がある。RLS（Row Level Security）が有効になるまでは、アプリケーションレベルで徹底する。

**代替案:**

1. tenant_id のチェックをミドルウェアで行う
   - ❌ クエリレベルでの防御が弱くなる

2. RLS に完全に依存する
   - ❌ Phase 1 では RLS 未実装

3. 現在の方式（全クエリに tenant_id）
   - ✅ 多層防御の原則に従う
   - ✅ コードレビューで漏れを発見しやすい
