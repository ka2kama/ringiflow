# Issue #529: リポジトリ・インフラ層のクローン削減

## Context

Epic #467（`just check` / `check-all` の警告をゼロにする）の Story 6。
インフラ層（`backend/crates/infra/src/`）に jscpd が検出した 21 クローン（338行、6.01%）を削減し、保守性を向上させる。
`user_repository.rs` のファイルサイズ超過（784行 > 500行閾値）も同時に改善する。

## スコープ

### 対象: 14 クローン

| # | カテゴリ | クローン数 | 手法 |
|---|---------|-----------|------|
| A | User 行マッピング（user_repository.rs 内部） | 6 | UserRow + TryFrom |
| B | Permission パース/変換（role ↔ user 間 + role 内部） | 3 | ヘルパー関数の共有・抽出 |
| D | WorkflowDefinition マッピング（内部） | 1 | Row + TryFrom |
| E | PostgreSQL deletion テーブル操作（3ファイル間） | 3 | 宣言的マクロ |
| F | session.rs SCAN ループ（内部） | 1 | ヘルパーメソッド抽出 |

### 対象外: 7 クローン（理由付き）

| # | カテゴリ | クローン数 | 理由 |
|---|---------|-----------|------|
| C | workflow_step/instance SELECT句 | 4 | sqlx マクロの制約（SQL文字列はリテラル必須）。変換ロジックは既に Row+TryFrom で共通化済み |
| - | user_repository.rs #7 (replace_user_roles) | 1 | DELETE+INSERT の3行SQL。先行 Story #527 の判断「2-3行のボイラープレートは統一しない」に該当 |
| - | user_repository.rs #9 (count_active match) | 1 | sqlx::query_scalar! の制約でSQLが異なる |
| - | redis_session.rs 内部 (scan_count ↔ scan_and_delete) | 1 | SCAN ループ構造は類似するが、async処理の差異（count only vs delete+count）があり、共通化にはasyncクロージャが必要で Rust では非実用的 |

## 設計判断

### 判断1: User 行マッピング → UserRow + TryFrom

workflow_step/instance_repository.rs で確立済みのパターンを踏襲。`sqlx::query!`（匿名型）から `sqlx::query_as!`（名前付き構造体）に変更。

参照パターン: `workflow_step_repository.rs` L79-135

### 判断2: PostgreSQL deletion → 宣言的マクロ

3ファイル（postgres_user/role/display_id）が完全に同一構造（テーブル名のみ異なる）。
マクロの引数に完全な SQL リテラルを渡すことで、`sqlx::query!` のコンパイル時検証を維持する。

`concat!` + `sqlx::query!` は proc macro の制約で動作しないため、SQL全文をリテラルとして渡す設計。

### 判断3: Permission パース → pub(crate) 共有

`role_repository.rs` L68-78 の既存 `parse_permissions` を `pub(crate)` に変更し、`user_repository.rs` から呼ぶ。
新モジュールを作るほどの複雑さではない。

### 判断4: session.rs SCAN ループ → ファイル内ヘルパー

session.rs 内の `delete_all_for_tenant` と `delete_all_csrf_for_tenant` の SCAN+DELETE ループが完全同一（21行）。
プライベートヘルパーメソッドに抽出する。redis_session.rs との間の共通化は行わない（戻り値型が異なる）。

## Phase 分割

### Phase 1: session.rs の SCAN ループ共通化

対象ファイル: `backend/crates/infra/src/session.rs`

変更内容:
- `scan_and_delete_keys` 非同期ヘルパー関数を追加（L328-349 の SCAN ループを抽出）
- `delete_all_for_tenant`（L323-355）をヘルパー呼び出しに書き換え
- `delete_all_csrf_for_tenant`（L410-436）をヘルパー呼び出しに書き換え

```rust
/// SCAN でパターンにマッチするキーを全て削除する
async fn scan_and_delete_keys(
    conn: &mut ConnectionManager,
    pattern: &str,
) -> Result<(), InfraError> {
    let mut cursor = 0u64;
    loop {
        let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(pattern)
            .arg("COUNT")
            .arg(100)
            .query_async(conn)
            .await?;
        if !keys.is_empty() {
            let _: () = conn.del(&keys).await?;
        }
        cursor = next_cursor;
        if cursor == 0 {
            break;
        }
    }
    Ok(())
}
```

#### 確認事項
- [ ] `ConnectionManager` の `&mut` 渡しパターン → session.rs 内の既存メソッドで確認
- [ ] redis::cmd("SCAN") の引数パターン → session.rs L332-338 で確認済み

#### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

