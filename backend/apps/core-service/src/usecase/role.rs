//! ロール管理ユースケース

use std::sync::Arc;

use ringiflow_domain::{
   clock::Clock,
   role::{Permission, Role, RoleId},
   tenant::TenantId,
};
use ringiflow_infra::repository::RoleRepository;

use crate::error::CoreError;

/// ロール作成の入力
pub struct CreateRoleInput {
   pub tenant_id:   TenantId,
   pub name:        String,
   pub description: Option<String>,
   pub permissions: Vec<String>,
}

/// ロール更新の入力
pub struct UpdateRoleInput {
   pub role_id:     RoleId,
   pub name:        Option<String>,
   pub description: Option<String>,
   pub permissions: Option<Vec<String>>,
}

/// ロール管理ユースケース
pub struct RoleUseCaseImpl {
   role_repository: Arc<dyn RoleRepository>,
   clock:           Arc<dyn Clock>,
}

impl RoleUseCaseImpl {
   pub fn new(role_repository: Arc<dyn RoleRepository>, clock: Arc<dyn Clock>) -> Self {
      Self {
         role_repository,
         clock,
      }
   }

   /// カスタムロールを作成する
   ///
   /// 1. 権限リストが空でないことを検証
   /// 2. Role ドメインオブジェクト作成
   /// 3. DB に挿入（重複名は DB 制約でエラー）
   pub async fn create_role(&self, input: CreateRoleInput) -> Result<Role, CoreError> {
      // 権限が空でないことを検証
      if input.permissions.is_empty() {
         return Err(CoreError::BadRequest(
            "権限を1つ以上指定してください".to_string(),
         ));
      }

      let now = self.clock.now();
      let permissions: Vec<Permission> =
         input.permissions.into_iter().map(Permission::new).collect();

      let role = Role::new_tenant(
         RoleId::new(),
         input.tenant_id,
         input.name,
         input.description,
         permissions,
         now,
      );

      self.role_repository.insert(&role).await.map_err(|e| {
         // UNIQUE 制約違反（tenant_id, name）の場合は Conflict
         if let ringiflow_infra::InfraError::Database(ref db_err) = e {
            if let Some(constraint) = db_err.as_database_error().and_then(|d| d.constraint()) {
               if constraint == "roles_tenant_name_key" {
                  return CoreError::Conflict("同名のロールが既に存在します".to_string());
               }
            }
         }
         CoreError::Database(e)
      })?;

      Ok(role)
   }

   /// カスタムロールを更新する
   ///
   /// - システムロールは編集拒否
   /// - 更新するフィールドのみ変更
   pub async fn update_role(&self, input: UpdateRoleInput) -> Result<Role, CoreError> {
      let role = self
         .role_repository
         .find_by_id(&input.role_id)
         .await?
         .ok_or_else(|| CoreError::NotFound("ロールが見つかりません".to_string()))?;

      // システムロールは編集不可
      if role.is_system() {
         return Err(CoreError::BadRequest(
            "システムロールは編集できません".to_string(),
         ));
      }

      let now = self.clock.now();

      // 各フィールドを更新
      let role = if let Some(name) = input.name {
         role.with_name(name, now)
      } else {
         role
      };

      let role = if let Some(description) = input.description {
         role.with_description(Some(description), now)
      } else {
         role
      };

      let role = if let Some(permissions) = input.permissions {
         if permissions.is_empty() {
            return Err(CoreError::BadRequest(
               "権限を1つ以上指定してください".to_string(),
            ));
         }
         let perms: Vec<Permission> = permissions.into_iter().map(Permission::new).collect();
         role.with_permissions(perms, now)
      } else {
         role
      };

      self.role_repository.update(&role).await?;

      Ok(role)
   }

   /// カスタムロールを削除する
   ///
   /// - システムロールは削除拒否
   /// - ユーザー割り当てありは削除拒否
   pub async fn delete_role(&self, role_id: &RoleId) -> Result<(), CoreError> {
      let role = self
         .role_repository
         .find_by_id(role_id)
         .await?
         .ok_or_else(|| CoreError::NotFound("ロールが見つかりません".to_string()))?;

      // システムロールは削除不可
      if role.is_system() {
         return Err(CoreError::BadRequest(
            "システムロールは削除できません".to_string(),
         ));
      }

      // ユーザー割り当てチェック
      let user_count = self.role_repository.count_users_with_role(role_id).await?;
      if user_count > 0 {
         return Err(CoreError::Conflict(format!(
            "このロールは {} 人のユーザーに割り当てられているため削除できません",
            user_count
         )));
      }

      self.role_repository.delete(role_id).await?;

      Ok(())
   }
}
