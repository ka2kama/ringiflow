//! RLS（Row Level Security）統合テスト
//!
//! PostgreSQL RLS ポリシーの動作を検証し、クロステナントアクセスが
//! 確実に防止されることをテストする。
//!
//! テスト方式:
//! - `#[sqlx::test]` で独立した DB 環境を作成
//! - superuser でテストデータを INSERT
//! - `SET ROLE ringiflow_app` で非 superuser に切り替え
//! - `set_config('app.tenant_id', ...)` でテナントコンテキストを設定
//! - RLS ポリシーによるフィルタリングを検証
//!
//! 実行方法:
//! ```bash
//! just dev-deps
//! cd backend && cargo test -p ringiflow-infra --test rls_test
//! ```

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

// =============================================================================
// テストデータ
// =============================================================================

/// 2テナント分のテストデータ ID
#[allow(dead_code)]
struct TwoTenantFixture {
    tenant_a:     Uuid,
    tenant_b:     Uuid,
    user_a:       Uuid,
    user_b:       Uuid,
    definition_a: Uuid,
    definition_b: Uuid,
    instance_a:   Uuid,
    instance_b:   Uuid,
}

/// 2テナント分のテストデータを全テーブルに INSERT（superuser で実行）
///
/// RLS テストに必要な全 9 テーブルにデータを投入する。
/// FK 依存順に INSERT する。
async fn setup_two_tenants(pool: &PgPool) -> TwoTenantFixture {
    let tenant_a = Uuid::now_v7();
    let tenant_b = Uuid::now_v7();
    let user_a = Uuid::now_v7();
    let user_b = Uuid::now_v7();
    let definition_a = Uuid::now_v7();
    let definition_b = Uuid::now_v7();
    let instance_a = Uuid::now_v7();
    let instance_b = Uuid::now_v7();

    // 1. tenants
    sqlx::query("INSERT INTO tenants (id, name, subdomain, plan, status) VALUES ($1, 'Tenant A', 'tenant-a', 'free', 'active')")
        .bind(tenant_a)
        .execute(pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO tenants (id, name, subdomain, plan, status) VALUES ($1, 'Tenant B', 'tenant-b', 'free', 'active')")
        .bind(tenant_b)
        .execute(pool)
        .await
        .unwrap();

    // 2. users
    sqlx::query("INSERT INTO users (id, tenant_id, display_number, email, name, status) VALUES ($1, $2, 1, 'user-a@example.com', 'User A', 'active')")
        .bind(user_a)
        .bind(tenant_a)
        .execute(pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO users (id, tenant_id, display_number, email, name, status) VALUES ($1, $2, 1, 'user-b@example.com', 'User B', 'active')")
        .bind(user_b)
        .bind(tenant_b)
        .execute(pool)
        .await
        .unwrap();

    // 3. roles（テナント固有ロール）
    // system roles はシードデータで作成済み
    let role_a = Uuid::now_v7();
    let role_b = Uuid::now_v7();

    sqlx::query("INSERT INTO roles (id, tenant_id, name, description, permissions, is_system) VALUES ($1, $2, 'custom_role', 'テナントAカスタムロール', '[\"custom:read\"]', false)")
        .bind(role_a)
        .bind(tenant_a)
        .execute(pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO roles (id, tenant_id, name, description, permissions, is_system) VALUES ($1, $2, 'custom_role', 'テナントBカスタムロール', '[\"custom:read\"]', false)")
        .bind(role_b)
        .bind(tenant_b)
        .execute(pool)
        .await
        .unwrap();

    // 4. user_roles（system 'user' ロールを割り当て）
    sqlx::query("INSERT INTO user_roles (user_id, role_id, tenant_id) SELECT $1, id, $2 FROM roles WHERE name = 'user' AND is_system = true")
        .bind(user_a)
        .bind(tenant_a)
        .execute(pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO user_roles (user_id, role_id, tenant_id) SELECT $1, id, $2 FROM roles WHERE name = 'user' AND is_system = true")
        .bind(user_b)
        .bind(tenant_b)
        .execute(pool)
        .await
        .unwrap();

    // 5. workflow_definitions
    let definition_json = serde_json::json!({
        "form": { "fields": [] },
        "steps": [
            {"id": "start", "type": "start", "name": "開始"},
            {"id": "end", "type": "end", "name": "完了", "status": "approved"}
        ],
        "transitions": [{"from": "start", "to": "end"}]
    });

    sqlx::query("INSERT INTO workflow_definitions (id, tenant_id, name, description, version, definition, status, created_by) VALUES ($1, $2, 'テスト定義A', '説明', 1, $3, 'published', $4)")
        .bind(definition_a)
        .bind(tenant_a)
        .bind(&definition_json)
        .bind(user_a)
        .execute(pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO workflow_definitions (id, tenant_id, name, description, version, definition, status, created_by) VALUES ($1, $2, 'テスト定義B', '説明', 1, $3, 'published', $4)")
        .bind(definition_b)
        .bind(tenant_b)
        .bind(&definition_json)
        .bind(user_b)
        .execute(pool)
        .await
        .unwrap();

    // 6. workflow_instances
    let form_data = serde_json::json!({});

    sqlx::query("INSERT INTO workflow_instances (id, tenant_id, definition_id, definition_version, display_number, title, form_data, status, initiated_by) VALUES ($1, $2, $3, 1, 100, 'テスト申請A', $4, 'pending', $5)")
        .bind(instance_a)
        .bind(tenant_a)
        .bind(definition_a)
        .bind(&form_data)
        .bind(user_a)
        .execute(pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO workflow_instances (id, tenant_id, definition_id, definition_version, display_number, title, form_data, status, initiated_by) VALUES ($1, $2, $3, 1, 100, 'テスト申請B', $4, 'pending', $5)")
        .bind(instance_b)
        .bind(tenant_b)
        .bind(definition_b)
        .bind(&form_data)
        .bind(user_b)
        .execute(pool)
        .await
        .unwrap();

    // 7. workflow_steps
    sqlx::query("INSERT INTO workflow_steps (id, instance_id, tenant_id, display_number, step_id, step_name, step_type, status, assigned_to) VALUES ($1, $2, $3, 1, 'start', '開始', 'start', 'completed', $4)")
        .bind(Uuid::now_v7())
        .bind(instance_a)
        .bind(tenant_a)
        .bind(user_a)
        .execute(pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO workflow_steps (id, instance_id, tenant_id, display_number, step_id, step_name, step_type, status, assigned_to) VALUES ($1, $2, $3, 1, 'start', '開始', 'start', 'completed', $4)")
        .bind(Uuid::now_v7())
        .bind(instance_b)
        .bind(tenant_b)
        .bind(user_b)
        .execute(pool)
        .await
        .unwrap();

    // 8. display_id_counters
    sqlx::query("INSERT INTO display_id_counters (tenant_id, entity_type, last_number) VALUES ($1, 'workflow_instance', 100)")
        .bind(tenant_a)
        .execute(pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO display_id_counters (tenant_id, entity_type, last_number) VALUES ($1, 'workflow_instance', 100)")
        .bind(tenant_b)
        .execute(pool)
        .await
        .unwrap();

    // 9. auth.credentials
    sqlx::query("INSERT INTO auth.credentials (user_id, tenant_id, credential_type, credential_data) VALUES ($1, $2, 'password', '$argon2id$hash_a')")
        .bind(user_a)
        .bind(tenant_a)
        .execute(pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO auth.credentials (user_id, tenant_id, credential_type, credential_data) VALUES ($1, $2, 'password', '$argon2id$hash_b')")
        .bind(user_b)
        .bind(tenant_b)
        .execute(pool)
        .await
        .unwrap();

    TwoTenantFixture {
        tenant_a,
        tenant_b,
        user_a,
        user_b,
        definition_a,
        definition_b,
        instance_a,
        instance_b,
    }
}

// =============================================================================
// ヘルパー関数
// =============================================================================

/// ringiflow_app ロールに切り替え、テナントコンテキストを設定
async fn set_tenant_context(conn: &mut PgConnection, tenant_id: &Uuid) {
    sqlx::query("SET ROLE ringiflow_app")
        .execute(&mut *conn)
        .await
        .unwrap();
    sqlx::query("SELECT set_config('app.tenant_id', $1, false)")
        .bind(tenant_id.to_string())
        .execute(&mut *conn)
        .await
        .unwrap();
}

/// テナントコンテキストなしで ringiflow_app ロールに切り替え
async fn set_app_role_without_tenant(conn: &mut PgConnection) {
    sqlx::query("SET ROLE ringiflow_app")
        .execute(&mut *conn)
        .await
        .unwrap();
}

/// ロールを superuser にリセット
async fn reset_role(conn: &mut PgConnection) {
    sqlx::query("RESET ROLE").execute(&mut *conn).await.unwrap();
}

// =============================================================================
// tenants テーブル
// =============================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn test_tenants_テナントaのコンテキストで自テナントのみ取得できる(
    pool: PgPool,
) {
    let fixture = setup_two_tenants(&pool).await;

    let mut conn = pool.acquire().await.unwrap();
    set_tenant_context(&mut conn, &fixture.tenant_a).await;

    let rows: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM tenants")
        .fetch_all(&mut *conn)
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, fixture.tenant_a);

    reset_role(&mut conn).await;
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_tenants_tenant_id未設定時にデータが返らない(pool: PgPool) {
    let _fixture = setup_two_tenants(&pool).await;

    let mut conn = pool.acquire().await.unwrap();
    set_app_role_without_tenant(&mut conn).await;

    let rows: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM tenants")
        .fetch_all(&mut *conn)
        .await
        .unwrap();

    assert!(rows.is_empty());

    reset_role(&mut conn).await;
}

// =============================================================================
// users テーブル
// =============================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn test_users_テナントaのコンテキストで自テナントのユーザーのみ取得できる(
    pool: PgPool,
) {
    let fixture = setup_two_tenants(&pool).await;

    let mut conn = pool.acquire().await.unwrap();
    set_tenant_context(&mut conn, &fixture.tenant_a).await;

    let rows: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM users")
        .fetch_all(&mut *conn)
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, fixture.user_a);

    reset_role(&mut conn).await;
}

// =============================================================================
// roles テーブル
// =============================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn test_roles_テナントaのコンテキストで自テナントロールとsystem_rolesが取得できる(
    pool: PgPool,
) {
    let fixture = setup_two_tenants(&pool).await;

    let mut conn = pool.acquire().await.unwrap();
    set_tenant_context(&mut conn, &fixture.tenant_a).await;

    let rows: Vec<(Uuid, Option<Uuid>, bool)> =
        sqlx::query_as("SELECT id, tenant_id, is_system FROM roles")
            .fetch_all(&mut *conn)
            .await
            .unwrap();

    // system roles（tenant_id IS NULL）+ テナント A のカスタムロール
    let system_count = rows.iter().filter(|r| r.2).count();
    let tenant_count = rows.iter().filter(|r| !r.2).count();

    // system roles は 3 つ（system_admin, tenant_admin, user）
    assert_eq!(system_count, 3);
    // テナント A のカスタムロールは 1 つ
    assert_eq!(tenant_count, 1);

    // テナント B のロールは含まれない
    for row in &rows {
        if let Some(tid) = row.1 {
            assert_eq!(tid, fixture.tenant_a);
        }
    }

    reset_role(&mut conn).await;
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_roles_テナント固有ロールはクロステナントアクセスできない(
    pool: PgPool,
) {
    let fixture = setup_two_tenants(&pool).await;

    // テナント B のコンテキストでクエリ
    let mut conn = pool.acquire().await.unwrap();
    set_tenant_context(&mut conn, &fixture.tenant_b).await;

    let rows: Vec<(Uuid, Option<Uuid>, bool)> =
        sqlx::query_as("SELECT id, tenant_id, is_system FROM roles")
            .fetch_all(&mut *conn)
            .await
            .unwrap();

    // テナント固有ロールはテナント B のもののみ
    for row in &rows {
        if let Some(tid) = row.1 {
            assert_eq!(tid, fixture.tenant_b);
        }
    }

    reset_role(&mut conn).await;
}

// =============================================================================
// user_roles テーブル
// =============================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn test_user_roles_テナントaのコンテキストで自テナントのuser_rolesのみ取得できる(
    pool: PgPool,
) {
    let fixture = setup_two_tenants(&pool).await;

    let mut conn = pool.acquire().await.unwrap();
    set_tenant_context(&mut conn, &fixture.tenant_a).await;

    let rows: Vec<(Uuid,)> = sqlx::query_as("SELECT user_id FROM user_roles")
        .fetch_all(&mut *conn)
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, fixture.user_a);

    reset_role(&mut conn).await;
}

// =============================================================================
// workflow_definitions テーブル
// =============================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn test_workflow_definitions_テナントaのコンテキストで自テナントの定義のみ取得できる(
    pool: PgPool,
) {
    let fixture = setup_two_tenants(&pool).await;

    let mut conn = pool.acquire().await.unwrap();
    set_tenant_context(&mut conn, &fixture.tenant_a).await;

    let rows: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM workflow_definitions")
        .fetch_all(&mut *conn)
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, fixture.definition_a);

    reset_role(&mut conn).await;
}

// =============================================================================
// workflow_instances テーブル
// =============================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn test_workflow_instances_テナントaのコンテキストで自テナントのインスタンスのみ取得できる(
    pool: PgPool,
) {
    let fixture = setup_two_tenants(&pool).await;

    let mut conn = pool.acquire().await.unwrap();
    set_tenant_context(&mut conn, &fixture.tenant_a).await;

    let rows: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM workflow_instances")
        .fetch_all(&mut *conn)
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, fixture.instance_a);

    reset_role(&mut conn).await;
}