統合テスト:
- [ ] 既存の session テスト（`just test-rust-integration` 内）がリグレッションなく通ること

E2E テスト（該当なし）

削減効果: 1 クローン、約 20 行削減

---

### Phase 2: deletion/ の PostgreSQL 共通化

対象ファイル:
- `backend/crates/infra/src/deletion/mod.rs`（マクロ定義追加）
- `backend/crates/infra/src/deletion/postgres_simple.rs`（新規、マクロ呼び出し）
- `backend/crates/infra/src/deletion/postgres_user.rs`（削除）
- `backend/crates/infra/src/deletion/postgres_role.rs`（削除）
- `backend/crates/infra/src/deletion/postgres_display_id.rs`（削除）

変更内容:
- `mod.rs` に `define_simple_postgres_deleter!` マクロを定義
- 3ファイルを `postgres_simple.rs` に統合（マクロ呼び出し3回）
- `mod.rs` の `mod` 宣言と `pub use` を更新
- `auth_credentials.rs` が同構造であれば 4つ目として統合

マクロ設計（SQL リテラルを直接渡し、sqlx コンパイル時検証を維持）:

```rust
macro_rules! define_simple_postgres_deleter {
    (
        name: $name:ident,
        deleter_name: $deleter_name:literal,
        delete_sql: $delete_sql:literal,
        count_sql: $count_sql:literal,
        doc: $doc:literal
    ) => {
        #[doc = $doc]
        pub struct $name {
            pool: PgPool,
        }

        impl $name {
            pub fn new(pool: PgPool) -> Self {
                Self { pool }
            }
        }

        #[async_trait]
        impl TenantDeleter for $name {
            fn name(&self) -> &'static str {
                $deleter_name
            }

            async fn delete(
                &self,
                tenant_id: &TenantId,
            ) -> Result<DeletionResult, InfraError> {
                let result =
                    sqlx::query!($delete_sql, tenant_id.as_uuid())
                        .execute(&self.pool)
                        .await?;

                Ok(DeletionResult {
                    deleted_count: result.rows_affected(),
                })
            }

            async fn count(&self, tenant_id: &TenantId) -> Result<u64, InfraError> {
                let count =
                    sqlx::query_scalar!($count_sql, tenant_id.as_uuid())
                        .fetch_one(&self.pool)
                        .await?;

                Ok(count as u64)
            }
        }
    };
}
```

#### 確認事項
- [ ] `sqlx::query!` がマクロ展開後のリテラル文字列を受け入れるか → 実装時にコンパイルで検証
- [ ] `auth_credentials.rs` の構造 → Read で確認（plan mode でアクセス拒否のため実装時に確認）
- [ ] `deletion/mod.rs` の `pub use` パスが外部参照と一致するか → mod.rs L22-29 で確認済み
- [ ] 既存テストが `PostgresUserDeleter` 等を名前で参照しているか → 確認が必要

#### テストリスト

ユニットテスト:
- [ ] `test_send_syncを満たす` テストが新構造で通ること（redis_session.rs L119-122 の既存パターン参照）

ハンドラテスト（該当なし）

API テスト（該当なし）

統合テスト:
- [ ] 既存の deletion 関連テストがリグレッションなく通ること

E2E テスト（該当なし）

削減効果: 3 クローン（+auth_credentials で最大 4）、約 60 行削減

---

### Phase 3: WorkflowDefinition の Row + TryFrom 導入

対象ファイル: `backend/crates/infra/src/repository/workflow_definition_repository.rs`

変更内容:
- `WorkflowDefinitionRow` 構造体を定義（workflow_step_repository.rs L79-100 のパターンに従う）
- `TryFrom<WorkflowDefinitionRow> for WorkflowDefinition` を実装
- `find_published_by_tenant`（L109-130）の `.map(|row| ...)` を `.map(WorkflowDefinition::try_from)` に
- `find_by_id`（L166-182）のインラインマッピングを `WorkflowDefinition::try_from(row)?` に
- `sqlx::query!` を `sqlx::query_as!` に変更

