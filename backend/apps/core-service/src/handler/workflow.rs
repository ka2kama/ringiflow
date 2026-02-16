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
    value_objects::{DisplayId, DisplayNumber, Version, display_prefix},
    workflow::{WorkflowComment, WorkflowDefinition, WorkflowInstance, WorkflowStep},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::CoreError,
    usecase::{StepApprover, WorkflowUseCaseImpl, WorkflowWithSteps},
};

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

/// ワークフロー再申請リクエスト
#[derive(Debug, Deserialize)]
pub struct ResubmitWorkflowRequest {
    /// 更新後のフォームデータ
    pub form_data: serde_json::Value,
    /// 各承認ステップの承認者リスト
    pub approvers: Vec<StepApproverRequest>,
    /// 楽観的ロック用バージョン
    pub version:   i32,
    /// テナント ID (内部 API 用)
    pub tenant_id: Uuid,
    /// 操作するユーザー ID (内部 API 用)
    pub user_id:   Uuid,
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
            display_id: DisplayId::new(
                display_prefix::WORKFLOW_INSTANCE,
                instance.display_number(),
            )
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
            display_id: DisplayId::new(
                display_prefix::WORKFLOW_INSTANCE,
                instance.display_number(),
            )
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

    /// ユーザー名を解決して WorkflowInstance から DTO を構築する（ステップなし）
    async fn resolve_from_instance(
        instance: &WorkflowInstance,
        usecase: &WorkflowUseCaseImpl,
    ) -> Result<Self, CoreError> {
        let user_ids = crate::usecase::workflow::collect_user_ids_from_workflow(instance, &[]);
        let user_names = usecase.resolve_user_names(&user_ids).await?;
        Ok(Self::from_instance(instance, &user_names))
    }

    /// ユーザー名を解決して WorkflowWithSteps から DTO を構築する
    pub(crate) async fn resolve_from_workflow_with_steps(
        data: &WorkflowWithSteps,
        usecase: &WorkflowUseCaseImpl,
    ) -> Result<Self, CoreError> {
        let user_ids =
            crate::usecase::workflow::collect_user_ids_from_workflow(&data.instance, &data.steps);
        let user_names = usecase.resolve_user_names(&user_ids).await?;
        Ok(Self::from_workflow_with_steps(data, &user_names))
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

/// i64 を DisplayNumber に変換する。
/// 不正な値の場合は CoreError::BadRequest を返す。
pub(crate) fn parse_display_number(value: i64, field: &str) -> Result<DisplayNumber, CoreError> {
    DisplayNumber::try_from(value)
        .map_err(|e| CoreError::BadRequest(format!("不正な {field}: {e}")))
}

/// i32 を Version に変換する。
pub(crate) fn parse_version(value: i32) -> Result<Version, CoreError> {
    Version::try_from(value).map_err(|e| CoreError::BadRequest(format!("不正なバージョン: {e}")))
}

/// StepApproverRequest のリストを StepApprover のリストに変換する。
pub(crate) fn convert_approvers(approvers: Vec<StepApproverRequest>) -> Vec<StepApprover> {
    approvers
        .into_iter()
        .map(|a| StepApprover {
            step_id:     a.step_id,
            assigned_to: UserId::from_uuid(a.assigned_to),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    #[test]
    fn test_parse_display_numberは正の整数で成功する() {
        let result = parse_display_number(1, "display_number");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_i64(), 1);
    }

    #[test]
    fn test_parse_display_numberはゼロで不正リクエストエラーを返す() {
        let result = parse_display_number(0, "display_number");
        assert!(
            matches!(result, Err(CoreError::BadRequest(msg)) if msg.contains("display_number"))
        );
    }

    #[test]
    fn test_parse_display_numberは負数で不正リクエストエラーを返す() {
        let result = parse_display_number(-1, "step_display_number");
        assert!(
            matches!(result, Err(CoreError::BadRequest(msg)) if msg.contains("step_display_number"))
        );
    }

    #[test]
    fn test_parse_versionは正の整数で成功する() {
        let result = parse_version(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_i32(), 1);
    }

    #[test]
    fn test_parse_versionはゼロで不正リクエストエラーを返す() {
        let result = parse_version(0);
        assert!(matches!(result, Err(CoreError::BadRequest(msg)) if msg.contains("バージョン")));
    }

    #[test]
    fn test_convert_approversは空入力で空を返す() {
        let result = convert_approvers(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_convert_approversは複数要素を変換する() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let input = vec![
            StepApproverRequest {
                step_id:     "step-1".to_string(),
                assigned_to: uuid1,
            },
            StepApproverRequest {
                step_id:     "step-2".to_string(),
                assigned_to: uuid2,
            },
        ];

        let result = convert_approvers(input);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].step_id, "step-1");
        assert_eq!(*result[0].assigned_to.as_uuid(), uuid1);
        assert_eq!(result[1].step_id, "step-2");
        assert_eq!(*result[1].assigned_to.as_uuid(), uuid2);
    }
}
