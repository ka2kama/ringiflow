//! # 監査ログ閲覧 API ハンドラ
//!
//! BFF の監査ログ閲覧エンドポイントを提供する。
//!
//! ## エンドポイント
//!
//! - `GET /api/v1/audit-logs` -
//!   テナント内の監査ログ一覧（カーソルベースページネーション）

use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use chrono::DateTime;
use ringiflow_domain::{
    audit_log::{AuditAction, AuditResult},
    user::UserId,
};
use ringiflow_infra::{
    InfraErrorKind,
    SessionManager,
    repository::audit_log_repository::{AuditLogFilter, AuditLogRepository},
};
use ringiflow_shared::{ErrorResponse, PaginatedResponse};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::error::{authenticate, internal_error_response, validation_error_response};

/// 監査ログ閲覧 API の共有状態
pub struct AuditLogState {
    pub audit_log_repository: Arc<dyn AuditLogRepository>,
    pub session_manager:      Arc<dyn SessionManager>,
}

/// 監査ログ一覧クエリパラメータ
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListAuditLogsQuery {
    /// カーソル（次ページ取得用、opaque 文字列）
    pub cursor:   Option<String>,
    /// 取得件数（デフォルト 50、最大 100）
    pub limit:    Option<i32>,
    /// 開始日時（ISO 8601）
    pub from:     Option<String>,
    /// 終了日時（ISO 8601）
    pub to:       Option<String>,
    /// 操作者 ID でフィルタ
    pub actor_id: Option<Uuid>,
    /// アクションでフィルタ（カンマ区切りで複数指定可）
    pub action:   Option<String>,
    /// 結果でフィルタ（success / failure）
    pub result:   Option<String>,
}

/// 監査ログ一覧の要素データ
#[derive(Debug, Serialize, ToSchema)]
pub struct AuditLogItemData {
    pub id: String,
    pub actor_id: String,
    pub actor_name: String,
    pub action: String,
    pub result: String,
    pub resource_type: String,
    pub resource_id: String,
    pub detail: Option<serde_json::Value>,
    pub source_ip: Option<String>,
    pub created_at: String,
}

/// GET /api/v1/audit-logs
///
/// テナント内の監査ログ一覧を取得する（新しい順）。
/// カーソルベースページネーション対応。
#[utoipa::path(
   get,
   path = "/api/v1/audit-logs",
   tag = "audit-logs",
   security(("session_auth" = [])),
   params(ListAuditLogsQuery),
   responses(
      (status = 200, description = "監査ログ一覧", body = PaginatedResponse<AuditLogItemData>),
      (status = 400, description = "バリデーションエラー（不正なカーソル等）", body = ErrorResponse),
      (status = 401, description = "認証エラー", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all)]
