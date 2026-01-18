//! # 認証ユースケース
//!
//! ログイン時の認証処理を担当する。
//!
//! ## 責務
//!
//! - メール/パスワードの検証
//! - ユーザーステータスの確認
//! - タイミング攻撃の防止
//!
//! 詳細: [認証機能設計](../../../../docs/03_詳細設計書/07_認証機能設計.md)

use ringiflow_domain::{
   password::{PasswordHash, PlainPassword},
   role::Role,
   tenant::TenantId,
   user::{Email, User},
};
use ringiflow_infra::{InfraError, PasswordChecker, repository::user_repository::UserRepository};
use thiserror::Error;

/// 認証エラー
#[derive(Debug, Error)]
pub enum AuthError {
   /// 認証失敗（メール不存在、パスワード不一致、非アクティブ）
   ///
   /// セキュリティ上、詳細な理由は外部に公開しない
   #[error("認証に失敗しました")]
   AuthenticationFailed,

   /// インフラ層エラー（DB 接続エラーなど）
   #[error("内部エラー: {0}")]
   Internal(#[from] InfraError),
}

/// 認証ユースケース
///
/// UserRepository と PasswordChecker を使って認証処理を行う。
pub struct AuthUseCase<R, P>
where
   R: UserRepository,
   P: PasswordChecker,
{
   user_repository:  R,
   password_checker: P,
}

impl<R, P> AuthUseCase<R, P>
where
   R: UserRepository,
   P: PasswordChecker,
{
   /// 新しいインスタンスを作成する
   pub fn new(user_repository: R, password_checker: P) -> Self {
      Self {
         user_repository,
         password_checker,
      }
   }

   /// UserRepository への参照を取得する
   pub fn user_repository(&self) -> &R {
      &self.user_repository
   }

   /// 認証情報を検証し、ユーザーとロールを返す
   ///
   /// # 引数
   ///
   /// - `tenant_id`: テナント ID
   /// - `email`: メールアドレス
   /// - `password`: パスワード
   ///
   /// # 戻り値
   ///
   /// - `Ok((User, Vec<Role>))`: 認証成功
   /// - `Err(AuthError::AuthenticationFailed)`: 認証失敗
   ///
   /// # セキュリティ
   ///
   /// - ユーザーが存在しない場合もダミーのパスワード検証を行い、タイミング攻撃を防ぐ
   /// - 認証失敗時は詳細な理由を返さない
   pub async fn verify_credentials(
      &self,
      tenant_id: &TenantId,
      email: &Email,
      password: &PlainPassword,
   ) -> Result<(User, Vec<Role>), AuthError> {
      // ユーザーを取得
      let user_result = self.user_repository.find_by_email(tenant_id, email).await?;

      // ユーザーが存在しない場合、タイミング攻撃対策としてダミー検証を実行
      let Some(user) = user_result else {
         // ダミーハッシュで検証（処理時間を均一化）
         let dummy_hash = PasswordHash::new(
            "$argon2id$v=19$m=65536,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
         );
         let _ = self.password_checker.verify(password, &dummy_hash);
         return Err(AuthError::AuthenticationFailed);
      };

      // パスワードを検証
      let password_hash = user
         .password_hash()
         .map(PasswordHash::new)
         .ok_or(AuthError::AuthenticationFailed)?;

      let verify_result = self.password_checker.verify(password, &password_hash)?;

      if verify_result.is_mismatch() {
         return Err(AuthError::AuthenticationFailed);
      }

      // ユーザーステータスを確認
      if !user.can_login() {
         return Err(AuthError::AuthenticationFailed);
      }

      // ロールを取得
      let (user, roles) = self
         .user_repository
         .find_with_roles(user.id())
         .await?
         .ok_or(AuthError::AuthenticationFailed)?;

      // 最終ログイン日時を更新（エラーは無視してログイン自体は成功させる）
      if let Err(e) = self.user_repository.update_last_login(user.id()).await {
         tracing::warn!("最終ログイン日時の更新に失敗: {}", e);
      }

      Ok((user, roles))
   }
}

#[cfg(test)]
mod tests {
   use async_trait::async_trait;
   use ringiflow_domain::{
      password::PasswordVerifyResult,
      role::{Permission, Role},
      tenant::TenantId,
      user::{Email, User, UserId, UserStatus},
      value_objects::UserName,
   };

   use super::*;

   // テスト用のスタブ実装

   /// スタブ UserRepository
   struct StubUserRepository {
      user: Option<User>,
      user_with_roles: Option<(User, Vec<Role>)>,
   }

   impl StubUserRepository {
      fn with_user(user: User, roles: Vec<Role>) -> Self {
         Self {
            user: Some(user.clone()),
            user_with_roles: Some((user, roles)),
         }
      }

      fn empty() -> Self {
         Self {
            user: None,
            user_with_roles: None,
         }
      }
   }

   #[async_trait]
   impl UserRepository for StubUserRepository {
      async fn find_by_email(
         &self,
         _tenant_id: &TenantId,
         _email: &Email,
      ) -> Result<Option<User>, InfraError> {
         Ok(self.user.clone())
      }

      async fn find_by_id(&self, _id: &UserId) -> Result<Option<User>, InfraError> {
         Ok(self.user.clone())
      }

