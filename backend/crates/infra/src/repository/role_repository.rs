//! # RoleRepository
//!
//! ロール情報の永続化を担当するリポジトリ。
//!
//! ## 設計方針
//!
//! - **UserRepository との分離**: SRP に基づき、ロール CRUD
//!   は独立トレイトで定義
//! - **system_admin の除外**: テナント管理画面の一覧取得では system_admin
//!   を除外
//! - **RLS 二重防御**: WHERE 句で明示的にテナント条件を指定
//!
//! 詳細: [認証機能設計](../../../../docs/03_詳細設計書/07_認証機能設計.md)

use async_trait::async_trait;
use ringiflow_domain::{
    role::{Permission, Role, RoleId},
    tenant::TenantId,
};
use sqlx::PgPool;

use crate::error::InfraError;

/// ロールリポジトリトレイト
///
/// ロールの CRUD 操作を定義する。
/// ユーザーとロールの関連（user_roles）は UserRepository が担当する。
#[async_trait]
pub trait RoleRepository: Send + Sync {
    /// テナントで利用可能なロール一覧をユーザー数付きで取得する
    ///
    /// システムロール（tenant_id = NULL）とテナント固有ロールの両方を返す。
    /// system_admin ロールは除外する。
    async fn find_all_by_tenant_with_user_count(
        &self,
        tenant_id: &TenantId,
    ) -> Result<Vec<(Role, i64)>, InfraError>;

    /// ID でロールを検索する
    async fn find_by_id(&self, id: &RoleId) -> Result<Option<Role>, InfraError>;

    /// ロールを挿入する
    async fn insert(&self, role: &Role) -> Result<(), InfraError>;

    /// ロールを更新する（名前、説明、権限）
    async fn update(&self, role: &Role) -> Result<(), InfraError>;

    /// ロールを削除する
    async fn delete(&self, id: &RoleId) -> Result<(), InfraError>;

    /// ロールに割り当てられたユーザー数をカウントする
    async fn count_users_with_role(&self, role_id: &RoleId) -> Result<i64, InfraError>;
}

/// PostgreSQL 実装の RoleRepository
#[derive(Debug, Clone)]
pub struct PostgresRoleRepository {
    pool: PgPool,
}

impl PostgresRoleRepository {
    /// 新しいリポジトリインスタンスを作成
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// JSONB から Permission の Vec に変換するヘルパー
fn parse_permissions(permissions: serde_json::Value) -> Vec<Permission> {
    permissions
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(Permission::new))
                .collect()
        })
        .unwrap_or_default()
}

#[async_trait]
impl RoleRepository for PostgresRoleRepository {
    async fn find_all_by_tenant_with_user_count(
        &self,
        tenant_id: &TenantId,
    ) -> Result<Vec<(Role, i64)>, InfraError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                r.id,
                r.tenant_id,
                r.name as "name!",
                r.description,
                r.permissions as "permissions!",
                r.is_system as "is_system!",
                r.created_at as "created_at!",
                r.updated_at as "updated_at!",
                COUNT(ur.id)::bigint as "user_count!"
            FROM roles r
            LEFT JOIN user_roles ur ON r.id = ur.role_id AND ur.tenant_id = $1
            WHERE (r.tenant_id = $1 OR r.is_system = true)
              AND r.name != 'system_admin'
            GROUP BY r.id
            ORDER BY r.is_system DESC, r.name ASC
            "#,
            tenant_id.as_uuid()
        )
        .fetch_all(&self.pool)
        .await?;

        let results = rows
            .into_iter()
            .map(|row| {
                let permissions = parse_permissions(row.permissions);
                let role = Role::from_db(
                    RoleId::from_uuid(row.id),
                    row.tenant_id.map(TenantId::from_uuid),
                    row.name,
                    row.description,
                    permissions,
                    row.is_system,
                    row.created_at,
                    row.updated_at,
                );
                (role, row.user_count)
            })
            .collect();

        Ok(results)
    }

    async fn find_by_id(&self, id: &RoleId) -> Result<Option<Role>, InfraError> {
        let row = sqlx::query!(
            r#"
            SELECT
                id,
                tenant_id,
                name as "name!",
                description,
                permissions as "permissions!",
                is_system as "is_system!",
                created_at as "created_at!",
                updated_at as "updated_at!"
            FROM roles
            WHERE id = $1
            "#,
            id.as_uuid()
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let permissions = parse_permissions(row.permissions);
        Ok(Some(Role::from_db(
            RoleId::from_uuid(row.id),
            row.tenant_id.map(TenantId::from_uuid),
            row.name,
            row.description,
            permissions,
            row.is_system,
            row.created_at,
            row.updated_at,
        )))
    }

    async fn insert(&self, role: &Role) -> Result<(), InfraError> {
        let permissions_json = serde_json::Value::Array(
            role.permissions()
                .iter()
                .map(|p| serde_json::Value::String(p.as_str().to_string()))
                .collect(),
        );

        sqlx::query!(
         r#"
            INSERT INTO roles (id, tenant_id, name, description, permissions, is_system, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
         role.id().as_uuid(),
         role.tenant_id().map(|t| *t.as_uuid()),
         role.name(),
         role.description(),
         permissions_json,
         role.is_system(),
         role.created_at(),
         role.updated_at()
      )
      .execute(&self.pool)
      .await?;

        Ok(())
    }

    async fn update(&self, role: &Role) -> Result<(), InfraError> {
        let permissions_json = serde_json::Value::Array(
            role.permissions()
                .iter()
                .map(|p| serde_json::Value::String(p.as_str().to_string()))
                .collect(),
        );

        sqlx::query!(
            r#"
            UPDATE roles
            SET name = $2, description = $3, permissions = $4, updated_at = $5
            WHERE id = $1
            "#,
            role.id().as_uuid(),
            role.name(),
            role.description(),
            permissions_json,
            role.updated_at()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete(&self, id: &RoleId) -> Result<(), InfraError> {
        sqlx::query!(
            r#"
            DELETE FROM roles
            WHERE id = $1
            "#,
            id.as_uuid()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn count_users_with_role(&self, role_id: &RoleId) -> Result<i64, InfraError> {
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*)::bigint as "count!"
            FROM user_roles
            WHERE role_id = $1
            "#,
            role_id.as_uuid()
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_トレイトはsendとsyncを実装している() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PostgresRoleRepository>();
    }
}