```rust
/// DB の workflow_definitions テーブルの行を表す中間構造体
struct WorkflowDefinitionRow {
    id: Uuid,
    tenant_id: Uuid,
    name: String,
    description: Option<String>,
    version: i32,
    definition: serde_json::Value,
    status: String,
    created_by: Uuid,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<WorkflowDefinitionRow> for WorkflowDefinition {
    type Error = InfraError;

    fn try_from(row: WorkflowDefinitionRow) -> Result<Self, Self::Error> {
        Ok(WorkflowDefinition::from_db(WorkflowDefinitionRecord {
            id:          WorkflowDefinitionId::from_uuid(row.id),
            tenant_id:   TenantId::from_uuid(row.tenant_id),
            name:        WorkflowName::new(&row.name)
                .map_err(|e| InfraError::Unexpected(e.to_string()))?,
            description: row.description,
            version:     Version::new(row.version as u32)
                .map_err(|e| InfraError::Unexpected(e.to_string()))?,
            definition:  row.definition,
            status:      row.status.parse::<WorkflowDefinitionStatus>()
                .map_err(|e| InfraError::Unexpected(e.to_string()))?,
            created_by:  UserId::from_uuid(row.created_by),
            created_at:  row.created_at,
            updated_at:  row.updated_at,
        }))
    }
}
```

#### 確認事項
- [ ] `WorkflowDefinitionRecord` のフィールド → domain 層で確認
- [ ] `WorkflowDefinitionStatus` の parse パターン → 既存コード L121-124 で確認済み
- [ ] 参照パターン: `workflow_step_repository.rs` L79-135 で確認済み

#### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

統合テスト:
- [ ] 既存の workflow_definition_repository テストがリグレッションなく通ること

E2E テスト（該当なし）

削減効果: 1 クローン、約 16 行削減

---

### Phase 4: Permission パース/変換の共通化

対象ファイル:
- `backend/crates/infra/src/repository/role_repository.rs`（parse_permissions を pub(crate) に + permissions_to_json 抽出）
- `backend/crates/infra/src/repository/user_repository.rs`（parse_permissions 呼び出しに変更）

変更内容:

1. `role_repository.rs` L68 の `fn parse_permissions` を `pub(crate) fn parse_permissions` に変更
2. `role_repository.rs` に `pub(crate) fn permissions_to_json` ヘルパーを追加:
   ```rust
   /// Permission の Vec を JSONB 用の serde_json::Value に変換する
   pub(crate) fn permissions_to_json(permissions: &[Permission]) -> serde_json::Value {
       serde_json::Value::Array(
           permissions
               .iter()
               .map(|p| serde_json::Value::String(p.as_str().to_string()))
               .collect(),
       )
   }
   ```
3. `role_repository.rs` L169-174 と L197-202 を `permissions_to_json(role.permissions())` に置換
4. `user_repository.rs` L274-282 と L667-675 のインラインパースを `parse_permissions(row.permissions)` に置換
   - インポート追加: `use crate::repository::role_repository::parse_permissions;`

#### 確認事項
- [ ] `parse_permissions` のシグネチャ → `fn parse_permissions(permissions: serde_json::Value) -> Vec<Permission>` (L68-78 で確認済み)
- [ ] `user_repository.rs` の `row.permissions` の型 → serde_json::Value（sqlx::query! の匿名型フィールド）
- [ ] `Permission::new` のシグネチャ → Grep で確認
- [ ] `pub(crate)` のインポートパス → `crate::repository::role_repository::parse_permissions`

#### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

統合テスト:
- [ ] 既存の role_repository テストがリグレッションなく通ること
- [ ] 既存の user_repository テストがリグレッションなく通ること

E2E テスト（該当なし）

削減効果: 3 クローン、約 23 行削減

---

### Phase 5: User 行マッピングの共通化

対象ファイル: `backend/crates/infra/src/repository/user_repository.rs`

変更内容:
1. `UserRow` 構造体を定義（workflow_step_repository.rs のパターンに従う）
2. `TryFrom<UserRow> for User` を実装
3. 以下の関数で `sqlx::query!` → `sqlx::query_as!` に変更、インライン `User::from_db(...)` を `User::try_from(row)?` に置換:
   - `find_by_email`（L156-197）
   - `find_by_id`（L199-240）
   - `find_by_ids`（L315-346）の `.map()` 内
   - `find_all_active_by_tenant`（L360-392）の `.map()` 内
   - `find_by_display_number`（L476-509）
   - `find_all_by_tenant`（L514-592）の match 両分岐

```rust
/// DB の users テーブルの行を表す中間構造体
struct UserRow {
    id: Uuid,
    tenant_id: Uuid,
    display_number: i64,
    email: String,
    name: String,
    status: String,
    last_login_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<UserRow> for User {
    type Error = InfraError;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        Ok(User::from_db(
            UserId::from_uuid(row.id),
            TenantId::from_uuid(row.tenant_id),
            DisplayNumber::new(row.display_number)
                .map_err(|e| InfraError::Unexpected(e.to_string()))?,
            Email::new(&row.email)
                .map_err(|e| InfraError::Unexpected(e.to_string()))?,
            UserName::new(&row.name)
                .map_err(|e| InfraError::Unexpected(e.to_string()))?,
            row.status
                .parse::<UserStatus>()
                .map_err(|e| InfraError::Unexpected(e.to_string()))?,
            row.last_login_at,
            row.created_at,
            row.updated_at,
        ))
    }
}
```

