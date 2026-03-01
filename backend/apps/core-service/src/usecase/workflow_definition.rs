//! # ワークフロー定義ユースケース
//!
//! ワークフロー定義の CRUD 操作とバリデーションを実装する。

use std::sync::Arc;

use ringiflow_domain::{
    clock::Clock,
    tenant::TenantId,
    user::UserId,
    value_objects::{Version, WorkflowName},
    workflow::{
        NewWorkflowDefinition,
        ValidationResult,
        WorkflowDefinition,
        WorkflowDefinitionId,
        validate_definition,
    },
};
use ringiflow_infra::{InfraErrorKind, repository::WorkflowDefinitionRepository};
use serde_json::Value as JsonValue;

use super::helpers::FindResultExt;
use crate::error::CoreError;

/// ワークフロー定義ユースケース
pub struct WorkflowDefinitionUseCaseImpl {
    definition_repo: Arc<dyn WorkflowDefinitionRepository>,
    clock:           Arc<dyn Clock>,
}

impl WorkflowDefinitionUseCaseImpl {
    pub fn new(
        definition_repo: Arc<dyn WorkflowDefinitionRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            definition_repo,
            clock,
        }
    }

    /// テナント内の全定義を取得（ステータス問わず）
    pub async fn list_definitions(
        &self,
        tenant_id: &TenantId,
    ) -> Result<Vec<WorkflowDefinition>, CoreError> {
        self.definition_repo
            .find_all_by_tenant(tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("定義一覧の取得に失敗: {}", e)))
    }

    /// ID で定義を取得
    pub async fn get_definition(
        &self,
        id: &WorkflowDefinitionId,
        tenant_id: &TenantId,
    ) -> Result<WorkflowDefinition, CoreError> {
        self.definition_repo
            .find_by_id(id, tenant_id)
            .await
            .or_not_found("ワークフロー定義")
    }

    /// 新規定義を作成（Draft 状態）
    pub async fn create_definition(
        &self,
        name: WorkflowName,
        description: Option<String>,
        definition: JsonValue,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> Result<WorkflowDefinition, CoreError> {
        let now = self.clock.now();
        let def = WorkflowDefinition::new(NewWorkflowDefinition {
            id: WorkflowDefinitionId::new(),
            tenant_id,
            name,
            description,
            definition,
            created_by: user_id,
            now,
        });

        self.definition_repo
            .insert(&def)
            .await
            .map_err(|e| CoreError::Internal(format!("定義の保存に失敗: {}", e)))?;

        Ok(def)
    }

    /// 定義を更新（Draft のみ）
    pub async fn update_definition(
        &self,
        id: &WorkflowDefinitionId,
        name: WorkflowName,
        description: Option<String>,
        definition: JsonValue,
        expected_version: Version,
        tenant_id: &TenantId,
    ) -> Result<WorkflowDefinition, CoreError> {
        let existing = self
            .definition_repo
            .find_by_id(id, tenant_id)
            .await
            .or_not_found("ワークフロー定義")?;

        let now = self.clock.now();
        let updated = existing
            .update(name, description, definition, now)
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        self.definition_repo
            .update_with_version_check(&updated, expected_version)
            .await
            .map_err(map_version_conflict)?;

        Ok(updated)
    }

    /// 定義を削除（Draft のみ）
    pub async fn delete_definition(
        &self,
        id: &WorkflowDefinitionId,
        tenant_id: &TenantId,
    ) -> Result<(), CoreError> {
        let existing = self
            .definition_repo
            .find_by_id(id, tenant_id)
            .await
            .or_not_found("ワークフロー定義")?;

        existing
            .can_delete()
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        self.definition_repo
            .delete(id, tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("定義の削除に失敗: {}", e)))?;

        Ok(())
    }

    /// 定義を公開（バリデーション成功後、Draft → Published）
    pub async fn publish_definition(
        &self,
        id: &WorkflowDefinitionId,
        expected_version: Version,
        tenant_id: &TenantId,
    ) -> Result<WorkflowDefinition, CoreError> {
        let existing = self
            .definition_repo
            .find_by_id(id, tenant_id)
            .await
            .or_not_found("ワークフロー定義")?;

        // 公開前バリデーション
        let result = validate_definition(existing.definition());
        if !result.valid {
            let messages: Vec<String> = result.errors.iter().map(|e| e.message.clone()).collect();
            return Err(CoreError::BadRequest(format!(
                "バリデーションエラー: {}",
                messages.join("; ")
            )));
        }

        let now = self.clock.now();
        let published = existing
            .published(now)
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        self.definition_repo
            .update_with_version_check(&published, expected_version)
            .await
            .map_err(map_version_conflict)?;

        Ok(published)
    }

    /// 定義をアーカイブ（Published → Archived）
    pub async fn archive_definition(
        &self,
        id: &WorkflowDefinitionId,
        expected_version: Version,
        tenant_id: &TenantId,
    ) -> Result<WorkflowDefinition, CoreError> {
        let existing = self
            .definition_repo
            .find_by_id(id, tenant_id)
            .await
            .or_not_found("ワークフロー定義")?;

        let now = self.clock.now();
        let archived = existing
            .archived(now)
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;

        self.definition_repo
            .update_with_version_check(&archived, expected_version)
            .await
            .map_err(map_version_conflict)?;

        Ok(archived)
    }

    /// 定義 JSON のバリデーションのみ実行
    pub fn validate_definition_json(&self, definition: &JsonValue) -> ValidationResult {
        validate_definition(definition)
    }
}

