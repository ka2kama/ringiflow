//! ワークフロー定義管理 API の認可テスト
//!
//! BFF の認可ミドルウェアが `workflow_definition:manage` 権限を
//! 正しく検証することを確認する。
//!
//! ## テストケース
//!
//! - `workflow:*` 権限では 403（wildcard は `workflow_definition:` に不一致）
//! - `workflow_definition:manage` 権限では認可通過
//! - 未認証では 401

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
    middleware::from_fn_with_state,
    routing::post,
};
use ringiflow_bff::{
    client::{
        CoreServiceError,
        CoreServiceWorkflowClient,
        CreateDefinitionCoreRequest,
        PublishArchiveCoreRequest,
        UpdateDefinitionCoreRequest,
        ValidateDefinitionCoreRequest,
        ValidationResultDto,
        WorkflowDefinitionDto,
        WorkflowInstanceDto,
    },
    handler::{WorkflowDefinitionState, create_definition},
    middleware::{AuthzState, require_permission},
};
use ringiflow_domain::{tenant::TenantId, user::UserId};
use ringiflow_infra::{InfraError, SessionData, SessionManager};
use ringiflow_shared::ApiResponse;
use tower::ServiceExt;
use uuid::Uuid;

const TEST_TENANT_ID: &str = "00000000-0000-0000-0000-000000000001";

// --- SessionManager スタブ ---

/// テスト用スタブ SessionManager
struct StubSessionManager {
    session: Option<SessionData>,
}

impl StubSessionManager {
    fn no_session() -> Self {
        Self { session: None }
    }

