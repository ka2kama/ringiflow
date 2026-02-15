//! # 認証ユースケース
//!
//! Auth Service のビジネスロジックを実装する。
//!
//! ## タイミング攻撃対策
//!
//! パスワード検証では、ユーザーが存在しない場合もダミーハッシュで
//! 検証を実行し、処理時間を均一化する。
//!
//! 詳細: [08_AuthService設計.md](../../../../docs/03_詳細設計書/
//! 08_AuthService設計.md)

use std::sync::Arc;

use ringiflow_domain::{password::PlainPassword, tenant::TenantId, user::UserId};
use ringiflow_infra::{
    PasswordChecker,
    repository::{CredentialType, CredentialsRepository},
};
use uuid::Uuid;

use crate::error::AuthError;

/// パスワード検証結果
#[derive(Debug, Clone)]
pub struct VerifyResult {
    pub verified:      bool,
    pub credential_id: Option<Uuid>,
}

/// 認証ユースケースの実装
pub struct AuthUseCaseImpl {
    credentials_repository: Arc<dyn CredentialsRepository>,
    password_checker:       Arc<dyn PasswordChecker>,
}

impl AuthUseCaseImpl {
    /// 新しいユースケースインスタンスを作成
    pub fn new(
        credentials_repository: Arc<dyn CredentialsRepository>,
        password_checker: Arc<dyn PasswordChecker>,
    ) -> Self {
        Self {
            credentials_repository,
            password_checker,
        }
    }

    /// パスワードを検証する
    ///
    /// ## タイミング攻撃対策
    ///
    /// 認証情報が見つからない場合もダミーハッシュで検証を実行し、
    /// 処理時間を均一化する。
    pub async fn verify_password(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        password: &str,
    ) -> Result<VerifyResult, AuthError> {
        let tenant_id = TenantId::from_uuid(tenant_id);
        let user_id = UserId::from_uuid(user_id);
        let plain_password = PlainPassword::new(password);

        // 認証情報を取得
        let credential = self
            .credentials_repository
            .find_by_user_and_type(&tenant_id, &user_id, CredentialType::Password)
            .await?;

        match credential {
            Some(cred) if cred.is_active => {
                // パスワードを検証
                let hash = ringiflow_domain::password::PasswordHash::new(&cred.credential_data);
                let result = self.password_checker.verify(&plain_password, &hash);

                match result {
                    Ok(r) if r.is_match() => {
                        // 最終使用日時を更新
                        let _ = self.credentials_repository.update_last_used(cred.id).await;

                        Ok(VerifyResult {
                            verified:      true,
                            credential_id: Some(cred.id),
                        })
                    }
                    _ => Err(AuthError::AuthenticationFailed),
                }
            }
            Some(_) => {
                // 認証情報が無効
                // ダミー検証を実行して処理時間を均一化
                self.dummy_verification(&plain_password);
                Err(AuthError::CredentialInactive)
            }
            None => {
                // 認証情報が見つからない
                // タイミング攻撃対策: ダミーハッシュで検証を実行
                self.dummy_verification(&plain_password);
                Err(AuthError::AuthenticationFailed)
            }
        }
    }

    /// 認証情報を作成する
    ///
    /// パスワードの場合、平文をハッシュ化して保存する。
    /// 現状はハッシュ化済みの値を受け取る設計（Core API
    /// が作成時にハッシュ化）。
    pub async fn create_credential(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        credential_type: &str,
        credential_data: &str,
    ) -> Result<Uuid, AuthError> {
        let user_id = UserId::from_uuid(user_id);
        let tenant_id = TenantId::from_uuid(tenant_id);

        let cred_type: CredentialType = credential_type.parse().map_err(AuthError::Internal)?;

        let credential_id = self
            .credentials_repository
            .create(&user_id, &tenant_id, cred_type, credential_data)
            .await?;

        Ok(credential_id)
    }

