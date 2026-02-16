//! # UserRepository
//!
//! ユーザー情報の永続化を担当するリポジトリ。
//!
//! ## 設計方針
//!
//! - **テナント分離**: すべてのクエリでテナント ID を考慮
//! - **型安全なクエリ**: sqlx のコンパイル時検証を活用
//! - **ロールの一括取得**: N+1 問題を避けるため JOIN で取得
//!
//! 詳細: [認証機能設計](../../../../docs/03_詳細設計書/07_認証機能設計.md)

use std::collections::HashMap;

use async_trait::async_trait;
use ringiflow_domain::{
    role::{Role, RoleId},
    tenant::TenantId,
    user::{Email, User, UserId, UserStatus},
    value_objects::{DisplayNumber, UserName},
};
use sqlx::PgPool;

use crate::{error::InfraError, repository::role_repository::parse_permissions};

/// ユーザーリポジトリトレイト
///
/// ユーザー情報の永続化操作を定義する。
/// インフラ層で具体的な実装を提供し、ユースケース層から利用する。
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// メールアドレスでユーザーを検索（テナント内）
    ///
    /// # 引数
    ///
    /// - `tenant_id`: テナント ID
    /// - `email`: 検索するメールアドレス
    ///
    /// # 戻り値
    ///
    /// - `Ok(Some(user))`: ユーザーが見つかった場合
    /// - `Ok(None)`: ユーザーが見つからない場合
    /// - `Err(_)`: データベースエラー
    async fn find_by_email(
        &self,
        tenant_id: &TenantId,
        email: &Email,
    ) -> Result<Option<User>, InfraError>;

    /// ID でユーザーを検索
    ///
    /// テナント ID は検証しない（内部 API 向け）。
    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, InfraError>;

    /// ユーザーをロール付きで取得
    ///
    /// ユーザー情報と、そのユーザーに割り当てられたロールを一括で取得する。
    async fn find_with_roles(&self, id: &UserId) -> Result<Option<(User, Vec<Role>)>, InfraError>;

    /// 複数の ID でユーザーを一括検索
    ///
    /// 存在しない ID は無視し、見つかったユーザーのみ返す。
    /// 空の配列を渡した場合は空の Vec を返す。
    async fn find_by_ids(&self, ids: &[UserId]) -> Result<Vec<User>, InfraError>;

    /// テナント内のアクティブユーザー一覧を取得
    ///
    /// ステータスが `active` のユーザーのみを返す。
    /// 承認者選択などで使用する。
    async fn find_all_active_by_tenant(
        &self,
        tenant_id: &TenantId,
    ) -> Result<Vec<User>, InfraError>;

    /// 最終ログイン日時を更新
    async fn update_last_login(&self, id: &UserId) -> Result<(), InfraError>;

    /// ユーザーを挿入する
    async fn insert(&self, user: &User) -> Result<(), InfraError>;

    /// ユーザー情報を更新する（name, updated_at）
    async fn update(&self, user: &User) -> Result<(), InfraError>;

    /// ユーザーステータスを更新する
    async fn update_status(&self, user: &User) -> Result<(), InfraError>;

    /// 表示用連番でユーザーを検索する
    async fn find_by_display_number(
        &self,
        tenant_id: &TenantId,
        display_number: DisplayNumber,
    ) -> Result<Option<User>, InfraError>;

    /// テナント内のユーザー一覧を取得する（deleted 除外、ステータスフィルタ可）
    async fn find_all_by_tenant(
        &self,
        tenant_id: &TenantId,
        status_filter: Option<UserStatus>,
    ) -> Result<Vec<User>, InfraError>;

    /// ユーザーにロールを割り当てる
    async fn insert_user_role(
        &self,
        user_id: &UserId,
        role_id: &RoleId,
        tenant_id: &TenantId,
    ) -> Result<(), InfraError>;

    /// ユーザーのロールを置き換える（既存削除 + 新規追加）
    async fn replace_user_roles(
        &self,
        user_id: &UserId,
        role_id: &RoleId,
        tenant_id: &TenantId,
    ) -> Result<(), InfraError>;

    /// ロール名でロールを検索する
    async fn find_role_by_name(&self, name: &str) -> Result<Option<Role>, InfraError>;

    /// テナント内の特定ロールを持つアクティブユーザー数をカウントする
    async fn count_active_users_with_role(
        &self,
        tenant_id: &TenantId,
        role_name: &str,
        excluding_user_id: Option<&UserId>,
    ) -> Result<i64, InfraError>;

    /// ユーザー ID のリストに対応するロール名を一括取得する
    async fn find_roles_for_users(
        &self,
        user_ids: &[UserId],
        tenant_id: &TenantId,
    ) -> Result<HashMap<UserId, Vec<String>>, InfraError>;
}

