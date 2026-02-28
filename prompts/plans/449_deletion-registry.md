# Issue #449: 削除レジストリ基盤の実装計画

## Context

テナント退会時のデータ削除を安全に行うための削除レジストリ基盤が未実装。設計書（`docs/40_詳細設計書/06_テナント退会時データ削除設計.md`）は策定済みだが、実装コードが存在しない。`RedisSessionManager::delete_all_for_tenant()` のみ既存。

## スコープ

### 対象

- `TenantDeleter` トレイト + `DeletionResult` 型
- `DeletionRegistry` 構造体
- PostgreSQL Deleter x4: users, roles, workflows, display_id_counters
- Auth Credentials Deleter x1
- DynamoDB Audit Log Deleter x1
- Redis Session Deleter x1
- 統合テスト（PostgreSQL Deleter + Auth Credentials）
- 登録漏れ検出テスト
- `data-store.md` のパス更新

### 対象外

- テナント退会フロー（Phase 1〜3）
- 削除マニフェスト、削除後検証（verify_all）
- S3 Deleter（S3 未導入）
- DynamoDB/Redis の統合テスト（別 Issue）
- `tenants` テーブル自体の削除

## 設計判断

### TenantDeleter トレイトの配置: infra 層

既存の全リポジトリトレイト（`UserRepository`, `AuditLogRepository`, `SessionManager` 等）が infra 層に定義されている。domain 層にはトレイトが一つも存在しない。一貫性を優先し infra 層に配置する。

設計書は domain 層を示唆するが、プロジェクトの実態と既存パターンを優先する。

### PostgreSQL Deleter の粒度: Issue スコープに従った4グループ

| Deleter | 対象テーブル | 削除方法 |
|---------|-----------|---------|
| `PostgresUserDeleter` | users | `DELETE FROM users WHERE tenant_id = $1`（user_roles は CASCADE） |
| `PostgresRoleDeleter` | roles | `DELETE FROM roles WHERE tenant_id = $1` |
| `PostgresWorkflowDeleter` | workflow_definitions, workflow_instances, workflow_steps | tenant_id で直接 DELETE（子テーブルから順に） |
| `PostgresDisplayIdCounterDeleter` | display_id_counters | `DELETE FROM display_id_counters WHERE tenant_id = $1` |

### Redis Deleter: ConnectionManager を直接保持

`RedisSessionManager` には `count` メソッドがなく、`TenantDeleter` のインターフェースに合わせるには独自実装が必要。`ConnectionManager` を直接保持し、`session.rs` の SCAN + DEL パターンを踏襲する。

## ファイル配置

```
backend/crates/infra/src/
├── deletion/                        # 新規モジュール
│   ├── mod.rs                       # TenantDeleter トレイト + DeletionResult + re-export
│   ├── registry.rs                  # DeletionRegistry
│   ├── postgres_user.rs             # PostgresUserDeleter
│   ├── postgres_role.rs             # PostgresRoleDeleter
│   ├── postgres_workflow.rs         # PostgresWorkflowDeleter
│   ├── postgres_display_id.rs       # PostgresDisplayIdCounterDeleter
│   ├── auth_credentials.rs          # AuthCredentialsDeleter
│   ├── dynamodb_audit_log.rs        # DynamoDbAuditLogDeleter
│   └── redis_session.rs             # RedisSessionDeleter
└── lib.rs                           # pub mod deletion を追加

backend/crates/infra/tests/
├── deletion_registry_test.rs        # 登録漏れ検出テスト
└── postgres_deleter_test.rs         # PostgreSQL + Auth Deleter 統合テスト
```

---

## Phase 1: TenantDeleter トレイト + DeletionResult + DeletionRegistry 骨格

### 確認事項

- [x] 型: `TenantId` → `tenant.rs` L75, Newtype(Uuid), `as_uuid()` / `from_uuid()` / `new()`
- [x] 型: `InfraError` → `error.rs` L18, enum (Database, Redis, Serialization, Conflict, DynamoDb, Unexpected)
- [x] パターン: `async_trait` + `Send + Sync` → `user_repository.rs` L30-31, `#[async_trait] pub trait Xxx: Send + Sync`
- [x] パターン: `lib.rs` のモジュール公開 → `lib.rs` L52-60, `pub mod module_name;`

