//! # BFF エラーハンドリング
//!
//! HTTP API のエラー定義と、axum レスポンスへの変換。
//!
//! BFF の各ハンドラが共通で使うエラー型とヘルパー関数を集約する。

use axum::{
    Json,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use ringiflow_domain::tenant::TenantId;
use ringiflow_infra::{SessionData, SessionManager};
use ringiflow_shared::ErrorResponse;
use uuid::Uuid;

use crate::client::CoreServiceError;

/// Cookie 名
const SESSION_COOKIE_NAME: &str = "session_id";

// --- エラー型 ---

/// テナント ID 抽出エラー
#[derive(Debug)]
pub enum TenantIdError {
    /// ヘッダーが存在しない
    Missing,
    /// UUID の形式が不正
    InvalidFormat,
}

impl IntoResponse for TenantIdError {
    fn into_response(self) -> Response {
        let detail = match self {
            TenantIdError::Missing => "X-Tenant-ID ヘッダーが必要です",
            TenantIdError::InvalidFormat => "X-Tenant-ID の形式が不正です",
        };
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::validation_error(detail)),
        )
            .into_response()
    }
}

// --- 共通ヘルパー関数 ---

/// X-Tenant-ID ヘッダーからテナント ID を抽出する
pub fn extract_tenant_id(headers: &HeaderMap) -> Result<Uuid, TenantIdError> {
    let tenant_id_str = headers
        .get("X-Tenant-ID")
        .and_then(|v| v.to_str().ok())
        .ok_or(TenantIdError::Missing)?;

    Uuid::parse_str(tenant_id_str).map_err(|_| TenantIdError::InvalidFormat)
}

/// セッションを取得する
pub async fn get_session(
    session_manager: &dyn SessionManager,
    jar: &CookieJar,
    tenant_id: Uuid,
) -> Result<ringiflow_infra::SessionData, Response> {
    // Cookie からセッション ID を取得
    let session_id = jar
        .get(SESSION_COOKIE_NAME)
        .map(|cookie| cookie.value().to_string())
        .ok_or_else(unauthorized_response)?;

    let tenant_id = TenantId::from_uuid(tenant_id);

    // セッションを取得
    match session_manager.get(&tenant_id, &session_id).await {
        Ok(Some(data)) => Ok(data),
        Ok(None) => Err(unauthorized_response()),
        Err(e) => {
            tracing::error!(
                error.category = "infrastructure",
                error.kind = "session",
                "セッション取得で内部エラー: {}",
                e
            );
            Err(internal_error_response())
        }
    }
}

/// セッション認証を行う
///
/// `extract_tenant_id` + `get_session` を統合したヘルパー。
/// ハンドラでの 10 行のボイラープレートを 1 行に削減する。
pub async fn authenticate(
    session_manager: &dyn SessionManager,
    headers: &HeaderMap,
    jar: &CookieJar,
) -> Result<SessionData, Response> {
    let tenant_id = extract_tenant_id(headers).map_err(IntoResponse::into_response)?;
    get_session(session_manager, jar, tenant_id).await
}

// --- IntoResponse for CoreServiceError ---

impl IntoResponse for CoreServiceError {
    fn into_response(self) -> Response {
        match self {
            CoreServiceError::UserNotFound => not_found_response(
                "user-not-found",
                "User Not Found",
                "ユーザーが見つかりません",
            ),
            CoreServiceError::WorkflowDefinitionNotFound => not_found_response(
                "workflow-definition-not-found",
                "Workflow Definition Not Found",
                "ワークフロー定義が見つかりません",
            ),
            CoreServiceError::WorkflowInstanceNotFound => not_found_response(
                "workflow-instance-not-found",
                "Workflow Instance Not Found",
                "ワークフローインスタンスが見つかりません",
            ),
            CoreServiceError::StepNotFound => not_found_response(
                "step-not-found",
                "Step Not Found",
                "ステップが見つかりません",
            ),
            CoreServiceError::RoleNotFound => {
                not_found_response("role-not-found", "Role Not Found", "ロールが見つかりません")
            }
            CoreServiceError::ValidationError(ref detail) => validation_error_response(detail),
            CoreServiceError::Forbidden(ref detail) => forbidden_response(detail),
            CoreServiceError::EmailAlreadyExists => {
                conflict_response("このメールアドレスは既に使用されています")
            }
            CoreServiceError::Conflict(ref detail) => conflict_response(detail),
            CoreServiceError::Network(_) | CoreServiceError::Unexpected(_) => {
                internal_error_response()
            }
        }
    }
}