変更例（find_by_email）:
```rust
async fn find_by_email(&self, tenant_id: &TenantId, email: &Email) -> Result<Option<User>, InfraError> {
    let row = sqlx::query_as!(
        UserRow,
        r#"SELECT id, tenant_id, display_number, email, name, status,
           last_login_at, created_at, updated_at
           FROM users WHERE tenant_id = $1 AND email = $2"#,
        tenant_id.as_uuid(),
        email.as_str()
    )
    .fetch_optional(&self.pool)
    .await?;

    row.map(User::try_from).transpose()
}
```

#### 確認事項
- [ ] `User::from_db` の全引数の型 → domain/src/user.rs で確認
- [ ] `sqlx::query_as!` の使用パターン → workflow_step_repository.rs L79-135 で確認済み
- [ ] 必要なインポート追加: `Uuid`, `DateTime<Utc>` → 既存のインポートを確認
- [ ] `find_all_by_tenant` の match 両分岐で UserRow が使えるか（SELECT列が同一か）→ 実装時に確認

#### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

統合テスト:
- [ ] 既存の user_repository テストがリグレッションなく通ること

E2E テスト（該当なし）

削減効果: 6 クローン、約 70 行削減

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `sqlx::query!` は proc macro であり `concat!` の結果を直接受け取れない | 技術的前提 | マクロの設計を変更: SQL 全文をリテラルとして渡す方式に |
| 2回目 | redis_session.rs の scan_count/scan_and_delete はSCANループが類似するが、async処理の差異があり共通化に async クロージャが必要 | シンプルさ | 対象外に分類。「意味のある差異」として記録 |
| 3回目 | `find_all_by_tenant` の match 分岐は SQL 重複が残るが、User::from_db 部分は Phase 5 で解消される | 曖昧 | 「SQLの重複は対象外（sqlx制約）、マッピングの重複は解消」と明記 |
| 4回目 | auth_credentials.rs がアクセス拒否で読めない（54行、他と同サイズ） | 不完全なパス | Phase 2 の確認事項に追加。実装時に構造を確認し、同構造なら統合 |
| 5回目 | Phase 4 と Phase 5 が共に user_repository.rs を変更するが、対象関数が異なる（Permission vs User行マッピング） | アーキテクチャ不整合 | 確認済み: Phase 4 は find_with_roles/find_role_by_name、Phase 5 はそれ以外の find 関数。独立してコミット可能 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue の 21 クローン全てに判断がある | OK | 14 対象 + 7 対象外（各理由付き）で 21 クローンを網羅 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 不確定要素は確認事項に明示（auth_credentials.rs の構造、sqlx+マクロの互換性） |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | 4つの設計判断に選択肢・理由を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象 14 / 対象外 7 を明示 |
| 5 | 技術的前提 | sqlx マクロの制約が考慮されている | OK | query!/query_as! のリテラル要件、concat! 非互換、macro_rules! 展開順序を考慮 |
| 6 | 既存ドキュメント整合 | 先行 Story のパターンと一致 | OK | workflow_step/instance の Row+TryFrom パターン、#527 の「過度な DRY を避ける」判断を尊重 |

## 検証方法

1. 各 Phase のコミット後: `cd backend && cargo test` で関連テスト通過を確認
2. 全 Phase 完了後: `just check-all` で全体リグレッションなしを確認
3. jscpd でクローン数を実測: `npx jscpd backend/crates/infra/src/ --min-lines 10 --min-tokens 50 --reporters console`
4. user_repository.rs の行数確認: `wc -l backend/crates/infra/src/repository/user_repository.rs`

## 削減効果の見積もり

| Phase | 削減クローン数 | 削減行数見積 |
|-------|--------------|-------------|
| Phase 1: session.rs SCAN | 1 | 約 20 行 |
| Phase 2: deletion/ マクロ | 3-4 | 約 60 行 |
| Phase 3: WorkflowDefinition Row | 1 | 約 16 行 |
| Phase 4: Permission 共通化 | 3 | 約 23 行 |
| Phase 5: User Row + TryFrom | 6 | 約 70 行 |
| **合計** | **14-15** | **約 189 行** |

Issue の 21 クローンに対して 14 クローン解消（67%）、7 クローン対象外（33%、sqlx制約 + 微小重複）。