### テストリスト

- [ ] `DeletionRegistry` を空で生成でき、`registered_names()` が空 Vec を返す
- [ ] `DeletionRegistry` に Deleter を登録でき、`registered_names()` で名前を取得できる
- [ ] `DeletionRegistry::delete_all()` が全 Deleter の `delete()` を呼び、結果を返す
- [ ] `DeletionRegistry::count_all()` が全 Deleter の `count()` を呼び、結果を返す

### 実装方針

**`deletion/mod.rs`**:

```rust
#[async_trait]
pub trait TenantDeleter: Send + Sync {
    fn name(&self) -> &'static str;
    async fn delete(&self, tenant_id: &TenantId) -> Result<DeletionResult, InfraError>;
    async fn count(&self, tenant_id: &TenantId) -> Result<u64, InfraError>;
}

#[derive(Debug, Clone)]
pub struct DeletionResult {
    pub deleted_count: u64,
}
```

**`deletion/registry.rs`**: `Vec<Box<dyn TenantDeleter>>` を保持。`delete_all`, `count_all`, `registered_names` メソッド。テストはモック Deleter で `#[cfg(test)]`。

---

## Phase 2: PostgreSQL Deleter 実装（4つ）+ Auth Credentials Deleter

### 確認事項

- [x] パターン: `PgPool` 保持 → `user_repository.rs` L136-138, `pool: PgPool` + `Self { pool }`
- [x] パターン: `sqlx::query!` DELETE + `rows_affected()` → `workflow_step_repository.rs` L219
- [x] 型: `auth.credentials` スキーマ → `20260122000001`, tenant_id UUID NOT NULL（FK なし、サービス境界独立）
- [x] パターン: CASCADE → users/roles/workflow_*/display_id_counters → tenants ON DELETE CASCADE, user_roles → users ON DELETE CASCADE
- [x] パターン: workflow_instances.definition_id → definitions(id) CASCADE なし → instances → steps の順で削除

### テストリスト（統合テスト `#[sqlx::test]`）

- [ ] `PostgresUserDeleter::count()` がテナントのユーザー数を返す
- [ ] `PostgresUserDeleter::delete()` がテナントのユーザーを削除し、件数を返す
- [ ] `PostgresUserDeleter::delete()` 後に `count()` が 0 を返す
- [ ] 他テナントのユーザーは削除されない
- [ ] `PostgresRoleDeleter` の count + delete が正しく動作する
- [ ] `PostgresWorkflowDeleter` の count + delete が正しく動作する（子テーブルも削除される）
- [ ] `PostgresDisplayIdCounterDeleter` の count + delete が正しく動作する
- [ ] `AuthCredentialsDeleter` の count + delete が正しく動作する
- [ ] `AuthCredentialsDeleter` で他テナントの credentials は削除されない

### 実装方針

各 Deleter は `PgPool` を保持する構造体。`delete()` で `DELETE FROM {table} WHERE tenant_id = $1` を実行し、`rows_affected()` を `DeletionResult` にマッピング。`count()` で `SELECT COUNT(*) ... WHERE tenant_id = $1`。

`PostgresWorkflowDeleter` は workflow_steps → workflow_instances → workflow_definitions の順で DELETE（FK 制約の安全順序）。count は workflow_definitions の件数。

---

## Phase 3: DynamoDB Audit Log Deleter

### 確認事項

- [x] 型: `DynamoDbAuditLogRepository` → `audit_log_repository.rs` L59-62, `Client` + `table_name: String`
- [x] パターン: DynamoDB Query → `audit_log_repository.rs` L141-250, key_condition + expression_attribute_values
- [x] ライブラリ: `batch_write_item` → Grep 未使用、AWS SDK 標準 API（25件制限）

### テストリスト

- [ ] `DynamoDbAuditLogDeleter::name()` が `"dynamodb:audit_logs"` を返す
- [ ] `DynamoDbAuditLogDeleter` が `Send + Sync` を満たす（コンパイル時テスト）

DynamoDB Local を使った統合テストは別 Issue。

### 実装方針

`Client` + `table_name` を保持。`count()`: Query with `Select::COUNT`。`delete()`: Query で PK/SK を取得し、`BatchWriteItem` で 25 件ずつ DeleteRequest。ページネーション対応。

