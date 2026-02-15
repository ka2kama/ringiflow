//! # 認証ハンドラ
//!
//! Auth Service の認証エンドポイントを提供する。
//!
//! ## エンドポイント
//!
//! - `POST /internal/auth/verify` - パスワード認証
//! - `POST /internal/auth/credentials` - 認証情報作成
//! - `DELETE /internal/auth/credentials/{tenant_id}/{user_id}` - 認証情報削除
//!
//! 詳細: [08_AuthService設計.md](../../../../docs/03_詳細設計書/
//! 08_AuthService設計.md)

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::AuthError, usecase::AuthUseCase};

/// 認証ハンドラの共有状態
pub struct AuthState {
    pub usecase: Arc<dyn AuthUseCase>,
}

// --- リクエスト/レスポンス型 ---

/// パスワード認証リクエスト
#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    pub tenant_id: Uuid,
    pub user_id:   Uuid,
    pub password:  String,
}

/// パスワード認証レスポンス
#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub verified:      bool,
    pub credential_id: Option<Uuid>,
}

/// 認証情報作成リクエスト
#[derive(Debug, Deserialize)]
pub struct CreateCredentialsRequest {
    pub user_id:         Uuid,
    pub tenant_id:       Uuid,
    pub credential_type: String,
    pub credential_data: String,
}

/// 認証情報作成レスポンス
#[derive(Debug, Serialize)]
pub struct CreateCredentialsResponse {
    pub credential_id: Uuid,
}

// --- ハンドラ ---

/// POST /internal/auth/verify
///
/// パスワード認証を実行する。
///
/// ## タイミング攻撃対策
///
/// ユーザーが存在しない場合も、実際にダミーハッシュで検証を行い、
/// 処理時間を均一化する。これによりユーザー存在確認攻撃を防ぐ。
pub async fn verify(
    State(state): State<Arc<AuthState>>,
    Json(req): Json<VerifyRequest>,
) -> Result<impl IntoResponse, AuthError> {
    let result = state
        .usecase
        .verify_password(req.tenant_id, req.user_id, &req.password)
        .await?;

    Ok(Json(VerifyResponse {
        verified:      result.verified,
        credential_id: result.credential_id,
    }))
}

/// POST /internal/auth/credentials
///
/// 認証情報を登録する（ユーザー作成時に呼び出し）。
pub async fn create_credentials(
    State(state): State<Arc<AuthState>>,
    Json(req): Json<CreateCredentialsRequest>,
) -> Result<impl IntoResponse, AuthError> {
    let credential_id = state
        .usecase
        .create_credential(
            req.user_id,
            req.tenant_id,
            &req.credential_type,
            &req.credential_data,
        )
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(CreateCredentialsResponse { credential_id }),
    ))
}

/// DELETE /internal/auth/credentials/{tenant_id}/{user_id}
///
/// ユーザーの全認証情報を削除する（ユーザー削除時に呼び出し）。
pub async fn delete_credentials(
    State(state): State<Arc<AuthState>>,
    Path((tenant_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AuthError> {
    state.usecase.delete_credentials(tenant_id, user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::{
        Router,
        body::Body,
        http::{Method, Request},
        routing::{delete, post},
    };
    use tower::ServiceExt;
    use uuid::Uuid;

    use super::*;
    use crate::usecase::VerifyResult;

    // テスト用スタブ
    struct StubAuthUseCase {
        verify_success: bool,
    }

    impl StubAuthUseCase {
        fn success() -> Self {
            Self {
                verify_success: true,
            }
        }

        fn auth_failed() -> Self {
            Self {
                verify_success: false,
            }
        }
    }

    #[async_trait]
    impl AuthUseCase for StubAuthUseCase {
        async fn verify_password(
            &self,
            _tenant_id: Uuid,
            _user_id: Uuid,
            _password: &str,
        ) -> Result<VerifyResult, AuthError> {
            if self.verify_success {
                Ok(VerifyResult {
                    verified:      true,
                    credential_id: Some(Uuid::now_v7()),
                })
            } else {
                Err(AuthError::AuthenticationFailed)
            }
        }

        async fn create_credential(
            &self,
            _user_id: Uuid,
            _tenant_id: Uuid,
            _credential_type: &str,
            _credential_data: &str,
        ) -> Result<Uuid, AuthError> {
            Ok(Uuid::now_v7())
        }

        async fn delete_credentials(
            &self,
            _tenant_id: Uuid,
            _user_id: Uuid,
        ) -> Result<(), AuthError> {
            Ok(())
        }
    }

    fn create_test_app(usecase: StubAuthUseCase) -> Router {
        let state = Arc::new(AuthState {
            usecase: Arc::new(usecase),
        });

        Router::new()
            .route("/internal/auth/verify", post(verify))
            .route("/internal/auth/credentials", post(create_credentials))
            .route(
                "/internal/auth/credentials/{tenant_id}/{user_id}",
                delete(delete_credentials),
            )
            .with_state(state)
    }

    #[tokio::test]
    async fn test_verify_認証成功() {
        // Given
        let sut = create_test_app(StubAuthUseCase::success());

        let body = serde_json::json!({
            "tenant_id": "550e8400-e29b-41d4-a716-446655440001",
            "user_id": "550e8400-e29b-41d4-a716-446655440000",
            "password": "password123"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/internal/auth/verify")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["verified"], true);
        assert!(json["credential_id"].is_string());
    }

    #[tokio::test]
    async fn test_verify_認証失敗() {
        // Given
        let sut = create_test_app(StubAuthUseCase::auth_failed());

        let body = serde_json::json!({
            "tenant_id": "550e8400-e29b-41d4-a716-446655440001",
            "user_id": "550e8400-e29b-41d4-a716-446655440000",
            "password": "wrongpassword"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/internal/auth/verify")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_create_credentials_成功() {
        // Given
        let sut = create_test_app(StubAuthUseCase::success());

        let body = serde_json::json!({
            "user_id": "550e8400-e29b-41d4-a716-446655440000",
            "tenant_id": "550e8400-e29b-41d4-a716-446655440001",
            "credential_type": "password",
            "credential_data": "plain_password_here"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/internal/auth/credentials")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert!(json["credential_id"].is_string());
    }

    #[tokio::test]
    async fn test_delete_credentials_成功() {
        // Given
        let sut = create_test_app(StubAuthUseCase::success());
        let tenant_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();

        let request = Request::builder()
            .method(Method::DELETE)
            .uri(format!(
                "/internal/auth/credentials/{}/{}",
                tenant_id, user_id
            ))
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }
}
