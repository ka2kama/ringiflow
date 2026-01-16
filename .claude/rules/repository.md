---
paths:
  - "**/repository/**/*.rs"
  - "**/infra/**/*.rs"
---

# リポジトリ層実装ルール

このルールはリポジトリ層（データアクセス層）を実装する際に適用される。

## アーキテクチャ原則

### 依存関係の方向

```
API 層 → インフラ層 → ドメイン層 → 共通層
                ↓
           データストア
```

- **ドメイン層はインフラ層に依存しない**: リポジトリは trait で抽象化
- **インフラ層がドメイン層に依存**: リポジトリの実装はインフラ層に配置

### リポジトリパターン

```rust
// ドメイン層: trait 定義
pub trait UserRepository {
    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, RepositoryError>;
    async fn save(&self, user: &User) -> Result<(), RepositoryError>;
}

// インフラ層: 実装
pub struct PostgresUserRepository {
    pool: PgPool,
}

impl UserRepository for PostgresUserRepository {
    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, RepositoryError> {
        // SQLx を使用した実装
    }
}
```

## SQLx の使用

### クエリマクロ（推奨）

コンパイル時に型チェックされる `query!` / `query_as!` を使用:

```rust
// Good: 型安全、コンパイル時チェック
let user = sqlx::query_as!(
    UserRow,
    r#"
    SELECT id, tenant_id, email, name, password_hash, status,
           last_login_at, created_at, updated_at
    FROM users
    WHERE id = $1 AND tenant_id = $2
    "#,
    user_id.as_uuid(),
    tenant_id.as_uuid()
)
.fetch_optional(&self.pool)
.await?;

// Bad: 実行時エラーの可能性
let user = sqlx::query("SELECT * FROM users WHERE id = $1")
    .bind(user_id.as_uuid())
    .fetch_optional(&self.pool)
    .await?;
```

### マイグレーション連携

- `SQLX_OFFLINE=true` 環境変数で CI でのビルドを可能に
- `sqlx prepare` で `.sqlx/` ディレクトリを生成（事前にコミット）

### テナント分離の徹底

**すべてのクエリに `tenant_id` 条件を含める**（RLS が有効になるまで）:

```rust
// Good: tenant_id でフィルタ
sqlx::query!(
    "SELECT * FROM workflows WHERE id = $1 AND tenant_id = $2",
    workflow_id.as_uuid(),
    tenant_id.as_uuid()
)

// Bad: tenant_id がない（他テナントのデータにアクセス可能）
sqlx::query!(
    "SELECT * FROM workflows WHERE id = $1",
    workflow_id.as_uuid()
)
```

## エラーハンドリング

### エラー型の変換

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("データベースエラー: {0}")]
    Database(#[from] sqlx::Error),

    #[error("エンティティが見つかりません: {entity_type} {id}")]
    NotFound {
        entity_type: &'static str,
        id: String,
    },

    #[error("一意制約違反: {0}")]
    UniqueViolation(String),
}

impl From<sqlx::Error> for RepositoryError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => {
                // 呼び出し側で適切な NotFound を生成
                Self::Database(err)
            }
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                Self::UniqueViolation(db_err.message().to_string())
            }
            _ => Self::Database(err),
        }
    }
}
```

### Option vs Result

```rust
// Good: 見つからない場合は None（正常系）
async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, RepositoryError>;

// Bad: 見つからない場合にエラー
async fn find_by_id(&self, id: &UserId) -> Result<User, RepositoryError>;
```

## トランザクション管理

### 単一操作

```rust
// 単一の SELECT/INSERT/UPDATE/DELETE はトランザクション不要
pub async fn save(&self, user: &User) -> Result<(), RepositoryError> {
    sqlx::query!(/* ... */)
        .execute(&self.pool)  // Pool を直接使用
        .await?;
    Ok(())
}
```

### 複数操作

```rust
// 複数の操作は Transaction で原子性を保証
pub async fn create_workflow_with_steps(
    &self,
    workflow: &WorkflowInstance,
    steps: &[WorkflowStep],
) -> Result<(), RepositoryError> {
    let mut tx = self.pool.begin().await?;

    // ワークフローインスタンスを挿入
    sqlx::query!(/* ... */)
        .execute(&mut *tx)
        .await?;

    // ステップを一括挿入
    for step in steps {
        sqlx::query!(/* ... */)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(())
}
```

## テスト

### テストデータベース

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    async fn setup_test_db() -> PgPool {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database");

        // マイグレーション実行
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        pool
    }

    #[tokio::test]
    async fn test_find_by_id_正常系() {
        let pool = setup_test_db().await;
        let repo = PostgresUserRepository::new(pool.clone());

        // テストデータ作成
        let user_id = UserId::new();
        // ...

        // Act
        let result = repo.find_by_id(&user_id).await;

        // Assert
        assert!(result.is_ok());
    }
}
```

### テストの分離

- **単体テスト**: モック/スタブを使用してロジックのみテスト
- **統合テスト**: 実際の DB を使用してクエリの動作確認

## パフォーマンス最適化

### N+1 問題の回避

```rust
// Bad: N+1 クエリ
for workflow in workflows {
    let steps = repo.find_steps_by_workflow_id(&workflow.id).await?; // N回実行
}

// Good: 一括取得
let workflow_ids: Vec<_> = workflows.iter().map(|w| w.id()).collect();
let steps = repo.find_steps_by_workflow_ids(&workflow_ids).await?; // 1回実行
```

### バッチ操作

```rust
// Good: バッチ INSERT
pub async fn save_many(&self, users: &[User]) -> Result<(), RepositoryError> {
    let mut query_builder = QueryBuilder::new(
        "INSERT INTO users (id, tenant_id, email, name, status) "
    );

    query_builder.push_values(users, |mut b, user| {
        b.push_bind(user.id().as_uuid())
         .push_bind(user.tenant_id().as_uuid())
         .push_bind(user.email().as_str())
         .push_bind(user.name())
         .push_bind(user.status().as_str());
    });

    query_builder.build()
        .execute(&self.pool)
        .await?;

    Ok(())
}
```

### インデックスの活用

- WHERE 句の条件列にインデックスを作成
- 複合インデックスは頻繁に使う組み合わせに限定
- `EXPLAIN ANALYZE` で実行計画を確認

## AI エージェントへの指示

リポジトリ層を実装する際:

1. **必ず `tenant_id` でフィルタ**: データ漏洩を防ぐ
2. **SQLx クエリマクロを使用**: 型安全性を確保
3. **エラーを適切に変換**: `RepositoryError` にマップ
4. **テストを書く**: 少なくとも CRUD の基本操作
5. **N+1 を意識**: バッチ操作や JOIN を活用

**禁止事項:**
- `tenant_id` なしのクエリ
- 文字列結合による SQL 構築
- トランザクションが必要な箇所での未使用
- テストなしでのリポジトリ実装

## 参照

- データベース設計: [docs/03_詳細設計書/02_データベース設計.md](../../docs/03_詳細設計書/02_データベース設計.md)
- テナント削除設計: [docs/03_詳細設計書/06_テナント退会時データ削除設計.md](../../docs/03_詳細設計書/06_テナント退会時データ削除設計.md)
