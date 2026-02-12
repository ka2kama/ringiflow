//! # ワークフロー API ハンドラ
//!
//! Core Service のワークフロー関連エンドポイントを実装する。
//!
//! ハンドラは CQRS パターンで分割されている:
//! - `command`: 状態変更系（POST）
//! - `query`: 読み取り系（GET）

mod command;
mod query;

use std::collections::HashMap;

pub use command::*;
pub use query::*;
use ringiflow_domain::{
   user::UserId,
   value_objects::{DisplayId, display_prefix},
   workflow::{WorkflowComment, WorkflowDefinition, WorkflowInstance, WorkflowStep},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::usecase::{WorkflowUseCaseImpl, WorkflowWithSteps};

/// ワークフロー作成リクエスト
#[derive(Debug, Deserialize)]
pub struct CreateWorkflowRequest {
   /// ワークフロー定義 ID
   pub definition_id: Uuid,
   /// ワークフロータイトル
   pub title:         String,
   /// フォームデータ
   pub form_data:     serde_json::Value,
   /// テナント ID (内部 API 用)
   pub tenant_id:     Uuid,
   /// 申請者のユーザー ID (内部 API 用)
   pub user_id:       Uuid,
}

/// ステップ承認者リクエスト
#[derive(Debug, Deserialize)]
pub struct StepApproverRequest {
   /// 定義 JSON のステップ ID
   pub step_id:     String,
   /// 承認者のユーザー ID
   pub assigned_to: Uuid,
}

/// ワークフロー申請リクエスト
#[derive(Debug, Deserialize)]
pub struct SubmitWorkflowRequest {
   /// 各承認ステップの承認者リスト
   pub approvers: Vec<StepApproverRequest>,
   /// テナント ID (内部 API 用)
   pub tenant_id: Uuid,
}

/// ステップ承認/却下リクエスト
#[derive(Debug, Deserialize)]
pub struct ApproveRejectRequest {
   /// 楽観的ロック用バージョン
   pub version:   i32,
   /// コメント（任意）
   pub comment:   Option<String>,
   /// テナント ID (内部 API 用)
   pub tenant_id: Uuid,
   /// 操作するユーザー ID (内部 API 用)
   pub user_id:   Uuid,
}

/// ステップパスパラメータ
#[derive(Debug, Deserialize)]
pub struct StepPathParams {
   /// ワークフローインスタンス ID
   /// 注: 現在の実装では step_id のみで検索するため未使用だが、
   /// 将来的に所属関係のバリデーションに使用する可能性あり
   #[allow(dead_code)]
   pub id:      Uuid,
   /// ステップ ID
   pub step_id: Uuid,
}

/// display_number によるステップパスパラメータ
#[derive(Debug, Deserialize)]
pub struct StepByDisplayNumberPathParams {
   /// ワークフローインスタンスの表示用連番
   pub display_number:      i64,
   /// ステップの表示用連番
   pub step_display_number: i64,
}

/// テナント指定クエリパラメータ（GET リクエスト用）
#[derive(Debug, Deserialize)]
pub struct TenantQuery {
   /// テナント ID
   pub tenant_id: Uuid,
}

/// ユーザー指定クエリパラメータ（GET リクエスト用）
#[derive(Debug, Deserialize)]
pub struct UserQuery {
   /// テナント ID
   pub tenant_id: Uuid,
   /// ユーザー ID
   pub user_id:   Uuid,
}

/// ユーザー参照 DTO
///
/// UUID 文字列の代わりに、ID とユーザー名をペアで返す。
/// フロントエンドでの表示用。
#[derive(Debug, Clone, Serialize)]
pub struct UserRefDto {
   pub id:   String,
   pub name: String,
}

/// ユーザー名マップからユーザー参照を作成する
///
/// ユーザーが見つからない場合は「（不明なユーザー）」にフォールバック。
pub(crate) fn to_user_ref(user_id: &UserId, user_names: &HashMap<UserId, String>) -> UserRefDto {
   let id = user_id.to_string();
   let name = user_names.get(user_id).cloned().unwrap_or_else(|| {
      tracing::warn!(user_id = %user_id, "User not found when resolving user name");
      "（不明なユーザー）".to_string()
   });
   UserRefDto { id, name }
}

/// ワークフロー定義 DTO
#[derive(Debug, Serialize)]
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

impl From<WorkflowDefinition> for WorkflowDefinitionDto {
   fn from(def: WorkflowDefinition) -> Self {
      Self {
         id:          def.id().to_string(),
         name:        def.name().to_string(),
         description: def.description().map(|s| s.to_string()),
         version:     def.version().as_i32(),
         definition:  def.definition().clone(),
         status:      format!("{:?}", def.status()),
         created_by:  def.created_by().to_string(),
         created_at:  def.created_at().to_rfc3339(),
         updated_at:  def.updated_at().to_rfc3339(),
      }
   }
}

/// ワークフローステップ DTO
#[derive(Debug, Serialize)]
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

impl WorkflowStepDto {
   pub(crate) fn from_step(step: &WorkflowStep, user_names: &HashMap<UserId, String>) -> Self {
      Self {
         id: step.id().to_string(),
         display_id: DisplayId::new(display_prefix::WORKFLOW_STEP, step.display_number())
            .to_string(),
         display_number: step.display_number().as_i64(),
         step_id: step.step_id().to_string(),
         step_name: step.step_name().to_string(),
         step_type: step.step_type().to_string(),
         status: format!("{:?}", step.status()),
         version: step.version().as_i32(),
         assigned_to: step.assigned_to().map(|u| to_user_ref(u, user_names)),
         decision: step.decision().map(|d| format!("{:?}", d)),
         comment: step.comment().map(|s| s.to_string()),
         due_date: step.due_date().map(|t| t.to_rfc3339()),
         started_at: step.started_at().map(|t| t.to_rfc3339()),
         completed_at: step.completed_at().map(|t| t.to_rfc3339()),
         created_at: step.created_at().to_rfc3339(),
         updated_at: step.updated_at().to_rfc3339(),
      }
   }
}

/// ワークフローインスタンス DTO
#[derive(Debug, Serialize)]
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
   pub steps: Vec<WorkflowStepDto>,
   pub submitted_at: Option<String>,
   pub completed_at: Option<String>,
   pub created_at: String,
   pub updated_at: String,
}

