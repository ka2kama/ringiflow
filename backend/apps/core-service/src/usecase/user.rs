//! ユーザー管理ユースケース

use std::sync::Arc;

use ringiflow_domain::{
    clock::Clock,
    role::{Role, RoleId},
    tenant::TenantId,
    user::{Email, User, UserId, UserStatus},
    value_objects::{DisplayIdEntityType, UserName},
};
use ringiflow_infra::repository::{DisplayIdCounterRepository, UserRepository};

use crate::error::CoreError;

/// ユーザー作成の入力
pub struct CreateUserInput {
    pub tenant_id: TenantId,
    pub email:     Email,
    pub name:      UserName,
    pub role_id:   RoleId,
}

/// ユーザー更新の入力
pub struct UpdateUserInput {
    pub user_id: UserId,
    pub name:    Option<UserName>,
    pub role_id: Option<RoleId>,
}

/// ユーザーステータス変更の入力
pub struct UpdateUserStatusInput {
    pub user_id:      UserId,
    pub tenant_id:    TenantId,
    pub status:       UserStatus,
    pub requester_id: UserId,
}

/// ユーザー管理ユースケース
pub struct UserUseCaseImpl {
    user_repository: Arc<dyn UserRepository>,
    display_id_counter_repository: Arc<dyn DisplayIdCounterRepository>,
    clock: Arc<dyn Clock>,
}

impl UserUseCaseImpl {
    pub fn new(
        user_repository: Arc<dyn UserRepository>,
        display_id_counter_repository: Arc<dyn DisplayIdCounterRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            user_repository,
            display_id_counter_repository,
            clock,
        }
    }

    /// ユーザーを作成する
    ///
    /// 1. メールアドレスの重複チェック
    /// 2. ロールの存在確認
    /// 3. display_number 採番
    /// 4. User ドメインオブジェクト作成
    /// 5. users テーブルに挿入
    /// 6. user_roles テーブルにロール割り当て
    pub async fn create_user(&self, input: CreateUserInput) -> Result<(User, Role), CoreError> {
        // メールアドレスの重複チェック
        if let Some(_existing) = self
            .user_repository
            .find_by_email(&input.tenant_id, &input.email)
            .await?
        {
            return Err(CoreError::Conflict(
                "このメールアドレスは既に使用されています".to_string(),
            ));
        }

        // ロールの存在確認
        let role = self
            .user_repository
            .find_role_by_id(&input.role_id)
            .await?
            .ok_or_else(|| {
                CoreError::BadRequest(format!("ロール ID '{}' が見つかりません", input.role_id))
            })?;

        // display_number 採番
        let display_number = self
            .display_id_counter_repository
            .next_display_number(&input.tenant_id, DisplayIdEntityType::User)
            .await
            .map_err(|e| CoreError::Internal(format!("採番に失敗: {}", e)))?;

        // User ドメインオブジェクト作成
        let now = self.clock.now();
        let user = User::new(
            UserId::new(),
            input.tenant_id.clone(),
            display_number,
            input.email,
            input.name,
            now,
        );

        // users テーブルに挿入
        self.user_repository.insert(&user).await?;

        // user_roles テーブルにロール割り当て
        self.user_repository
            .insert_user_role(user.id(), role.id(), &input.tenant_id)
            .await?;

        Ok((user, role))
    }

    /// ユーザー情報を更新する（名前、ロール）
    pub async fn update_user(&self, input: UpdateUserInput) -> Result<User, CoreError> {
        let user = self
            .user_repository
            .find_by_id(&input.user_id)
            .await?
            .ok_or_else(|| CoreError::NotFound("ユーザーが見つかりません".to_string()))?;

        let now = self.clock.now();

        // 名前の更新
        let user = if let Some(name) = input.name {
            let updated = user.with_name(name, now);
            self.user_repository.update(&updated).await?;
            updated
        } else {
            user
        };

        // ロールの更新
        if let Some(role_id) = input.role_id {
            let role = self
                .user_repository
                .find_role_by_id(&role_id)
                .await?
                .ok_or_else(|| {
                    CoreError::BadRequest(format!("ロール ID '{}' が見つかりません", role_id))
                })?;

            self.user_repository
                .replace_user_roles(user.id(), role.id(), user.tenant_id())
                .await?;
        }

        Ok(user)
    }

    /// ユーザーステータスを変更する
    ///
    /// - 自己無効化防止（requester_id == target_id のチェック）
    /// - 最後のテナント管理者保護
    pub async fn update_user_status(
        &self,
        input: UpdateUserStatusInput,
    ) -> Result<User, CoreError> {
        // 自己無効化防止
        if input.status != UserStatus::Active && input.requester_id == input.user_id {
            return Err(CoreError::BadRequest(
                "自分自身を無効化することはできません".to_string(),
            ));
        }

        let user = self
            .user_repository
            .find_by_id(&input.user_id)
            .await?
            .ok_or_else(|| CoreError::NotFound("ユーザーが見つかりません".to_string()))?;

        // 最後のテナント管理者保護
        if input.status != UserStatus::Active {
            let admin_count = self
                .user_repository
                .count_active_users_with_role(
                    &input.tenant_id,
                    "tenant_admin",
                    Some(&input.user_id),
                )
                .await?;

            if admin_count == 0 {
                // 対象ユーザーが tenant_admin かチェック
                let (_, roles) = self
                    .user_repository
                    .find_with_roles(&input.user_id)
                    .await?
                    .ok_or_else(|| CoreError::NotFound("ユーザーが見つかりません".to_string()))?;

                let is_admin = roles.iter().any(|r| r.name() == "tenant_admin");
                if is_admin {
                    return Err(CoreError::BadRequest(
                        "最後のテナント管理者を無効化することはできません".to_string(),
                    ));
                }
            }
        }

        let now = self.clock.now();
        let updated = user.with_status(input.status, now);
        self.user_repository.update_status(&updated).await?;

        Ok(updated)
    }
}
