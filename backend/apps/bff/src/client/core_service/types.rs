//! Core Service クライアントの DTO / リクエスト型

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// --- レスポンス型 ---

/// ユーザー情報レスポンス
#[derive(Debug, Clone, Deserialize)]
pub struct UserResponse {
    pub id:        Uuid,
    pub tenant_id: Uuid,
    pub email:     String,
    pub name:      String,
    pub status:    String,
}

/// ユーザー詳細データ（権限付き）
#[derive(Debug, Clone, Deserialize)]
pub struct UserWithPermissionsData {
    pub user:        UserResponse,
    pub tenant_name: String,
    pub roles:       Vec<String>,
    pub permissions: Vec<String>,
}

/// ユーザー一覧の要素 DTO
#[derive(Debug, Clone, Deserialize)]
pub struct UserItemDto {
    pub id: Uuid,
    pub display_id: String,
    pub display_number: i64,
    pub name: String,
    pub email: String,
    pub status: String,
    pub roles: Vec<String>,
}

/// ユーザー作成リクエスト（Core Service 内部 API 用）
#[derive(Debug, Serialize)]
pub struct CreateUserCoreRequest {
    pub tenant_id: Uuid,
    pub email:     String,
    pub name:      String,
    pub role_id:   Uuid,
}

/// ユーザー作成レスポンス（Core Service 内部 API 用）
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserCoreResponse {
    pub id: Uuid,
    pub display_id: String,
    pub display_number: i64,
    pub name: String,
    pub email: String,
    pub role: String,
}

/// ユーザー更新リクエスト（Core Service 内部 API 用）
#[derive(Debug, Serialize)]
pub struct UpdateUserCoreRequest {
    pub name:    Option<String>,
    pub role_id: Option<Uuid>,
}

/// ユーザーステータス変更リクエスト（Core Service 内部 API 用）
#[derive(Debug, Serialize)]
pub struct UpdateUserStatusCoreRequest {
    pub status:       String,
    pub tenant_id:    Uuid,
    pub requester_id: Uuid,
}

// --- ユーザー参照型 ---

/// ユーザー参照 DTO（Core Service からのデシリアライズ用）
#[derive(Debug, Clone, Deserialize)]
pub struct UserRefDto {
    pub id:   String,
    pub name: String,
}

// --- ワークフロー関連の型 ---

/// ワークフロー作成リクエスト（Core Service 内部 API 用）
#[derive(Debug, Serialize)]
pub struct CreateWorkflowRequest {
    pub definition_id: Uuid,
    pub title:         String,
    pub form_data:     serde_json::Value,
    pub tenant_id:     Uuid,
    pub user_id:       Uuid,
}

/// ステップ承認者リクエスト（Core Service 内部 API 用）
#[derive(Debug, Serialize)]
pub struct StepApproverRequest {
    pub step_id:     String,
    pub assigned_to: Uuid,
}

/// ワークフロー申請リクエスト（Core Service 内部 API 用）
#[derive(Debug, Serialize)]
pub struct SubmitWorkflowRequest {
    pub approvers: Vec<StepApproverRequest>,
    pub tenant_id: Uuid,
}

/// ステップ承認/却下リクエスト（Core Service 内部 API 用）
#[derive(Debug, Serialize)]
pub struct ApproveRejectRequest {
    pub version:   i32,
    pub comment:   Option<String>,
    pub tenant_id: Uuid,
    pub user_id:   Uuid,
}

/// ワークフロー再申請リクエスト（Core Service 内部 API 用）
#[derive(Debug, Serialize)]
pub struct ResubmitWorkflowRequest {
    pub form_data: serde_json::Value,
    pub approvers: Vec<StepApproverRequest>,
    pub version:   i32,
    pub tenant_id: Uuid,
    pub user_id:   Uuid,
}