pub async fn list_audit_logs(
    State(state): State<Arc<AuditLogState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Query(query): Query<ListAuditLogsQuery>,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    // limit のバリデーション（デフォルト 50、最大 100）
    let limit = query.limit.unwrap_or(50).clamp(1, 100);

    // フィルタの構築
    let from = query
        .from
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));

    let to = query
        .to
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));

    let actor_id = query.actor_id.map(UserId::from_uuid);

    let actions: Option<Vec<AuditAction>> = query
        .action
        .as_deref()
        .map(|s| s.split(',').filter_map(|a| a.trim().parse().ok()).collect());

    let result: Option<AuditResult> = query.result.as_deref().and_then(|s| s.parse().ok());

    let filter = AuditLogFilter {
        from,
        to,
        actor_id,
        actions,
        result,
    };

    match state
        .audit_log_repository
        .find_by_tenant(
            session_data.tenant_id(),
            query.cursor.as_deref(),
            limit,
            &filter,
        )
        .await
    {
        Ok(page) => {
            let items: Vec<AuditLogItemData> = page
                .items
                .into_iter()
                .map(|log| AuditLogItemData {
                    id: log.id.to_string(),
                    actor_id: log.actor_id.as_uuid().to_string(),
                    actor_name: log.actor_name,
                    action: log.action.to_string(),
                    result: log.result.to_string(),
                    resource_type: log.resource_type,
                    resource_id: log.resource_id,
                    detail: log.detail,
                    source_ip: log.source_ip,
                    created_at: log.created_at.to_rfc3339(),
                })
                .collect();

            let response = PaginatedResponse {
                data:        items,
                next_cursor: page.next_cursor,
            };
            Ok((StatusCode::OK, Json(response)).into_response())
        }
        Err(e) if matches!(e.kind(), InfraErrorKind::InvalidInput(_)) => {
            tracing::warn!("監査ログの検索でバリデーションエラー: {}", e);
            Err(validation_error_response("カーソルの形式が不正です"))
        }
        Err(e) => {
            tracing::error!("監査ログの検索に失敗: {}", e);
            Err(internal_error_response())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::{
        Router,
        body::Body,
        http::{Method, Request, StatusCode},
        routing::get,
    };
    use ringiflow_domain::tenant::TenantId;
    use ringiflow_infra::{
        InfraError,
        InfraErrorKind,
        SessionData,
        SessionManager,
        repository::audit_log_repository::{AuditLogFilter, AuditLogPage, AuditLogRepository},
    };
    use ringiflow_shared::ErrorResponse;
    use tower::ServiceExt;
    use uuid::Uuid;

    use super::*;

    // --- テスト用スタブ ---

    const TEST_TENANT_ID: &str = "00000000-0000-0000-0000-000000000001";

    struct StubAuditLogRepository {
        find_result: Result<AuditLogPage, InfraError>,
    }

    impl StubAuditLogRepository {
        fn with_error(err: InfraError) -> Self {
            Self {
                find_result: Err(err),
            }
        }
    }

    #[async_trait]
    impl AuditLogRepository for StubAuditLogRepository {
        async fn record(
            &self,
            _log: &ringiflow_domain::audit_log::AuditLog,
        ) -> Result<(), InfraError> {
            Ok(())
        }

        async fn find_by_tenant(
            &self,
            _tenant_id: &TenantId,
            _cursor: Option<&str>,
            _limit: i32,
            _filter: &AuditLogFilter,
        ) -> Result<AuditLogPage, InfraError> {
            match &self.find_result {
                Ok(page) => Ok(AuditLogPage {
                    items:       page.items.clone(),
                    next_cursor: page.next_cursor.clone(),
                }),
                Err(_) => {
                    // find_result を消費せずにエラーを再生成
                    // InfraError は Clone 非対応のため、パターンで再構築
                    Err(match self.find_result.as_ref().unwrap_err().kind() {
                        InfraErrorKind::InvalidInput(msg) => InfraError::invalid_input(msg.clone()),
                        InfraErrorKind::DynamoDb(msg) => InfraError::dynamo_db(msg.clone()),
                        _ => InfraError::unexpected("unexpected error"),
                    })
                }
            }
        }
    }

    struct StubSessionManager;

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
            Ok(Some(SessionData::new(
                ringiflow_domain::user::UserId::new(),
                TenantId::from_uuid(Uuid::parse_str(TEST_TENANT_ID).unwrap()),
                "user@example.com".to_string(),
                "Test User".to_string(),
                vec!["user".to_string()],
                vec!["workflow:read".to_string()],
            )))
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
            Ok(Some("a".repeat(64)))
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

    fn create_test_app(repo: StubAuditLogRepository) -> Router {
        let state = Arc::new(AuditLogState {
            audit_log_repository: Arc::new(repo),
            session_manager:      Arc::new(StubSessionManager),
        });
        Router::new()
            .route("/api/v1/audit-logs", get(list_audit_logs))
            .with_state(state)
    }

    async fn response_status_and_body(
        response: axum::response::Response,
    ) -> (StatusCode, ErrorResponse) {
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let error: ErrorResponse = serde_json::from_slice(&body).unwrap();
        (status, error)
    }

    // --- テスト ---

    #[tokio::test]
    async fn test_リポジトリがinvalid_inputを返すとき400を返す() {
        // Given
        let sut = create_test_app(StubAuditLogRepository::with_error(
            InfraError::invalid_input("カーソルのデコードに失敗"),
        ));

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/audit-logs?cursor=invalid")
            .header("X-Tenant-ID", TEST_TENANT_ID)
            .header("cookie", "session_id=test-session")
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        let (status, body) = response_status_and_body(response).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(
            body.error_type.ends_with("/validation-error"),
            "error_type should end with /validation-error, got: {}",
            body.error_type
        );
    }

    #[tokio::test]
    async fn test_リポジトリがdynamo_dbエラーを返すとき500を返す() {
        // Given
        let sut = create_test_app(StubAuditLogRepository::with_error(InfraError::dynamo_db(
            "DynamoDB connection failed",
        )));

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/audit-logs")
            .header("X-Tenant-ID", TEST_TENANT_ID)
            .header("cookie", "session_id=test-session")
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        let (status, body) = response_status_and_body(response).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(
            body.error_type.ends_with("/internal-error"),
            "error_type should end with /internal-error, got: {}",
            body.error_type
        );
    }
}