// =============================================================================
// workflow_steps テーブル
// =============================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn test_workflow_steps_テナントaのコンテキストで自テナントのステップのみ取得できる(
    pool: PgPool,
) {
    let fixture = setup_two_tenants(&pool).await;

    let mut conn = pool.acquire().await.unwrap();
    set_tenant_context(&mut conn, &fixture.tenant_a).await;

    let rows: Vec<(Uuid, Uuid)> = sqlx::query_as("SELECT id, instance_id FROM workflow_steps")
        .fetch_all(&mut *conn)
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, fixture.instance_a);

    reset_role(&mut conn).await;
}

// =============================================================================
// display_id_counters テーブル
// =============================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn test_display_id_counters_テナントaのコンテキストで自テナントのカウンターのみ取得できる(
    pool: PgPool,
) {
    let fixture = setup_two_tenants(&pool).await;

    let mut conn = pool.acquire().await.unwrap();
    set_tenant_context(&mut conn, &fixture.tenant_a).await;

    let rows: Vec<(Uuid,)> = sqlx::query_as("SELECT tenant_id FROM display_id_counters")
        .fetch_all(&mut *conn)
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, fixture.tenant_a);

    reset_role(&mut conn).await;
}

// =============================================================================
// auth.credentials テーブル
// =============================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn test_auth_credentials_テナントaのコンテキストで自テナントのcredentialsのみ取得できる(
    pool: PgPool,
) {
    let fixture = setup_two_tenants(&pool).await;

    let mut conn = pool.acquire().await.unwrap();
    set_tenant_context(&mut conn, &fixture.tenant_a).await;

    let rows: Vec<(Uuid,)> = sqlx::query_as("SELECT user_id FROM auth.credentials")
        .fetch_all(&mut *conn)
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, fixture.user_a);

    reset_role(&mut conn).await;
}