    fn with_permissions(permissions: Vec<String>) -> Self {
        let tenant_id = TenantId::from_uuid(Uuid::parse_str(TEST_TENANT_ID).unwrap());
        Self {
            session: Some(SessionData::new(
                UserId::new(),
                tenant_id,
                "user@example.com".to_string(),
                "Test User".to_string(),
                vec!["user".to_string()],
                permissions,
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
        Ok(None)
    }

    async fn delete_csrf_token(
        &self,
        _tenant_id: &TenantId,
        _session_id: &str,
    ) -> Result<(), InfraError> {
        Ok(())
    }

    async fn delete_all_csrf_for_tenant(&self, _tenant_id: &TenantId) -> Result<(), InfraError> {
        Ok(())
    }
}

// --- CoreServiceWorkflowClient スタブ ---
//
// 認可ミドルウェアが拒否する場合、ハンドラは呼ばれないため
// これらのメソッドは実行されない。

struct UnusedWorkflowClient;

#[async_trait]
impl CoreServiceWorkflowClient for UnusedWorkflowClient {
    async fn create_workflow(
        &self,
        _req: ringiflow_bff::client::CreateWorkflowRequest,
    ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
        unimplemented!("ミドルウェアが拒否するため呼ばれない")
    }

    async fn submit_workflow(
        &self,
        _workflow_id: Uuid,
        _req: ringiflow_bff::client::SubmitWorkflowRequest,
    ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
        unimplemented!()
    }

    async fn list_workflow_definitions(
        &self,
        _tenant_id: Uuid,
    ) -> Result<ApiResponse<Vec<WorkflowDefinitionDto>>, CoreServiceError> {
        unimplemented!()
    }

    async fn get_workflow_definition(
        &self,
        _definition_id: Uuid,
        _tenant_id: Uuid,
    ) -> Result<ApiResponse<WorkflowDefinitionDto>, CoreServiceError> {
        unimplemented!()
    }

    async fn list_my_workflows(
        &self,
        _tenant_id: Uuid,
        _user_id: Uuid,
    ) -> Result<ApiResponse<Vec<WorkflowInstanceDto>>, CoreServiceError> {
        unimplemented!()
    }

    async fn get_workflow(
        &self,
        _workflow_id: Uuid,
        _tenant_id: Uuid,
    ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
        unimplemented!()
    }

    async fn approve_step(
        &self,
        _workflow_id: Uuid,
        _step_id: Uuid,
        _req: ringiflow_bff::client::ApproveRejectRequest,
    ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
        unimplemented!()
    }

    async fn reject_step(
        &self,
        _workflow_id: Uuid,
        _step_id: Uuid,
        _req: ringiflow_bff::client::ApproveRejectRequest,
    ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
        unimplemented!()
    }

    async fn get_workflow_by_display_number(
        &self,
        _display_number: i64,
        _tenant_id: Uuid,
    ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
        unimplemented!()
    }

    async fn submit_workflow_by_display_number(
        &self,
        _display_number: i64,
        _req: ringiflow_bff::client::SubmitWorkflowRequest,
    ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
        unimplemented!()
    }

    async fn approve_step_by_display_number(
        &self,
        _workflow_display_number: i64,
        _step_display_number: i64,
        _req: ringiflow_bff::client::ApproveRejectRequest,
    ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
        unimplemented!()
    }

    async fn reject_step_by_display_number(
        &self,
        _workflow_display_number: i64,
        _step_display_number: i64,
        _req: ringiflow_bff::client::ApproveRejectRequest,
    ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
        unimplemented!()
    }

    async fn request_changes_step_by_display_number(
        &self,
        _workflow_display_number: i64,
        _step_display_number: i64,
        _req: ringiflow_bff::client::ApproveRejectRequest,
    ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
        unimplemented!()
    }

    async fn resubmit_workflow_by_display_number(
        &self,
        _display_number: i64,
        _req: ringiflow_bff::client::ResubmitWorkflowRequest,
    ) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
        unimplemented!()
    }

    async fn post_comment(
        &self,
        _display_number: i64,
        _req: ringiflow_bff::client::PostCommentCoreRequest,
    ) -> Result<ApiResponse<ringiflow_bff::client::WorkflowCommentDto>, CoreServiceError> {
        unimplemented!()
    }

    async fn list_comments(
        &self,
        _display_number: i64,
        _tenant_id: Uuid,
    ) -> Result<ApiResponse<Vec<ringiflow_bff::client::WorkflowCommentDto>>, CoreServiceError> {
        unimplemented!()
    }

    async fn create_workflow_definition(
        &self,
        _req: &CreateDefinitionCoreRequest,
    ) -> Result<ApiResponse<WorkflowDefinitionDto>, CoreServiceError> {
        // 認可通過テストではハンドラまで到達するため、パニックではなくエラーを返す
        Err(CoreServiceError::Unexpected("テスト用スタブ".to_string()))
    }

    async fn update_workflow_definition(
        &self,
        _definition_id: Uuid,
        _req: &UpdateDefinitionCoreRequest,
    ) -> Result<ApiResponse<WorkflowDefinitionDto>, CoreServiceError> {
        unimplemented!()
    }

    async fn delete_workflow_definition(
        &self,
        _definition_id: Uuid,
        _tenant_id: Uuid,
    ) -> Result<(), CoreServiceError> {
        unimplemented!()
    }

    async fn publish_workflow_definition(
        &self,
        _definition_id: Uuid,
        _req: &PublishArchiveCoreRequest,
    ) -> Result<ApiResponse<WorkflowDefinitionDto>, CoreServiceError> {
        unimplemented!()
    }

    async fn archive_workflow_definition(
        &self,
        _definition_id: Uuid,
        _req: &PublishArchiveCoreRequest,
    ) -> Result<ApiResponse<WorkflowDefinitionDto>, CoreServiceError> {
        unimplemented!()
    }

    async fn validate_workflow_definition(
        &self,
        _req: &ValidateDefinitionCoreRequest,
    ) -> Result<ApiResponse<ValidationResultDto>, CoreServiceError> {
        unimplemented!()
    }
}

// --- テストヘルパー ---

fn create_test_app(session_manager: StubSessionManager) -> Router {
    let session_manager: Arc<dyn SessionManager> = Arc::new(session_manager);

    let authz_state = AuthzState {
        session_manager:     session_manager.clone(),
        required_permission: "workflow_definition:manage".to_string(),
    };

    let workflow_def_state = Arc::new(WorkflowDefinitionState {
        core_service_client: Arc::new(UnusedWorkflowClient),
        session_manager:     session_manager.clone(),
    });

    Router::new()
        .route("/api/v1/workflow-definitions", post(create_definition))
        .layer(from_fn_with_state(authz_state, require_permission))
        .with_state(workflow_def_state)
}

fn create_request() -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri("/api/v1/workflow-definitions")
        .header("content-type", "application/json")
        .header("X-Tenant-ID", TEST_TENANT_ID)
        .header("Cookie", "session_id=test-session-id")
        .body(Body::from(
            serde_json::json!({
                "name": "テスト",
                "definition": {"steps": []},
            })
            .to_string(),
        ))
        .unwrap()
}

// --- テストケース ---

#[tokio::test]
async fn test_workflow_wildcard権限ではworkflow_definition_manageが拒否される() {
    // Given: workflow:* 権限を持つユーザー
    // workflow:* の satisfies は starts_with("workflow:") で判定するが、
    // workflow_definition:manage は starts_with("workflow_definition:") のため不一致
    let sut = create_test_app(StubSessionManager::with_permissions(vec![
        "workflow:*".to_string(),
    ]));

    // When
    let response = sut.oneshot(create_request()).await.unwrap();

    // Then: 403 Forbidden
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_workflow_definition_manage権限があれば認可を通過する() {
    // Given: workflow_definition:manage 権限を持つユーザー
    let sut = create_test_app(StubSessionManager::with_permissions(vec![
        "workflow_definition:manage".to_string(),
    ]));

    // When
    let response = sut.oneshot(create_request()).await.unwrap();

    // Then: 403 ではない（ミドルウェアを通過した）
    // ハンドラ内でスタブ CoreService が呼ばれるため 500 になるが、
    // 認可が通過したことが重要
    assert_ne!(response.status(), StatusCode::FORBIDDEN);
    assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_未認証ユーザーは401を返す() {
    // Given: セッションなし
    let sut = create_test_app(StubSessionManager::no_session());

    // When
    let response = sut.oneshot(create_request()).await.unwrap();

    // Then: 401 Unauthorized
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