/// PostgreSQL 実装の UserRepository
#[derive(Debug, Clone)]
pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    /// 新しいリポジトリインスタンスを作成
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn find_by_email(
        &self,
        tenant_id: &TenantId,
        email: &Email,
    ) -> Result<Option<User>, InfraError> {
        let row = sqlx::query!(
            r#"
            SELECT
                id,
                tenant_id,
                display_number,
                email,
                name,
                status,
                last_login_at,
                created_at,
                updated_at
            FROM users
            WHERE tenant_id = $1 AND email = $2
            "#,
            tenant_id.as_uuid(),
            email.as_str()
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let user = User::from_db(
            UserId::from_uuid(row.id),
            TenantId::from_uuid(row.tenant_id),
            DisplayNumber::new(row.display_number)
                .map_err(|e| InfraError::Unexpected(e.to_string()))?,
            Email::new(&row.email).map_err(|e| InfraError::Unexpected(e.to_string()))?,
            UserName::new(&row.name).map_err(|e| InfraError::Unexpected(e.to_string()))?,
            row.status
                .parse::<UserStatus>()
                .map_err(|e| InfraError::Unexpected(e.to_string()))?,
            row.last_login_at,
            row.created_at,
            row.updated_at,
        );

        Ok(Some(user))
    }

    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, InfraError> {
        let row = sqlx::query!(
            r#"
            SELECT
                id,
                tenant_id,
                display_number,
                email,
                name,
                status,
                last_login_at,
                created_at,
                updated_at
            FROM users
            WHERE id = $1
            "#,
            id.as_uuid()
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let user = User::from_db(
            UserId::from_uuid(row.id),
            TenantId::from_uuid(row.tenant_id),
            DisplayNumber::new(row.display_number)
                .map_err(|e| InfraError::Unexpected(e.to_string()))?,
            Email::new(&row.email).map_err(|e| InfraError::Unexpected(e.to_string()))?,
            UserName::new(&row.name).map_err(|e| InfraError::Unexpected(e.to_string()))?,
            row.status
                .parse::<UserStatus>()
                .map_err(|e| InfraError::Unexpected(e.to_string()))?,
            row.last_login_at,
            row.created_at,
            row.updated_at,
        );

        Ok(Some(user))
    }

    async fn find_with_roles(&self, id: &UserId) -> Result<Option<(User, Vec<Role>)>, InfraError> {
        // ユーザーを取得
        let Some(user) = self.find_by_id(id).await? else {
            return Ok(None);
        };

        // ロールを取得（JOIN で一括）
        // tenant_id フィルタで RLS 二重防御（user_roles テーブルに tenant_id あり）
        let role_rows = sqlx::query!(
            r#"
            SELECT
                r.id,
                r.tenant_id,
                r.name,
                r.description,
                r.permissions,
                r.is_system,
                r.created_at,
                r.updated_at
            FROM roles r
            INNER JOIN user_roles ur ON ur.role_id = r.id
            WHERE ur.user_id = $1 AND ur.tenant_id = $2
            "#,
            id.as_uuid(),
            user.tenant_id().as_uuid()
        )
        .fetch_all(&self.pool)
        .await?;

        let roles = role_rows
            .into_iter()
            .map(|row| {
                Role::from_db(
                    RoleId::from_uuid(row.id),
                    row.tenant_id.map(TenantId::from_uuid),
                    row.name,
                    row.description,
                    parse_permissions(row.permissions),
                    row.is_system,
                    row.created_at,
                    row.updated_at,
                )
            })
            .collect();

        Ok(Some((user, roles)))
    }

    async fn find_by_ids(&self, ids: &[UserId]) -> Result<Vec<User>, InfraError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let uuid_ids: Vec<uuid::Uuid> = ids.iter().map(|id| *id.as_uuid()).collect();

        let rows = sqlx::query!(
            r#"
            SELECT
                id,
                tenant_id,
                display_number,
                email,
                name,
                status,
                last_login_at,
                created_at,
                updated_at
            FROM users
            WHERE id = ANY($1)
            "#,
            &uuid_ids
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(User::from_db(
                    UserId::from_uuid(row.id),
                    TenantId::from_uuid(row.tenant_id),
                    DisplayNumber::new(row.display_number)
                        .map_err(|e| InfraError::Unexpected(e.to_string()))?,
                    Email::new(&row.email).map_err(|e| InfraError::Unexpected(e.to_string()))?,
                    UserName::new(&row.name).map_err(|e| InfraError::Unexpected(e.to_string()))?,
                    row.status
                        .parse::<UserStatus>()
                        .map_err(|e| InfraError::Unexpected(e.to_string()))?,
                    row.last_login_at,
                    row.created_at,
                    row.updated_at,
                ))
            })
            .collect()
    }

    async fn find_all_active_by_tenant(
        &self,
        tenant_id: &TenantId,
    ) -> Result<Vec<User>, InfraError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                id,
                tenant_id,
                display_number,
                email,
                name,
                status,
                last_login_at,
                created_at,
                updated_at
            FROM users
            WHERE tenant_id = $1 AND status = 'active'
            ORDER BY display_number
            "#,
            tenant_id.as_uuid()
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(User::from_db(
                    UserId::from_uuid(row.id),
                    TenantId::from_uuid(row.tenant_id),
                    DisplayNumber::new(row.display_number)
                        .map_err(|e| InfraError::Unexpected(e.to_string()))?,
                    Email::new(&row.email).map_err(|e| InfraError::Unexpected(e.to_string()))?,
                    UserName::new(&row.name).map_err(|e| InfraError::Unexpected(e.to_string()))?,
                    row.status
                        .parse::<UserStatus>()
                        .map_err(|e| InfraError::Unexpected(e.to_string()))?,
                    row.last_login_at,
                    row.created_at,
                    row.updated_at,
                ))
            })
            .collect()
    }

    async fn update_last_login(&self, id: &UserId) -> Result<(), InfraError> {
        sqlx::query!(
            r#"
            UPDATE users
            SET last_login_at = NOW()
            WHERE id = $1
            "#,
            id.as_uuid()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn insert(&self, user: &User) -> Result<(), InfraError> {
        let status: &str = user.status().into();
        sqlx::query!(
            r#"
            INSERT INTO users (
                id, tenant_id, display_number, email, name,
                status, last_login_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            user.id().as_uuid(),
            user.tenant_id().as_uuid(),
            user.display_number().as_i64(),
            user.email().as_str(),
            user.name().as_str(),
            status,
            user.last_login_at(),
            user.created_at(),
            user.updated_at()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update(&self, user: &User) -> Result<(), InfraError> {
        sqlx::query!(
            r#"
            UPDATE users
            SET name = $2, updated_at = $3
            WHERE id = $1
            "#,
            user.id().as_uuid(),
            user.name().as_str(),
            user.updated_at()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_status(&self, user: &User) -> Result<(), InfraError> {
        let status: &str = user.status().into();
        sqlx::query!(
            r#"
            UPDATE users
            SET status = $2, updated_at = $3
            WHERE id = $1
            "#,
            user.id().as_uuid(),
            status,
            user.updated_at()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_display_number(
        &self,
        tenant_id: &TenantId,
        display_number: DisplayNumber,
    ) -> Result<Option<User>, InfraError> {
        let row = sqlx::query!(
            r#"
            SELECT
                id, tenant_id, display_number, email, name,
                status, last_login_at, created_at, updated_at
            FROM users
            WHERE tenant_id = $1 AND display_number = $2
            "#,
            tenant_id.as_uuid(),
            display_number.as_i64()
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let user = User::from_db(
            UserId::from_uuid(row.id),
            TenantId::from_uuid(row.tenant_id),
            DisplayNumber::new(row.display_number)
                .map_err(|e| InfraError::Unexpected(e.to_string()))?,
            Email::new(&row.email).map_err(|e| InfraError::Unexpected(e.to_string()))?,
            UserName::new(&row.name).map_err(|e| InfraError::Unexpected(e.to_string()))?,
            row.status
                .parse::<UserStatus>()
                .map_err(|e| InfraError::Unexpected(e.to_string()))?,
            row.last_login_at,
            row.created_at,
            row.updated_at,
        );

        Ok(Some(user))
    }

    async fn find_all_by_tenant(
        &self,
        tenant_id: &TenantId,
        status_filter: Option<UserStatus>,
    ) -> Result<Vec<User>, InfraError> {
        // sqlx::query! は各呼び出しで異なる匿名 Record 型を生成するため、
        // match で統一できない。各ブランチで完結させる。
        match status_filter {
            Some(status) => {
                let status_str: &str = status.into();
                let rows = sqlx::query!(
                    r#"
                  SELECT
                      id, tenant_id, display_number, email, name,
                      status, last_login_at, created_at, updated_at
                  FROM users
                  WHERE tenant_id = $1 AND status = $2
                  ORDER BY display_number
                  "#,
                    tenant_id.as_uuid(),
                    status_str
                )
                .fetch_all(&self.pool)
                .await?;

                rows.into_iter()
                    .map(|row| {
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
                    })
                    .collect()
            }
            None => {
                let rows = sqlx::query!(
                    r#"
                  SELECT
                      id, tenant_id, display_number, email, name,
                      status, last_login_at, created_at, updated_at
                  FROM users
                  WHERE tenant_id = $1 AND status != 'deleted'
                  ORDER BY display_number
                  "#,
                    tenant_id.as_uuid()
                )
                .fetch_all(&self.pool)
                .await?;

                rows.into_iter()
                    .map(|row| {
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
                    })
                    .collect()
            }
        }
    }

    async fn insert_user_role(
        &self,
        user_id: &UserId,
        role_id: &RoleId,
        tenant_id: &TenantId,
    ) -> Result<(), InfraError> {
        sqlx::query!(
            r#"
            INSERT INTO user_roles (user_id, role_id, tenant_id)
            VALUES ($1, $2, $3)
            "#,
            user_id.as_uuid(),
            role_id.as_uuid(),
            tenant_id.as_uuid()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn replace_user_roles(
        &self,
        user_id: &UserId,
        role_id: &RoleId,
        tenant_id: &TenantId,
    ) -> Result<(), InfraError> {
        // 既存のロール割り当てを削除
        sqlx::query!(
            r#"
            DELETE FROM user_roles
            WHERE user_id = $1 AND tenant_id = $2
            "#,
            user_id.as_uuid(),
            tenant_id.as_uuid()
        )
        .execute(&self.pool)
        .await?;

        // 新しいロールを割り当て
        sqlx::query!(
            r#"
            INSERT INTO user_roles (user_id, role_id, tenant_id)
            VALUES ($1, $2, $3)
            "#,
            user_id.as_uuid(),
            role_id.as_uuid(),
            tenant_id.as_uuid()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_role_by_name(&self, name: &str) -> Result<Option<Role>, InfraError> {
        let row = sqlx::query!(
            r#"
            SELECT
                id, tenant_id, name, description, permissions,
                is_system, created_at, updated_at
            FROM roles
            WHERE name = $1
            "#,
            name
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(Role::from_db(
            RoleId::from_uuid(row.id),
            row.tenant_id.map(TenantId::from_uuid),
            row.name,
            row.description,
            parse_permissions(row.permissions),
            row.is_system,
            row.created_at,
            row.updated_at,
        )))
    }

    async fn count_active_users_with_role(
        &self,
        tenant_id: &TenantId,
        role_name: &str,
        excluding_user_id: Option<&UserId>,
    ) -> Result<i64, InfraError> {
        let count = match excluding_user_id {
            Some(exclude_id) => {
                sqlx::query_scalar!(
                    r#"
                  SELECT COUNT(*) as "count!"
                  FROM users u
                  INNER JOIN user_roles ur ON ur.user_id = u.id AND ur.tenant_id = u.tenant_id
                  INNER JOIN roles r ON r.id = ur.role_id
                  WHERE u.tenant_id = $1
                    AND u.status = 'active'
                    AND r.name = $2
                    AND u.id != $3
                  "#,
                    tenant_id.as_uuid(),
                    role_name,
                    exclude_id.as_uuid()
                )
                .fetch_one(&self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar!(
                    r#"
                  SELECT COUNT(*) as "count!"
                  FROM users u
                  INNER JOIN user_roles ur ON ur.user_id = u.id AND ur.tenant_id = u.tenant_id
                  INNER JOIN roles r ON r.id = ur.role_id
                  WHERE u.tenant_id = $1
                    AND u.status = 'active'
                    AND r.name = $2
                  "#,
                    tenant_id.as_uuid(),
                    role_name
                )
                .fetch_one(&self.pool)
                .await?
            }
        };

        Ok(count)
    }

    async fn find_roles_for_users(
        &self,
        user_ids: &[UserId],
        tenant_id: &TenantId,
    ) -> Result<HashMap<UserId, Vec<String>>, InfraError> {
        if user_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let uuid_ids: Vec<uuid::Uuid> = user_ids.iter().map(|id| *id.as_uuid()).collect();

        let rows = sqlx::query!(
            r#"
            SELECT
                ur.user_id,
                r.name
            FROM user_roles ur
            INNER JOIN roles r ON r.id = ur.role_id
            WHERE ur.user_id = ANY($1) AND ur.tenant_id = $2
            "#,
            &uuid_ids,
            tenant_id.as_uuid()
        )
        .fetch_all(&self.pool)
        .await?;

        let mut result: HashMap<UserId, Vec<String>> = HashMap::new();
        for row in rows {
            result
                .entry(UserId::from_uuid(row.user_id))
                .or_default()
                .push(row.name);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_トレイトはsendとsyncを実装している() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PostgresUserRepository>();
    }
}