/// ワークフローステップ DTO
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowStepDto {
    pub id: String,
    pub display_id: String,
    pub display_number: i64,
    pub step_id: String,
    pub step_name: String,
    pub step_type: String,
    pub status: String,
    pub version: i32,
    pub assigned_to: Option<UserRefDto>,
    pub decision: Option<String>,
    pub comment: Option<String>,
    pub due_date: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// ワークフローインスタンス DTO
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowInstanceDto {
    pub id: String,
    pub display_id: String,
    pub display_number: i64,
    pub title: String,
    pub definition_id: String,
    pub status: String,
    pub version: i32,
    pub form_data: serde_json::Value,
    pub initiated_by: UserRefDto,
    pub current_step_id: Option<String>,
    #[serde(default)]
    pub steps: Vec<WorkflowStepDto>,
    pub submitted_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// ワークフロー定義 DTO
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowDefinitionDto {
    pub id:          String,
    pub name:        String,
    pub description: Option<String>,
    pub version:     i32,
    pub definition:  serde_json::Value,
    pub status:      String,
    pub created_by:  String,
    pub created_at:  String,
    pub updated_at:  String,
}

// --- ワークフロー定義管理リクエスト型 ---

/// ワークフロー定義作成リクエスト（Core Service 内部 API 用）
#[derive(Debug, Serialize)]
pub struct CreateDefinitionCoreRequest {
    pub name:        String,
    pub description: Option<String>,
    pub definition:  serde_json::Value,
    pub tenant_id:   Uuid,
    pub user_id:     Uuid,
}

/// ワークフロー定義更新リクエスト（Core Service 内部 API 用）
#[derive(Debug, Serialize)]
pub struct UpdateDefinitionCoreRequest {
    pub name:        String,
    pub description: Option<String>,
    pub definition:  serde_json::Value,
    pub version:     i32,
    pub tenant_id:   Uuid,
}

/// ワークフロー定義公開/アーカイブリクエスト（Core Service 内部 API 用）
#[derive(Debug, Serialize)]
pub struct PublishArchiveCoreRequest {
    pub version:   i32,
    pub tenant_id: Uuid,
}

/// ワークフロー定義バリデーションリクエスト（Core Service 内部 API 用）
#[derive(Debug, Serialize)]
pub struct ValidateDefinitionCoreRequest {
    pub definition: serde_json::Value,
}

/// バリデーション結果 DTO
#[derive(Debug, Clone, Deserialize)]
pub struct ValidationResultDto {
    pub valid:  bool,
    pub errors: Vec<ValidationErrorDto>,
}

/// バリデーションエラー DTO
#[derive(Debug, Clone, Deserialize)]
pub struct ValidationErrorDto {
    pub code:    String,
    pub message: String,
    pub step_id: Option<String>,
}

// --- タスク関連の型 ---

/// ワークフロー概要 DTO（タスク一覧用）
#[derive(Debug, Clone, Deserialize)]
pub struct TaskWorkflowSummaryDto {
    pub id: String,
    pub display_id: String,
    pub display_number: i64,
    pub title: String,
    pub status: String,
    pub initiated_by: UserRefDto,
    pub submitted_at: Option<String>,
}

/// タスク一覧の要素 DTO
#[derive(Debug, Clone, Deserialize)]
pub struct TaskItemDto {
    pub id: String,
    pub display_number: i64,
    pub step_name: String,
    pub status: String,
    pub version: i32,
    pub assigned_to: Option<UserRefDto>,
    pub due_date: Option<String>,
    pub started_at: Option<String>,
    pub created_at: String,
    pub workflow: TaskWorkflowSummaryDto,
}

/// タスク詳細 DTO
#[derive(Debug, Clone, Deserialize)]
pub struct TaskDetailDto {
    pub step:     WorkflowStepDto,
    pub workflow: WorkflowInstanceDto,
}

// --- ロール関連の型 ---

/// ロール一覧の要素 DTO
#[derive(Debug, Clone, Deserialize)]
pub struct RoleItemDto {
    pub id:          Uuid,
    pub name:        String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub is_system:   bool,
    pub user_count:  i64,
}

/// ロール詳細 DTO
#[derive(Debug, Clone, Deserialize)]
pub struct RoleDetailDto {
    pub id:          Uuid,
    pub name:        String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub is_system:   bool,
    pub created_at:  String,
    pub updated_at:  String,
}

/// ロール作成リクエスト（Core Service 内部 API 用）
#[derive(Debug, Serialize)]
pub struct CreateRoleCoreRequest {
    pub tenant_id:   Uuid,
    pub name:        String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
}

/// ロール更新リクエスト（Core Service 内部 API 用）
#[derive(Debug, Serialize)]
pub struct UpdateRoleCoreRequest {
    pub name:        Option<String>,
    pub description: Option<String>,
    pub permissions: Option<Vec<String>>,
}

// --- コメント関連の型 ---

/// コメント投稿リクエスト（Core Service 内部 API 用）
#[derive(Debug, Serialize)]
pub struct PostCommentCoreRequest {
    pub body:      String,
    pub tenant_id: Uuid,
    pub user_id:   Uuid,
}

/// ワークフローコメント DTO
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowCommentDto {
    pub id:         String,
    pub posted_by:  UserRefDto,
    pub body:       String,
    pub created_at: String,
}

// --- フォルダ関連の型 ---

/// フォルダ DTO（Core Service からのデシリアライズ用）
#[derive(Debug, Clone, Deserialize)]
pub struct FolderItemDto {
    pub id:         Uuid,
    pub name:       String,
    pub parent_id:  Option<Uuid>,
    pub path:       String,
    pub depth:      i32,
    pub created_at: String,
    pub updated_at: String,
}

/// フォルダ作成リクエスト（Core Service 内部 API 用）
#[derive(Debug, Serialize)]
pub struct CreateFolderCoreRequest {
    pub tenant_id:  Uuid,
    pub name:       String,
    pub parent_id:  Option<Uuid>,
    pub created_by: Uuid,
}

/// フォルダ更新リクエスト（Core Service 内部 API 用）
#[derive(Debug, Serialize)]
pub struct UpdateFolderCoreRequest {
    pub tenant_id: Uuid,
    pub name:      Option<String>,
    pub parent_id: Option<Option<Uuid>>,
}

// --- ドキュメント関連の型 ---

/// Upload URL 発行リクエスト（Core Service 内部 API 用）
#[derive(Debug, Serialize)]
pub struct RequestUploadUrlCoreRequest {
    pub tenant_id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub content_length: i64,
    pub folder_id: Option<Uuid>,
    pub workflow_instance_id: Option<Uuid>,
    pub uploaded_by: Uuid,
}

/// Upload URL レスポンス DTO（Core Service からのデシリアライズ用）
#[derive(Debug, Clone, Deserialize)]
pub struct UploadUrlCoreDto {
    pub document_id: Uuid,
    pub upload_url:  String,
    pub expires_in:  u64,
}

/// ドキュメント詳細 DTO（Core Service からのデシリアライズ用）
#[derive(Debug, Clone, Deserialize)]
pub struct DocumentDetailCoreDto {
    pub id:           Uuid,
    pub filename:     String,
    pub content_type: String,
    pub size:         i64,
    pub status:       String,
    pub created_at:   String,
}

/// ダウンロード URL DTO（Core Service からのデシリアライズ用）
#[derive(Debug, Clone, Deserialize)]
pub struct DownloadUrlCoreDto {
    pub download_url: String,
    pub expires_in:   u64,
}

// --- ダッシュボード関連の型 ---

/// ダッシュボード統計 DTO
#[derive(Debug, Clone, Deserialize)]
pub struct DashboardStatsDto {
    pub pending_tasks: i64,
    pub my_workflows_in_progress: i64,
    pub completed_today: i64,
}
