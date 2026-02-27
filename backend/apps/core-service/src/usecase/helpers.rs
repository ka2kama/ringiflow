//! ユースケース層の共通ヘルパー
//!
//! リポジトリ呼び出し結果の変換や権限チェックなど、
//! 複数のユースケースで繰り返されるパターンを共通化する。

use ringiflow_domain::{user::UserId, workflow::WorkflowStep};
use ringiflow_infra::InfraError;

use crate::error::CoreError;

/// リポジトリの `Result<Option<T>, InfraError>` を `Result<T, CoreError>` に変換する
///
/// `find_by_id` 等の `Option` を返すリポジトリメソッドの結果を、
/// `CoreError::NotFound` または `CoreError::Internal` に変換する。
///
/// ```ignore
/// // Before
/// let step = self.step_repo.find_by_id(&step_id, &tenant_id).await
///     .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?
///     .ok_or_else(|| CoreError::NotFound("ステップが見つかりません".to_string()))?;
///
/// // After
/// let step = self.step_repo.find_by_id(&step_id, &tenant_id).await
///     .or_not_found("ステップ")?;
/// ```
pub(crate) trait FindResultExt<T> {
    /// `None` の場合は `CoreError::NotFound`、`InfraError` の場合は `CoreError::Internal` を返す
    fn or_not_found(self, entity_name: &str) -> Result<T, CoreError>;
}

impl<T> FindResultExt<T> for Result<Option<T>, InfraError> {
    fn or_not_found(self, entity_name: &str) -> Result<T, CoreError> {
        self.map_err(|e| CoreError::Internal(format!("{}の取得に失敗: {}", entity_name, e)))?
            .ok_or_else(|| CoreError::NotFound(format!("{}が見つかりません", entity_name)))
    }
}

/// ステップの担当者をチェックする
///
/// 指定されたユーザーがステップの担当者でない場合、`CoreError::Forbidden` を返す。
pub(crate) fn check_step_assigned_to(
    step: &WorkflowStep,
    user_id: &UserId,
    action: &str,
) -> Result<(), CoreError> {
    if step.assigned_to() != Some(user_id) {
        return Err(CoreError::Forbidden(format!(
            "このステップを{}する権限がありません",
            action,
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use ringiflow_domain::{
        user::UserId,
        value_objects::DisplayNumber,
        workflow::{NewWorkflowStep, WorkflowInstanceId, WorkflowStep, WorkflowStepId},
    };
    use ringiflow_infra::InfraError;

    use super::*;

    // === FindResultExt ===

    #[test]
    fn test_or_not_found_ok_some_は値を返す() {
        let result: Result<Option<i32>, InfraError> = Ok(Some(42));

        let value = result.or_not_found("テスト").unwrap();

        assert_eq!(value, 42);
    }

    #[test]
    fn test_or_not_found_ok_none_はnotfoundエラーを返す() {
        let result: Result<Option<i32>, InfraError> = Ok(None);

        let err = result.or_not_found("ステップ").unwrap_err();

        match err {
            CoreError::NotFound(msg) => {
                assert_eq!(msg, "ステップが見つかりません");
            }
            other => panic!("NotFound を期待したが {:?} を受信", other),
        }
    }

    #[test]
    fn test_or_not_found_errはinternalエラーを返す() {
        let result: Result<Option<i32>, InfraError> = Err(InfraError::unexpected("接続失敗"));

        let err = result.or_not_found("インスタンス").unwrap_err();

        match err {
            CoreError::Internal(msg) => {
                assert!(msg.contains("インスタンスの取得に失敗"));
                assert!(msg.contains("接続失敗"));
            }
            other => panic!("Internal を期待したが {:?} を受信", other),
        }
    }

    // === check_step_assigned_to ===

    fn create_test_step(assigned_to: Option<UserId>) -> WorkflowStep {
        let now = chrono::Utc::now();
        WorkflowStep::new(NewWorkflowStep {
            id: WorkflowStepId::new(),
            instance_id: WorkflowInstanceId::new(),
            display_number: DisplayNumber::new(1).unwrap(),
            step_id: "test_step".to_string(),
            step_name: "テストステップ".to_string(),
            step_type: "approval".to_string(),
            assigned_to,
            now,
        })
    }

    #[test]
    fn test_check_step_assigned_to_担当者一致はokを返す() {
        let user_id = UserId::new();
        let step = create_test_step(Some(user_id.clone()));

        let result = check_step_assigned_to(&step, &user_id, "承認");

        assert!(result.is_ok());
    }

    #[test]
    fn test_check_step_assigned_to_担当者不一致はforbiddenを返す() {
        let assigned_user = UserId::new();
        let other_user = UserId::new();
        let step = create_test_step(Some(assigned_user));

        let err = check_step_assigned_to(&step, &other_user, "承認").unwrap_err();

        match err {
            CoreError::Forbidden(msg) => {
                assert_eq!(msg, "このステップを承認する権限がありません");
            }
            other => panic!("Forbidden を期待したが {:?} を受信", other),
        }
    }

    #[test]
    fn test_check_step_assigned_to_担当者なしはforbiddenを返す() {
        let user_id = UserId::new();
        let step = create_test_step(None);

        let err = check_step_assigned_to(&step, &user_id, "却下").unwrap_err();

        match err {
            CoreError::Forbidden(msg) => {
                assert_eq!(msg, "このステップを却下する権限がありません");
            }
            other => panic!("Forbidden を期待したが {:?} を受信", other),
        }
    }
}
