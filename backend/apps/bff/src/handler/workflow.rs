//! # ワークフロー API ハンドラ
//!
//! BFF のワークフロー関連エンドポイントを提供する。
//!
//! ハンドラは CQRS パターンで分割されている:
//! - `command`: 状態変更系（POST）
//! - `query`: 読み取り系（GET）
//!
//! ## BFF の責務
//!
//! 1. セッションから `tenant_id`, `user_id` を取得
//! 2. Core Service の内部 API を呼び出し
//! 3. レスポンスをクライアントに返す

mod command;
mod query;

use std::sync::Arc;

pub use command::*;
pub use query::*;
use ringiflow_infra::SessionManager;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::client::CoreServiceClient;

/// ワークフローハンドラの共有状態
pub struct WorkflowState {
   pub core_service_client: Arc<dyn CoreServiceClient>,
   pub session_manager:     Arc<dyn SessionManager>,
}

// --- リクエスト/レスポンス型 ---

/// ワークフロー作成リクエスト（BFF 公開 API）
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWorkflowRequest {
   /// ワークフロー定義 ID
   pub definition_id: Uuid,
   /// ワークフロータイトル
   pub title:         String,
   /// フォームデータ
   pub form_data:     serde_json::Value,
}

/// ワークフロー申請リクエスト（BFF 公開 API）
#[derive(Debug, Deserialize, ToSchema)]
pub struct SubmitWorkflowRequest {
   /// 承認者のユーザー ID
   pub assigned_to: Uuid,
}

/// ステップ承認/却下リクエスト（BFF 公開 API）
#[derive(Debug, Deserialize, ToSchema)]
pub struct ApproveRejectRequest {
   /// 楽観的ロック用バージョン
   pub version: i32,
   /// コメント（任意）
   pub comment: Option<String>,
}

/// ステップパスパラメータ（display_number 用）
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
pub struct StepPathParams {
   /// ワークフローの表示用連番
   pub display_number:      i64,
   /// ステップの表示用連番
   pub step_display_number: i64,
}

/// ユーザー参照データ（フロントエンドへの Serialize 用）
#[derive(Debug, Serialize, ToSchema)]
pub struct UserRefData {
   pub id:   String,
   pub name: String,
}

impl From<crate::client::UserRefDto> for UserRefData {
   fn from(dto: crate::client::UserRefDto) -> Self {
      Self {
         id:   dto.id,
         name: dto.name,
      }
   }
}

/// ワークフローステップデータ
#[derive(Debug, Serialize, ToSchema)]
pub struct WorkflowStepData {
   pub id: String,
   pub display_id: String,
   pub display_number: i64,
   pub step_id: String,
   pub step_name: String,
   pub step_type: String,
   pub status: String,
   pub version: i32,
   pub assigned_to: Option<UserRefData>,
   pub decision: Option<String>,
   pub comment: Option<String>,
   pub due_date: Option<String>,
   pub started_at: Option<String>,
   pub completed_at: Option<String>,
   pub created_at: String,
   pub updated_at: String,
}

impl From<crate::client::WorkflowStepDto> for WorkflowStepData {
   fn from(dto: crate::client::WorkflowStepDto) -> Self {
      Self {
         id: dto.id,
         display_id: dto.display_id,
         display_number: dto.display_number,
         step_id: dto.step_id,
         step_name: dto.step_name,
         step_type: dto.step_type,
         status: dto.status,
         version: dto.version,
         assigned_to: dto.assigned_to.map(UserRefData::from),
         decision: dto.decision,
         comment: dto.comment,
         due_date: dto.due_date,
         started_at: dto.started_at,
         completed_at: dto.completed_at,
         created_at: dto.created_at,
         updated_at: dto.updated_at,
      }
   }
}

/// ワークフローデータ
#[derive(Debug, Serialize, ToSchema)]
pub struct WorkflowData {
   pub id: String,
   pub display_id: String,
   pub display_number: i64,
   pub title: String,
   pub definition_id: String,
   pub status: String,
   pub version: i32,
   pub form_data: serde_json::Value,
   pub initiated_by: UserRefData,
   pub current_step_id: Option<String>,
   pub steps: Vec<WorkflowStepData>,
   pub submitted_at: Option<String>,
   pub completed_at: Option<String>,
   pub created_at: String,
   pub updated_at: String,
}

impl From<crate::client::WorkflowInstanceDto> for WorkflowData {
   fn from(dto: crate::client::WorkflowInstanceDto) -> Self {
      Self {
         id: dto.id,
         display_id: dto.display_id,
         display_number: dto.display_number,
         title: dto.title,
         definition_id: dto.definition_id,
         status: dto.status,
         version: dto.version,
         form_data: dto.form_data,
         initiated_by: UserRefData::from(dto.initiated_by),
         current_step_id: dto.current_step_id,
         steps: dto.steps.into_iter().map(WorkflowStepData::from).collect(),
         submitted_at: dto.submitted_at,
         completed_at: dto.completed_at,
         created_at: dto.created_at,
         updated_at: dto.updated_at,
      }
   }
}

/// ワークフロー定義データ
#[derive(Debug, Serialize, ToSchema)]
pub struct WorkflowDefinitionData {
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

impl From<crate::client::WorkflowDefinitionDto> for WorkflowDefinitionData {
   fn from(dto: crate::client::WorkflowDefinitionDto) -> Self {
      Self {
         id:          dto.id,
         name:        dto.name,
         description: dto.description,
         version:     dto.version,
         definition:  dto.definition,
         status:      dto.status,
         created_by:  dto.created_by,
         created_at:  dto.created_at,
         updated_at:  dto.updated_at,
      }
   }
}
