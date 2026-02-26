//! # OpenAPI 仕様定義
//!
//! utoipa を使用して BFF の OpenAPI 仕様を Rust の型から自動生成する。
//! `ApiDoc::openapi()` で OpenAPI ドキュメントを取得できる。

use utoipa::{
    Modify,
    OpenApi,
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
};

use crate::handler::{
    audit_log,
    auth,
    dashboard,
    document,
    folder,
    health,
    role,
    task,
    user,
    workflow,
    workflow_definition,
};

#[derive(OpenApi)]
#[openapi(
   info(
      title = "RingiFlow API",
      version = "0.1.0",
      description = "ワークフロー管理システム RingiFlow の BFF API"
   ),
   paths(
      // health
      health::readiness_check,
      // auth
      auth::login,
      auth::logout,
      auth::me,
      auth::csrf,
      // workflows
      workflow::list_workflow_definitions,
      workflow::get_workflow_definition,
      workflow::list_my_workflows,
      workflow::create_workflow,
      workflow::get_workflow,
      workflow::submit_workflow,
      workflow::approve_step,
      workflow::reject_step,
      workflow::request_changes_step,
      workflow::resubmit_workflow,
      workflow::post_comment,
      workflow::list_comments,
      // workflow-definitions (管理)
      workflow_definition::create_definition,
      workflow_definition::update_definition,
      workflow_definition::delete_definition,
      workflow_definition::publish_definition,
      workflow_definition::archive_definition,
      workflow_definition::validate_definition,
      // tasks
      task::list_my_tasks,
      workflow::get_task_by_display_numbers,
      // users
      user::list_users,
      user::create_user,
      user::get_user_detail,
      user::update_user,
      user::update_user_status,
      // audit-logs
      audit_log::list_audit_logs,
      // roles
      role::list_roles,
      role::get_role,
      role::create_role,
      role::update_role,
      role::delete_role,
      // folders
      folder::list_folders,
      folder::create_folder,
      folder::update_folder,
      folder::delete_folder,
      // documents
      document::request_upload_url,
      document::confirm_upload,
      // dashboard
      dashboard::get_dashboard_stats,
   ),
   components(schemas(
      ringiflow_shared::ErrorResponse,
   )),
   tags(
      (name = "health", description = "ヘルスチェック"),
      (name = "auth", description = "認証"),
      (name = "workflows", description = "ワークフロー管理"),
      (name = "workflow-definitions", description = "ワークフロー定義管理"),
      (name = "tasks", description = "タスク管理"),
      (name = "users", description = "ユーザー管理"),
      (name = "roles", description = "ロール管理"),
      (name = "folders", description = "フォルダ管理"),
      (name = "documents", description = "ドキュメント管理"),
      (name = "audit-logs", description = "監査ログ"),
      (name = "dashboard", description = "ダッシュボード"),
   ),
   modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

/// セキュリティスキーム定義
///
/// Cookie ベースのセッション認証を追加する。
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_default();
        components.add_security_scheme(
            "session_auth",
            SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("session_id"))),
        );
    }
}