// =============================================================================
// WITH CHECK テスト（INSERT 制約）
// =============================================================================

#[sqlx::test(migrations = "../../migrations")]
async fn test_with_check_テナントaのコンテキストでテナントbのデータをinsertできない(
    pool: PgPool,
) {
    let fixture = setup_two_tenants(&pool).await;

    let mut conn = pool.acquire().await.unwrap();
    set_tenant_context(&mut conn, &fixture.tenant_a).await;

    // テナント A のコンテキストでテナント B の tenant_id を持つユーザーを INSERT
    let result = sqlx::query(
      "INSERT INTO users (id, tenant_id, display_number, email, name, status) VALUES ($1, $2, 99, 'cross@example.com', 'Cross', 'active')",
   )
   .bind(Uuid::now_v7())
   .bind(fixture.tenant_b)
   .execute(&mut *conn)
   .await;

    // RLS WITH CHECK 違反でエラーになる
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("row-level security"),
        "Expected RLS violation error, got: {err_msg}"
    );

    reset_role(&mut conn).await;
}

// =============================================================================
// TenantConnection 統合テスト
// =============================================================================

/// TenantConnection が本番で行う操作（set_config）と SET ROLE の組み合わせで
/// テナント分離が機能することを検証する。
///
/// 本番では ringiflow_app ロールで接続するため SET ROLE は不要だが、
/// テスト環境では superuser プールを使うため SET ROLE でシミュレーションする。
#[sqlx::test(migrations = "../../migrations")]
async fn test_tenant_connection_set_roleとset_configでテナント分離が機能する(
    pool: PgPool,
) {
    let fixture = setup_two_tenants(&pool).await;

    // TenantConnection が本番で行う操作をシミュレーション:
    // 1. SET ROLE ringiflow_app（本番ではプール接続ユーザーが担当）
    // 2. set_config('app.tenant_id', ...)（TenantConnection::acquire が担当）
    let mut conn = pool.acquire().await.unwrap();
    sqlx::query("SET ROLE ringiflow_app")
        .execute(&mut *conn)
        .await
        .unwrap();
    sqlx::query("SELECT set_config('app.tenant_id', $1, false)")
        .bind(fixture.tenant_a.to_string())
        .execute(&mut *conn)
        .await
        .unwrap();

    // テナント A のデータのみ取得できる
    let rows: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM users")
        .fetch_all(&mut *conn)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, fixture.user_a);

    // テナントコンテキストを切り替え（after_release 相当 + 再設定）
    sqlx::query("SELECT set_config('app.tenant_id', $1, false)")
        .bind(fixture.tenant_b.to_string())
        .execute(&mut *conn)
        .await
        .unwrap();

    // テナント B のデータのみ取得できる
    let rows: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM users")
        .fetch_all(&mut *conn)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, fixture.user_b);

    reset_role(&mut conn).await;
}