impl WorkflowInstanceDto {
   /// 一覧 API 用: ステップなしの変換
   fn from_instance(instance: &WorkflowInstance, user_names: &HashMap<UserId, String>) -> Self {
      Self {
         id: instance.id().to_string(),
         display_id: DisplayId::new(display_prefix::WORKFLOW_INSTANCE, instance.display_number())
            .to_string(),
         display_number: instance.display_number().as_i64(),
         title: instance.title().to_string(),
         definition_id: instance.definition_id().to_string(),
         status: format!("{:?}", instance.status()),
         version: instance.version().as_i32(),
         form_data: instance.form_data().clone(),
         initiated_by: to_user_ref(instance.initiated_by(), user_names),
         current_step_id: instance.current_step_id().map(|s| s.to_string()),
         steps: Vec::new(),
         submitted_at: instance.submitted_at().map(|t| t.to_rfc3339()),
         completed_at: instance.completed_at().map(|t| t.to_rfc3339()),
         created_at: instance.created_at().to_rfc3339(),
         updated_at: instance.updated_at().to_rfc3339(),
      }
   }

   /// 詳細 API 用: ステップ付きの変換
   pub(crate) fn from_workflow_with_steps(
      data: &WorkflowWithSteps,
      user_names: &HashMap<UserId, String>,
   ) -> Self {
      let instance = &data.instance;
      Self {
         id: instance.id().to_string(),
         display_id: DisplayId::new(display_prefix::WORKFLOW_INSTANCE, instance.display_number())
            .to_string(),
         display_number: instance.display_number().as_i64(),
         title: instance.title().to_string(),
         definition_id: instance.definition_id().to_string(),
         status: format!("{:?}", instance.status()),
         version: instance.version().as_i32(),
         form_data: instance.form_data().clone(),
         initiated_by: to_user_ref(instance.initiated_by(), user_names),
         current_step_id: instance.current_step_id().map(|s| s.to_string()),
         steps: data
            .steps
            .iter()
            .map(|s| WorkflowStepDto::from_step(s, user_names))
            .collect(),
         submitted_at: instance.submitted_at().map(|t| t.to_rfc3339()),
         completed_at: instance.completed_at().map(|t| t.to_rfc3339()),
         created_at: instance.created_at().to_rfc3339(),
         updated_at: instance.updated_at().to_rfc3339(),
      }
   }
}

/// コメント投稿リクエスト
#[derive(Debug, Deserialize)]
pub struct PostCommentRequest {
   /// コメント本文
   pub body:      String,
   /// テナント ID (内部 API 用)
   pub tenant_id: Uuid,
   /// 投稿者のユーザー ID (内部 API 用)
   pub user_id:   Uuid,
}

/// ワークフローコメント DTO
#[derive(Debug, Serialize)]
pub struct WorkflowCommentDto {
   pub id:         String,
   pub posted_by:  UserRefDto,
   pub body:       String,
   pub created_at: String,
}

impl WorkflowCommentDto {
   pub(crate) fn from_comment(
      comment: &WorkflowComment,
      user_names: &HashMap<UserId, String>,
   ) -> Self {
      Self {
         id:         comment.id().to_string(),
         posted_by:  to_user_ref(comment.posted_by(), user_names),
         body:       comment.body().as_str().to_string(),
         created_at: comment.created_at().to_rfc3339(),
      }
   }
}

/// ワークフローハンドラーの State
pub struct WorkflowState {
   pub usecase: WorkflowUseCaseImpl,
}
