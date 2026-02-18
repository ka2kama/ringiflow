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

### テスト配置

| 配置場所 | 用途 | DB 接続 | CI ジョブ |
|---------|------|--------|-----------|
| `src/` の `#[cfg(test)]` | トレイトの Send + Sync チェックのみ | 不要 | Rust（ユニットテスト） |
| `tests/` | 実際のリポジトリテスト | 必要 | Rust Integration |

**重要:** DB 接続が必要なテストは必ず `backend/crates/infra/tests/` に配置する。

```rust
// src/repository/user_repository.rs
#[cfg(test)]
mod tests {
    use super::*;

    /// トレイトオブジェクトとして使用できることを確認
    #[test]
    fn test_トレイトはsendとsyncを実装している() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn UserRepository>>();
    }
}
```

```rust
// tests/user_repository_test.rs
#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_id_正常系(pool: PgPool) {
    let repo = PostgresUserRepository::new(pool);
    // ...
}
```

### sqlx::test マクロの使用

`tests/` 内のテストでは `#[sqlx::test]` マクロを使用し、必ず `migrations` パラメータを指定する：

```rust
#[sqlx::test(migrations = "../../migrations")]
async fn test_メールアドレスでユーザーを取得できる(pool: PgPool) {
    let repo = PostgresUserRepository::new(pool);
    let tenant_id = TenantId::from_uuid("...".parse().unwrap());

    let result = repo.find_by_email(&tenant_id, &email).await;

    assert!(result.is_ok());
}
```

**理由:** ワークスペース構成では、デフォルトのマイグレーションパス（プロジェクトルートの `migrations/`）が機能しない。

### SQLx オフラインモード対応

新しい `sqlx::query!` を追加したら、必ずクエリキャッシュを更新する：

```bash
just sqlx-prepare
```

または

```bash
cd backend && cargo sqlx prepare --workspace -- --all-targets
```

**重要:** `--all-targets` を指定しないと `tests/` 内のクエリがキャッシュされず、CI で失敗する。

変更された `.sqlx/` ファイルは必ずコミットに含める。

### テストの分離

- **単体テスト（src/）**: トレイトの型チェックのみ、DB 接続不要
- **統合テスト（tests/）**: 実際の DB を使用してクエリの動作確認

## ORDER BY の決定性

`ORDER BY` には必ず決定的（ユニーク）なカラムを含める。同一値を持つ行の並び順は PostgreSQL で保証されない。

```sql
-- Bad: created_at が同一の行があると順序が非決定的
ORDER BY created_at ASC

-- Good: display_number はインスタンス内でユニーク
ORDER BY display_number ASC

-- Good: ユニークでないカラムで並べる場合は、ユニークカラムをタイブレーカーに追加
ORDER BY created_at DESC, id ASC
```

判定テスト: 「このカラムに同一値の行が複数存在しうるか？」→ Yes なら、ユニークカラムをタイブレーカーとして追加する。

改善の経緯: `ORDER BY created_at ASC` で `workflow_steps` の順序が非決定的になり、API テストが失敗した（#679）

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

### 新しいリポジトリを実装する際の必須手順

1. **既存パターンの確認**
   ```bash
   ls backend/crates/infra/tests/
   grep -r "sqlx::test" backend/crates/infra/tests/
   ```

2. **テストファイルの作成**
   - `backend/crates/infra/tests/` に新しいテストファイルを作成
   - `#[sqlx::test(migrations = "../../migrations")]` を使用
   - DB 接続が必要なテストを `src/` に書かない

3. **実装**
   - 必ず `tenant_id` でフィルタ（データ漏洩を防ぐ）
   - SQLx クエリマクロを使用（型安全性を確保）
   - エラーを適切に変換（`RepositoryError` にマップ）

4. **SQLx クエリキャッシュの更新**
   ```bash
   just sqlx-prepare
   ```

5. **コミット前の確認**
   ```bash
   just pre-commit
   ```

この手順を省略しない。ローカルで動作していても、CI で失敗する可能性がある。

**禁止事項:**
- DB 接続が必要なテストを `src/` に配置
- `sqlx::test` で `migrations` パラメータを省略
- `sqlx-prepare` を実行せずにコミット
- `tenant_id` なしのクエリ
- 文字列結合による SQL 構築
- トランザクションが必要な箇所での未使用
- テストなしでのリポジトリ実装
- ユニークでないカラムのみの `ORDER BY`（タイブレーカーなし）

## エンティティ追加・更新パス追加時の必須対応

新しいエンティティを追加する場合、または既存エンティティに新しい更新パス（ユースケース）を追加する場合、エンティティ影響マップを作成・更新する。

→ 詳細: [`docs/03_詳細設計書/エンティティ影響マップ/`](../../docs/03_詳細設計書/エンティティ影響マップ/)

1. [テンプレート](../../docs/03_詳細設計書/エンティティ影響マップ/TEMPLATE.md)を使って影響マップを作成
2. 更新パスと競合リスクを洗い出す
3. 必要に応じて競合対策（楽観的ロック等）を設計する

## 参照

- データベース設計: [docs/03_詳細設計書/02_データベース設計.md](../../docs/03_詳細設計書/02_データベース設計.md)
- テナント削除設計: [docs/03_詳細設計書/06_テナント退会時データ削除設計.md](../../docs/03_詳細設計書/06_テナント退会時データ削除設計.md)
