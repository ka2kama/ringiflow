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

use async_trait::async_trait;
use ringiflow_domain::{
   role::{Permission, Role, RoleId},
   tenant::TenantId,
   user::{Email, User, UserId, UserStatus},
   value_objects::{DisplayNumber, UserName},
};
use sqlx::PgPool;

use crate::error::InfraError;

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
   async fn find_all_active_by_tenant(&self, tenant_id: &TenantId)
   -> Result<Vec<User>, InfraError>;

   /// 最終ログイン日時を更新
   async fn update_last_login(&self, id: &UserId) -> Result<(), InfraError>;
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
            let permissions: Vec<Permission> = row
               .permissions
               .as_array()
               .map(|arr| {
                  arr.iter()
                     .filter_map(|v| v.as_str().map(Permission::new))
                     .collect()
               })
               .unwrap_or_default();

            Role::from_db(
               RoleId::from_uuid(row.id),
               row.tenant_id.map(TenantId::from_uuid),
               row.name,
               row.description,
               permissions,
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

      rows
         .into_iter()
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

      rows
         .into_iter()
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