/// InfraError のバージョン競合を CoreError::Conflict にマッピング
fn map_version_conflict(e: ringiflow_infra::InfraError) -> CoreError {
    match e.kind() {
        InfraErrorKind::Conflict { .. } => CoreError::Conflict(
            "このワークフロー定義は既に更新されています。最新の状態を取得してください。"
                .to_string(),
        ),
        _ => CoreError::Internal(format!("定義の保存に失敗: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use ringiflow_domain::clock::FixedClock;
    use ringiflow_infra::fake::FakeWorkflowDefinitionRepository;
    use serde_json::json;

    use super::*;

    fn fixed_now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn create_usecase() -> (
        WorkflowDefinitionUseCaseImpl,
        Arc<FakeWorkflowDefinitionRepository>,
    ) {
        let repo = Arc::new(FakeWorkflowDefinitionRepository::new());
        let clock = Arc::new(FixedClock::new(fixed_now()));
        let usecase = WorkflowDefinitionUseCaseImpl::new(repo.clone(), clock);
        (usecase, repo)
    }

    fn valid_definition_json() -> JsonValue {
        json!({
            "steps": [
                {"id": "start", "type": "start", "name": "開始"},
                {"id": "approval_1", "type": "approval", "name": "承認"},
                {"id": "end_approved", "type": "end", "name": "承認完了", "status": "approved"},
                {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
            ],
            "transitions": [
                {"from": "start", "to": "approval_1"},
                {"from": "approval_1", "to": "end_approved", "trigger": "approve"},
                {"from": "approval_1", "to": "end_rejected", "trigger": "reject"}
            ]
        })
    }

    fn tenant_id() -> TenantId {
        TenantId::new()
    }

    fn user_id() -> UserId {
        UserId::new()
    }

    #[tokio::test]
    async fn test_定義作成が成功しdraft状態で保存される() {
        let (usecase, _repo) = create_usecase();
        let tid = tenant_id();

        let def = usecase
            .create_definition(
                WorkflowName::new("テスト定義").unwrap(),
                Some("テスト".to_string()),
                json!({"steps": []}),
                tid.clone(),
                user_id(),
            )
            .await
            .unwrap();

        assert_eq!(
            def.status(),
            ringiflow_domain::workflow::WorkflowDefinitionStatus::Draft
        );
        assert_eq!(def.name().as_str(), "テスト定義");

        // リポジトリに保存されていることを確認
        let list = usecase.list_definitions(&tid).await.unwrap();
        assert_eq!(list.len(), 1);
    }

    #[tokio::test]
    async fn test_draft定義の更新が成功しバージョンがインクリメントされる() {
        let (usecase, _repo) = create_usecase();
        let tid = tenant_id();

        let def = usecase
            .create_definition(
                WorkflowName::new("元の名前").unwrap(),
                None,
                json!({"steps": []}),
                tid.clone(),
                user_id(),
            )
            .await
            .unwrap();
        let original_version = def.version();

        let updated = usecase
            .update_definition(
                def.id(),
                WorkflowName::new("更新後の名前").unwrap(),
                Some("説明追加".to_string()),
                json!({"steps": [{"id": "s1"}]}),
                original_version,
                &tid,
            )
            .await
            .unwrap();

        assert_eq!(updated.name().as_str(), "更新後の名前");
        assert_eq!(updated.version(), original_version.next());
    }

    #[tokio::test]
    async fn test_published定義の更新がエラーを返す() {
        let (usecase, repo) = create_usecase();
        let tid = tenant_id();

        // Published 定義を直接セットアップ
        let def = WorkflowDefinition::new(NewWorkflowDefinition {
            id:          WorkflowDefinitionId::new(),
            tenant_id:   tid.clone(),
            name:        WorkflowName::new("公開済み").unwrap(),
            description: None,
            definition:  valid_definition_json(),
            created_by:  user_id(),
            now:         fixed_now(),
        });
        let published = def.published(fixed_now()).unwrap();
        repo.add_definition(published.clone());

        let result = usecase
            .update_definition(
                published.id(),
                WorkflowName::new("更新").unwrap(),
                None,
                json!({}),
                published.version(),
                &tid,
            )
            .await;

        assert!(matches!(result, Err(CoreError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_draft定義の削除が成功する() {
        let (usecase, _repo) = create_usecase();
        let tid = tenant_id();

        let def = usecase
            .create_definition(
                WorkflowName::new("削除対象").unwrap(),
                None,
                json!({"steps": []}),
                tid.clone(),
                user_id(),
            )
            .await
            .unwrap();

        usecase.delete_definition(def.id(), &tid).await.unwrap();

        let list = usecase.list_definitions(&tid).await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_published定義の削除がエラーを返す() {
        let (usecase, repo) = create_usecase();
        let tid = tenant_id();

        let def = WorkflowDefinition::new(NewWorkflowDefinition {
            id:          WorkflowDefinitionId::new(),
            tenant_id:   tid.clone(),
            name:        WorkflowName::new("公開済み").unwrap(),
            description: None,
            definition:  valid_definition_json(),
            created_by:  user_id(),
            now:         fixed_now(),
        });
        let published = def.published(fixed_now()).unwrap();
        repo.add_definition(published.clone());

        let result = usecase.delete_definition(published.id(), &tid).await;

        assert!(matches!(result, Err(CoreError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_バリデーション成功後に公開が成功する() {
        let (usecase, _repo) = create_usecase();
        let tid = tenant_id();

        let def = usecase
            .create_definition(
                WorkflowName::new("公開予定").unwrap(),
                None,
                valid_definition_json(),
                tid.clone(),
                user_id(),
            )
            .await
            .unwrap();

        let published = usecase
            .publish_definition(def.id(), def.version(), &tid)
            .await
            .unwrap();

        assert_eq!(
            published.status(),
            ringiflow_domain::workflow::WorkflowDefinitionStatus::Published
        );
    }

    #[tokio::test]
    async fn test_バリデーション失敗で公開がエラーを返す() {
        let (usecase, _repo) = create_usecase();
        let tid = tenant_id();

        // バリデーションに失敗する定義（steps が空）
        let def = usecase
            .create_definition(
                WorkflowName::new("不正な定義").unwrap(),
                None,
                json!({"steps": [], "transitions": []}),
                tid.clone(),
                user_id(),
            )
            .await
            .unwrap();

        let result = usecase
            .publish_definition(def.id(), def.version(), &tid)
            .await;

        assert!(matches!(result, Err(CoreError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_published定義のアーカイブが成功する() {
        let (usecase, repo) = create_usecase();
        let tid = tenant_id();

        let def = WorkflowDefinition::new(NewWorkflowDefinition {
            id:          WorkflowDefinitionId::new(),
            tenant_id:   tid.clone(),
            name:        WorkflowName::new("公開済み").unwrap(),
            description: None,
            definition:  valid_definition_json(),
            created_by:  user_id(),
            now:         fixed_now(),
        });
        let published = def.published(fixed_now()).unwrap();
        repo.add_definition(published.clone());

        let archived = usecase
            .archive_definition(published.id(), published.version(), &tid)
            .await
            .unwrap();

        assert_eq!(
            archived.status(),
            ringiflow_domain::workflow::WorkflowDefinitionStatus::Archived
        );
    }

    #[tokio::test]
    async fn test_draft定義のアーカイブがエラーを返す() {
        let (usecase, _repo) = create_usecase();
        let tid = tenant_id();

        let def = usecase
            .create_definition(
                WorkflowName::new("下書き").unwrap(),
                None,
                json!({"steps": []}),
                tid.clone(),
                user_id(),
            )
            .await
            .unwrap();

        let result = usecase
            .archive_definition(def.id(), def.version(), &tid)
            .await;

        assert!(matches!(result, Err(CoreError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_存在しない定義の操作がnotfoundを返す() {
        let (usecase, _repo) = create_usecase();
        let tid = tenant_id();
        let nonexistent_id = WorkflowDefinitionId::new();

        let result = usecase.get_definition(&nonexistent_id, &tid).await;

        assert!(matches!(result, Err(CoreError::NotFound(_))));
    }
}