/// Core Service エラーをログ付きでレスポンスに変換する
///
/// `Network`/`Unexpected` エラーの場合はコンテキスト付きで `tracing::error!` を出力する。
/// その他のエラーは `IntoResponse` でレスポンスに変換するのみ。
pub fn log_and_convert_core_error(context: &str, err: CoreServiceError) -> Response {
    match &err {
        CoreServiceError::Network(_) | CoreServiceError::Unexpected(_) => {
            tracing::error!(
                error.category = "external_service",
                error.kind = "service_communication",
                "{}で内部エラー: {}",
                context,
                err
            );
        }
        _ => {}
    }
    err.into_response()
}

// --- レスポンスヘルパー ---

/// 認証失敗レスポンス
pub fn authentication_failed_response() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse::new(
            "authentication-failed",
            "Authentication Failed",
            401,
            "メールアドレスまたはパスワードが正しくありません",
        )),
    )
        .into_response()
}

/// 未認証レスポンス
pub fn unauthorized_response() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse::unauthorized("認証が必要です")),
    )
        .into_response()
}

/// 内部エラーレスポンス
pub fn internal_error_response() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::internal_error()),
    )
        .into_response()
}

/// Auth Service 利用不可レスポンス
pub fn service_unavailable_response() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorResponse::service_unavailable(
            "認証サービスが一時的に利用できません",
        )),
    )
        .into_response()
}

/// 404 Not Found レスポンス
pub fn not_found_response(error_type_suffix: &str, title: &str, detail: &str) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new(error_type_suffix, title, 404, detail)),
    )
        .into_response()
}

/// バリデーションエラーレスポンス
pub fn validation_error_response(detail: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse::validation_error(detail)),
    )
        .into_response()
}

/// 403 Forbidden レスポンス
pub fn forbidden_response(detail: &str) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse::forbidden(detail)),
    )
        .into_response()
}