    /// ユーザーの全認証情報を削除する
    pub async fn delete_credentials(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AuthError> {
        let tenant_id = TenantId::from_uuid(tenant_id);
        let user_id = UserId::from_uuid(user_id);
        self.credentials_repository
            .delete_by_user(&tenant_id, &user_id)
            .await?;
        Ok(())
    }

    /// ダミーハッシュで検証を実行する（タイミング攻撃対策）
    ///
    /// ユーザーが存在しない場合も実際のパスワード検証と同等の時間を消費する。
    /// 固定 sleep ではなく実際に Argon2id 検証を実行することで、
    /// CPU/メモリ状況による自然な変動も含めて同じ時間特性になる。
    fn dummy_verification(&self, password: &PlainPassword) {
        // ダミーハッシュ（有効な Argon2id 形式）
        let dummy_hash = ringiflow_domain::password::PasswordHash::new(
            "$argon2id$v=19$m=65536,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        );
        // 結果は無視（エラーでも問題ない）
        let _ = self.password_checker.verify(password, &dummy_hash);
    }
}

#[cfg(test)]
mod tests {
    use ringiflow_domain::password::{PasswordHash, PasswordVerifyResult};
    use ringiflow_infra::{InfraError, repository::Credential};

    use super::*;

    // テスト用スタブ

    struct StubCredentialsRepository {
        credential: Option<Credential>,
    }

    impl StubCredentialsRepository {
        fn with_active_credential(hash: &str) -> Self {
            Self {
                credential: Some(Credential {
                    id: Uuid::now_v7(),
                    user_id: UserId::new(),
                    tenant_id: TenantId::new(),
                    credential_type: CredentialType::Password,
                    credential_data: hash.to_string(),
                    is_active: true,
                    last_used_at: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }),
            }
        }

        fn with_inactive_credential() -> Self {
            Self {
                credential: Some(Credential {
                    id: Uuid::now_v7(),
                    user_id: UserId::new(),
                    tenant_id: TenantId::new(),
                    credential_type: CredentialType::Password,
                    credential_data: "dummy".to_string(),
                    is_active: false,
                    last_used_at: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }),
            }
        }

        fn empty() -> Self {
            Self { credential: None }
        }
    }

    #[async_trait::async_trait]
    impl CredentialsRepository for StubCredentialsRepository {
        async fn find_by_user_and_type(
            &self,
            _tenant_id: &TenantId,
            _user_id: &UserId,
            _credential_type: CredentialType,
        ) -> Result<Option<Credential>, InfraError> {
            Ok(self.credential.clone())
        }

        async fn create(
            &self,
            _user_id: &UserId,
            _tenant_id: &TenantId,
            _credential_type: CredentialType,
            _credential_data: &str,
        ) -> Result<Uuid, InfraError> {
            Ok(Uuid::now_v7())
        }

        async fn delete_by_user(
            &self,
            _tenant_id: &TenantId,
            _user_id: &UserId,
        ) -> Result<(), InfraError> {
            Ok(())
        }

        async fn delete_by_tenant(&self, _tenant_id: &TenantId) -> Result<(), InfraError> {
            Ok(())
        }

        async fn update_last_used(&self, _id: Uuid) -> Result<(), InfraError> {
            Ok(())
        }
    }

    struct StubPasswordChecker {
        result: bool,
    }

    impl StubPasswordChecker {
        fn success() -> Self {
            Self { result: true }
        }

        fn failure() -> Self {
            Self { result: false }
        }
    }

    impl PasswordChecker for StubPasswordChecker {
        fn verify(
            &self,
            _password: &PlainPassword,
            _hash: &PasswordHash,
        ) -> Result<PasswordVerifyResult, InfraError> {
            Ok(PasswordVerifyResult::from(self.result))
        }
    }

    #[tokio::test]
    async fn test_verify_password_成功() {
        // Given
        let repo = StubCredentialsRepository::with_active_credential("dummy_hash");
        let checker = StubPasswordChecker::success();
        let sut = AuthUseCaseImpl::new(Arc::new(repo), Arc::new(checker));

        // When
        let result = sut
            .verify_password(Uuid::now_v7(), Uuid::now_v7(), "password123")
            .await;

        // Then
        let result = result.unwrap();
        assert!(result.verified);
        assert!(result.credential_id.is_some());
    }

    #[tokio::test]
    async fn test_verify_password_パスワード不一致() {
        // Given
        let repo = StubCredentialsRepository::with_active_credential("dummy_hash");
        let checker = StubPasswordChecker::failure();
        let sut = AuthUseCaseImpl::new(Arc::new(repo), Arc::new(checker));

        // When
        let result = sut
            .verify_password(Uuid::now_v7(), Uuid::now_v7(), "wrongpassword")
            .await;

        // Then
        assert!(matches!(result, Err(AuthError::AuthenticationFailed)));
    }

    #[tokio::test]
    async fn test_verify_password_認証情報なし() {
        // Given
        let repo = StubCredentialsRepository::empty();
        let checker = StubPasswordChecker::success();
        let sut = AuthUseCaseImpl::new(Arc::new(repo), Arc::new(checker));

        // When
        let result = sut
            .verify_password(Uuid::now_v7(), Uuid::now_v7(), "password123")
            .await;

        // Then
        assert!(matches!(result, Err(AuthError::AuthenticationFailed)));
    }

    #[tokio::test]
    async fn test_verify_password_無効な認証情報() {
        // Given
        let repo = StubCredentialsRepository::with_inactive_credential();
        let checker = StubPasswordChecker::success();
        let sut = AuthUseCaseImpl::new(Arc::new(repo), Arc::new(checker));

        // When
        let result = sut
            .verify_password(Uuid::now_v7(), Uuid::now_v7(), "password123")
            .await;

        // Then
        assert!(matches!(result, Err(AuthError::CredentialInactive)));
    }

    #[tokio::test]
    async fn test_create_credential_成功() {
        // Given
        let repo = StubCredentialsRepository::empty();
        let checker = StubPasswordChecker::success();
        let sut = AuthUseCaseImpl::new(Arc::new(repo), Arc::new(checker));

        // When
        let result = sut
            .create_credential(
                Uuid::now_v7(),
                Uuid::now_v7(),
                "password",
                "hashed_password",
            )
            .await;

        // Then
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_credential_不正な種別() {
        // Given
        let repo = StubCredentialsRepository::empty();
        let checker = StubPasswordChecker::success();
        let sut = AuthUseCaseImpl::new(Arc::new(repo), Arc::new(checker));

        // When
        let result = sut
            .create_credential(Uuid::now_v7(), Uuid::now_v7(), "invalid_type", "data")
            .await;

        // Then
        assert!(matches!(result, Err(AuthError::Internal(_))));
    }
}