      async fn find_with_roles(
         &self,
         _id: &UserId,
      ) -> Result<Option<(User, Vec<Role>)>, InfraError> {
         Ok(self.user_with_roles.clone())
      }

      async fn update_last_login(&self, _id: &UserId) -> Result<(), InfraError> {
         Ok(())
      }
   }

   /// スタブ PasswordChecker
   struct StubPasswordChecker {
      result: PasswordVerifyResult,
   }

   impl StubPasswordChecker {
      fn matching() -> Self {
         Self {
            result: PasswordVerifyResult::Match,
         }
      }

      fn mismatching() -> Self {
         Self {
            result: PasswordVerifyResult::Mismatch,
         }
      }
   }

   impl PasswordChecker for StubPasswordChecker {
      fn verify(
         &self,
         _password: &PlainPassword,
         _hash: &PasswordHash,
      ) -> Result<PasswordVerifyResult, InfraError> {
         Ok(self.result)
      }
   }

   // テストデータ生成

   fn create_active_user(tenant_id: &TenantId) -> User {
      User::from_db(
         UserId::new(),
         tenant_id.clone(),
         Email::new("user@example.com").unwrap(),
         UserName::new("Test User").unwrap(),
         Some("$argon2id$v=19$m=65536,t=1,p=1$...".to_string()),
         UserStatus::Active,
         None,
         chrono::Utc::now(),
         chrono::Utc::now(),
      )
   }

   fn create_inactive_user(tenant_id: &TenantId) -> User {
      User::from_db(
         UserId::new(),
         tenant_id.clone(),
         Email::new("inactive@example.com").unwrap(),
         UserName::new("Inactive User").unwrap(),
         Some("$argon2id$v=19$m=65536,t=1,p=1$...".to_string()),
         UserStatus::Inactive,
         None,
         chrono::Utc::now(),
         chrono::Utc::now(),
      )
   }

   fn create_user_role() -> Role {
      Role::new_system(
         "user".to_string(),
         Some("一般ユーザー".to_string()),
         vec![
            Permission::new("workflow:read"),
            Permission::new("task:read"),
         ],
      )
   }

   // テストケース

   #[tokio::test]
   async fn test_正しい認証情報でユーザーを取得できる() {
      // Given
      let tenant_id = TenantId::new();
      let user = create_active_user(&tenant_id);
      let roles = vec![create_user_role()];
      let repo = StubUserRepository::with_user(user.clone(), roles.clone());
      let checker = StubPasswordChecker::matching();
      let usecase = AuthUseCase::new(repo, checker);

      let email = Email::new("user@example.com").unwrap();
      let password = PlainPassword::new("password123");

      // When
      let result = usecase
         .verify_credentials(&tenant_id, &email, &password)
         .await;

      // Then
      assert!(result.is_ok());
      let (returned_user, returned_roles) = result.unwrap();
      assert_eq!(returned_user.email().as_str(), "user@example.com");
      assert_eq!(returned_roles.len(), 1);
      assert_eq!(returned_roles[0].name(), "user");
   }

   #[tokio::test]
   async fn test_不正なパスワードで認証失敗() {
      // Given
      let tenant_id = TenantId::new();
      let user = create_active_user(&tenant_id);
      let roles = vec![create_user_role()];
      let repo = StubUserRepository::with_user(user, roles);
      let checker = StubPasswordChecker::mismatching();
      let usecase = AuthUseCase::new(repo, checker);

      let email = Email::new("user@example.com").unwrap();
      let password = PlainPassword::new("wrongpassword");

      // When
      let result = usecase
         .verify_credentials(&tenant_id, &email, &password)
         .await;

      // Then
      assert!(result.is_err());
      assert!(matches!(
         result.unwrap_err(),
         AuthError::AuthenticationFailed
      ));
   }

   #[tokio::test]
   async fn test_存在しないユーザーで認証失敗() {
      // Given
      let tenant_id = TenantId::new();
      let repo = StubUserRepository::empty();
      let checker = StubPasswordChecker::matching();
      let usecase = AuthUseCase::new(repo, checker);

      let email = Email::new("notfound@example.com").unwrap();
      let password = PlainPassword::new("password123");

      // When
      let result = usecase
         .verify_credentials(&tenant_id, &email, &password)
         .await;

      // Then
      assert!(result.is_err());
      assert!(matches!(
         result.unwrap_err(),
         AuthError::AuthenticationFailed
      ));
   }

   #[tokio::test]
   async fn test_非アクティブユーザーは認証失敗() {
      // Given
      let tenant_id = TenantId::new();
      let user = create_inactive_user(&tenant_id);
      let roles = vec![create_user_role()];
      let repo = StubUserRepository::with_user(user, roles);
      let checker = StubPasswordChecker::matching();
      let usecase = AuthUseCase::new(repo, checker);

      let email = Email::new("inactive@example.com").unwrap();
      let password = PlainPassword::new("password123");

      // When
      let result = usecase
         .verify_credentials(&tenant_id, &email, &password)
         .await;

      // Then
      assert!(result.is_err());
      assert!(matches!(
         result.unwrap_err(),
         AuthError::AuthenticationFailed
      ));
   }
}