/// 409 Conflict レスポンス
pub fn conflict_response(detail: &str) -> Response {
    (StatusCode::CONFLICT, Json(ErrorResponse::conflict(detail))).into_response()
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use axum::{body::to_bytes, http::HeaderMap, response::IntoResponse};
    use axum_extra::extract::{CookieJar, cookie::Cookie};
    use ringiflow_domain::{tenant::TenantId, user::UserId};
    use ringiflow_infra::{InfraError, SessionData, SessionManager};
    use uuid::Uuid;

    use super::*;

    // --- テスト用スタブ ---

    struct StubSessionManager {
        session: Option<SessionData>,
    }

    impl StubSessionManager {
        fn new() -> Self {
            Self { session: None }
        }

        fn with_session(user_id: UserId, tenant_id: TenantId) -> Self {
            Self {
                session: Some(SessionData::new(
                    user_id,
                    tenant_id,
                    "user@example.com".to_string(),
                    "Test User".to_string(),
                    vec!["user".to_string()],
                    vec!["workflow:read".to_string()],
                )),
            }
        }
    }

    #[async_trait]
    impl SessionManager for StubSessionManager {
        async fn create(&self, _data: &SessionData) -> Result<String, InfraError> {
            Ok(Uuid::now_v7().to_string())
        }

        async fn create_with_id(
            &self,
            _session_id: &str,
            _data: &SessionData,
        ) -> Result<(), InfraError> {
            Ok(())
        }

        async fn get(
            &self,
            _tenant_id: &TenantId,
            _session_id: &str,
        ) -> Result<Option<SessionData>, InfraError> {
            Ok(self.session.clone())
        }

        async fn delete(&self, _tenant_id: &TenantId, _session_id: &str) -> Result<(), InfraError> {
            Ok(())
        }

        async fn delete_all_for_tenant(&self, _tenant_id: &TenantId) -> Result<(), InfraError> {
            Ok(())
        }

        async fn get_ttl(
            &self,
            _tenant_id: &TenantId,
            _session_id: &str,
        ) -> Result<Option<i64>, InfraError> {
            Ok(Some(28800))
        }

        async fn create_csrf_token(
            &self,
            _tenant_id: &TenantId,
            _session_id: &str,
        ) -> Result<String, InfraError> {
            Ok("a".repeat(64))
        }

        async fn get_csrf_token(
            &self,
            _tenant_id: &TenantId,
            _session_id: &str,
        ) -> Result<Option<String>, InfraError> {
            if self.session.is_some() {
                Ok(Some("a".repeat(64)))
            } else {
                Ok(None)
            }
        }

        async fn delete_csrf_token(
            &self,
            _tenant_id: &TenantId,
            _session_id: &str,
        ) -> Result<(), InfraError> {
            Ok(())
        }

        async fn delete_all_csrf_for_tenant(
            &self,
            _tenant_id: &TenantId,
        ) -> Result<(), InfraError> {
            Ok(())
        }
    }

    // --- ヘルパー ---

    fn make_headers_with_tenant(tenant_id: Uuid) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("X-Tenant-ID", tenant_id.to_string().parse().unwrap());
        headers
    }

    fn make_jar_with_session(session_id: &str) -> CookieJar {
        CookieJar::new().add(Cookie::new("session_id", session_id.to_string()))
    }

    async fn response_status_and_body(
        response: Response,
    ) -> (StatusCode, ringiflow_shared::ErrorResponse) {
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let error: ringiflow_shared::ErrorResponse = serde_json::from_slice(&body).unwrap();
        (status, error)
    }

    // --- authenticate テスト ---

    #[tokio::test]
    async fn authenticate_正常系でsession_dataを返す() {
        let tenant_id = Uuid::now_v7();
        let user_id = UserId::new();
        let sm = StubSessionManager::with_session(user_id.clone(), TenantId::from_uuid(tenant_id));
        let headers = make_headers_with_tenant(tenant_id);
        let jar = make_jar_with_session("valid-session-id");

        let result = authenticate(&sm, &headers, &jar).await;
        assert!(result.is_ok());
        let session = result.unwrap();
        assert_eq!(session.user_id(), &user_id);
    }

    #[tokio::test]
    async fn authenticate_テナントidヘッダーなしで400() {
        let sm = StubSessionManager::new();
        let headers = HeaderMap::new();
        let jar = make_jar_with_session("session-id");

        let result = authenticate(&sm, &headers, &jar).await;
        assert!(result.is_err());
        let (status, _) = response_status_and_body(result.unwrap_err()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn authenticate_テナントid不正形式で400() {
        let sm = StubSessionManager::new();
        let mut headers = HeaderMap::new();
        headers.insert("X-Tenant-ID", "not-a-uuid".parse().unwrap());
        let jar = make_jar_with_session("session-id");

        let result = authenticate(&sm, &headers, &jar).await;
        assert!(result.is_err());
        let (status, _) = response_status_and_body(result.unwrap_err()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn authenticate_セッションcookieなしで401() {
        let tenant_id = Uuid::now_v7();
        let sm = StubSessionManager::new();
        let headers = make_headers_with_tenant(tenant_id);
        let jar = CookieJar::new();

        let result = authenticate(&sm, &headers, &jar).await;
        assert!(result.is_err());
        let (status, _) = response_status_and_body(result.unwrap_err()).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn authenticate_セッション存在しない場合に401() {
        let tenant_id = Uuid::now_v7();
        let sm = StubSessionManager::new(); // session: None
        let headers = make_headers_with_tenant(tenant_id);
        let jar = make_jar_with_session("nonexistent-session");

        let result = authenticate(&sm, &headers, &jar).await;
        assert!(result.is_err());
        let (status, _) = response_status_and_body(result.unwrap_err()).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    // --- IntoResponse for CoreServiceError テスト ---

    fn assert_error_type_ends_with(error: &ringiflow_shared::ErrorResponse, suffix: &str) {
        assert!(
            error.error_type.ends_with(suffix),
            "expected error_type to end with '{}', got '{}'",
            suffix,
            error.error_type
        );
    }

    #[tokio::test]
    async fn core_service_error_user_not_foundで404() {
        let response = CoreServiceError::UserNotFound.into_response();
        let (status, body) = response_status_and_body(response).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_error_type_ends_with(&body, "/user-not-found");
    }

    #[tokio::test]
    async fn core_service_error_workflow_definition_not_foundで404() {
        let response = CoreServiceError::WorkflowDefinitionNotFound.into_response();
        let (status, body) = response_status_and_body(response).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_error_type_ends_with(&body, "/workflow-definition-not-found");
    }

    #[tokio::test]
    async fn core_service_error_workflow_instance_not_foundで404() {
        let response = CoreServiceError::WorkflowInstanceNotFound.into_response();
        let (status, body) = response_status_and_body(response).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_error_type_ends_with(&body, "/workflow-instance-not-found");
    }

    #[tokio::test]
    async fn core_service_error_step_not_foundで404() {
        let response = CoreServiceError::StepNotFound.into_response();
        let (status, body) = response_status_and_body(response).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_error_type_ends_with(&body, "/step-not-found");
    }

    #[tokio::test]
    async fn core_service_error_role_not_foundで404() {
        let response = CoreServiceError::RoleNotFound.into_response();
        let (status, body) = response_status_and_body(response).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_error_type_ends_with(&body, "/role-not-found");
    }

    #[tokio::test]
    async fn core_service_error_validation_errorで400() {
        let response =
            CoreServiceError::ValidationError("入力が不正です".to_string()).into_response();
        let (status, body) = response_status_and_body(response).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_error_type_ends_with(&body, "/validation-error");
    }

    #[tokio::test]
    async fn core_service_error_forbiddenで403() {
        let response = CoreServiceError::Forbidden("権限なし".to_string()).into_response();
        let (status, body) = response_status_and_body(response).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_error_type_ends_with(&body, "/forbidden");
    }

    #[tokio::test]
    async fn core_service_error_email_already_existsで409() {
        let response = CoreServiceError::EmailAlreadyExists.into_response();
        let (status, body) = response_status_and_body(response).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_error_type_ends_with(&body, "/conflict");
    }

    #[tokio::test]
    async fn core_service_error_conflictで409() {
        let response = CoreServiceError::Conflict("バージョン競合".to_string()).into_response();
        let (status, body) = response_status_and_body(response).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_error_type_ends_with(&body, "/conflict");
    }

    #[tokio::test]
    async fn core_service_error_networkで500() {
        let response = CoreServiceError::Network("接続失敗".to_string()).into_response();
        let (status, body) = response_status_and_body(response).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_error_type_ends_with(&body, "/internal-error");
    }

    #[tokio::test]
    async fn core_service_error_unexpectedで500() {
        let response = CoreServiceError::Unexpected("予期しないエラー".to_string()).into_response();
        let (status, body) = response_status_and_body(response).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_error_type_ends_with(&body, "/internal-error");
    }

    // --- log_and_convert_core_error テスト ---

    #[tokio::test]
    async fn log_and_convert_core_error_networkで500() {
        let response =
            log_and_convert_core_error("テスト操作", CoreServiceError::Network("err".to_string()));
        let (status, _) = response_status_and_body(response).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn log_and_convert_core_error_user_not_foundで404() {
        let response = log_and_convert_core_error("テスト操作", CoreServiceError::UserNotFound);
        let (status, body) = response_status_and_body(response).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_error_type_ends_with(&body, "/user-not-found");
    }
}