---

## Phase 4: Redis Session Deleter

### 確認事項

- [x] 型: `ConnectionManager` → `session.rs` L23, `redis::aio::ConnectionManager`
- [x] パターン: SCAN + DEL → `session.rs` L327-346, SCAN cursor + MATCH pattern + DEL
- [x] パターン: CSRF 削除 → `session.rs` L411-430, 同じ SCAN + DEL パターン

### テストリスト

- [ ] `RedisSessionDeleter::name()` が `"redis:sessions"` を返す
- [ ] `RedisSessionDeleter` が `Send + Sync` を満たす（コンパイル時テスト）

Redis 統合テストは別 Issue。

### 実装方針

`ConnectionManager` を保持。session + csrf の2パターンを SCAN + DEL。`count()` は SCAN でキー数をカウント。`session.rs` の既存パターンを踏襲。

---

## Phase 5: DeletionRegistry ファクトリ + 登録漏れ検出テスト

### 確認事項

確認事項: なし（Phase 1-4 で確認済み）

### テストリスト

- [ ] 登録漏れ検出: 期待リストと `registered_names()` が完全一致する
- [ ] 期待リストに未登録の名前があればテスト失敗
- [ ] 期待リストにない名前が登録されていればテスト失敗

### 実装方針

`DeletionRegistry::with_all_deleters(pg_pool, dynamodb_client, table_name, redis_conn)` ファクトリメソッド。

登録漏れ検出テストは DB 不要で実行するため、`expected_deleter_names()` を定数関数として提供し、ファクトリメソッドの登録と対応させる。テストは `backend/crates/infra/tests/deletion_registry_test.rs`。

期待リスト:

```
"postgres:users", "postgres:roles", "postgres:workflows",
"postgres:display_id_counters", "auth:credentials",
"dynamodb:audit_logs", "redis:sessions"
```

---

## Phase 6: lib.rs 更新 + data-store.md パス修正 + check-all

### テストリスト

- [ ] `just check-all` がパスする

### 実装内容

- `backend/crates/infra/src/lib.rs` に `pub mod deletion;` を追加
- `.claude/rules/data-store.md` のパスを実装に合わせて更新
- `just sqlx-prepare` でキャッシュ更新

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `data-store.md` のパスが `apps/core-service/src/domain/tenant/deletion/` だが infra 層に配置する判断と不整合 | アーキテクチャ不整合 | Phase 6 で data-store.md のパスを更新 |
| 2回目 | 登録漏れ検出テストに DB 接続が不要な設計が必要 | 未定義 | `expected_deleter_names()` 定数関数を提供し、DB 不要テストを実現 |
| 3回目 | `TenantDeleter::delete()` の引数型（値渡し vs 参照渡し）が未定義 | 曖昧 | 既存パターン（`&TenantId` 参照渡し）に統一 |
| 4回目 | PostgresWorkflowDeleter の DELETE 順序が FK 制約に影響する可能性 | 不完全なパス | 子テーブルから順に削除（steps → instances → definitions）に確定 |
| 5回目 | Phase 2 と Phase 3（Auth Credentials）を分離する必要性が低い | シンプルさ | Auth Credentials を Phase 2 に統合（同じ PgPool パターン） |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 探索で発見された全対象が計画に含まれている | OK | Issue スコープの全データストア（PostgreSQL 4グループ + auth.credentials + DynamoDB audit_logs + Redis sessions）を網羅。S3 は未導入のため対象外として明記 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 全 Deleter の配置・削除方法・count 対象・テスト方針が確定済み |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | トレイト配置（infra）、Deleter 粒度、Redis 実装方式の判断に理由あり |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象（7 Deleter + Registry + テスト）と対象外（退会フロー、マニフェスト、S3 等）を明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | CASCADE 方向（全テーブル → tenants ON DELETE CASCADE）、DynamoDB BatchWriteItem 25件制限、Redis SCAN パターンを確認済み |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | 設計書の Deleter 名に準拠。data-store.md のパスは Phase 6 で更新 |

## 検証方法

```bash
just check-all                      # lint + test + API test + E2E test
cd backend && cargo test -p ringiflow-infra --test postgres_deleter_test
cd backend && cargo test -p ringiflow-infra --test deletion_registry_test
```
