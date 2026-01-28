//! # ワークフローユースケース
//!
//! ワークフローの作成・取得・申請に関するビジネスロジックを実装する。

use ringiflow_domain::{
   tenant::TenantId,
   user::UserId,
   value_objects::Version,
   workflow::{
      WorkflowDefinition,
      WorkflowDefinitionId,
      WorkflowInstance,
      WorkflowInstanceId,
      WorkflowInstanceStatus,
      WorkflowStep,
      WorkflowStepId,
   },
};
use ringiflow_infra::repository::{
   WorkflowDefinitionRepository,
   WorkflowInstanceRepository,
   WorkflowStepRepository,
};
use serde_json::Value as JsonValue;

use crate::error::CoreError;

/// ワークフロー作成入力
#[derive(Debug, Clone)]
pub struct CreateWorkflowInput {
   /// ワークフロー定義 ID
   pub definition_id: WorkflowDefinitionId,
   /// ワークフロータイトル
   pub title:         String,
   /// フォームデータ
   pub form_data:     JsonValue,
}

/// ワークフロー申請入力
#[derive(Debug, Clone)]
pub struct SubmitWorkflowInput {
   /// 承認者のユーザー ID
   pub assigned_to: UserId,
}

/// ステップ承認/却下入力
#[derive(Debug, Clone)]
pub struct ApproveRejectInput {
   /// 楽観的ロック用バージョン
   pub version: Version,
   /// コメント（任意）
   pub comment: Option<String>,
}

/// ワークフローユースケース実装
///
/// ワークフローの作成・申請に関するビジネスロジックを実装する。
pub struct WorkflowUseCaseImpl<D, I, S> {
   definition_repo: D,
   instance_repo:   I,
   step_repo:       S,
}

impl<D, I, S> WorkflowUseCaseImpl<D, I, S>
where
   D: WorkflowDefinitionRepository,
   I: WorkflowInstanceRepository,
   S: WorkflowStepRepository,
{
   /// 新しいワークフローユースケースを作成
   pub fn new(definition_repo: D, instance_repo: I, step_repo: S) -> Self {
      Self {
         definition_repo,
         instance_repo,
         step_repo,
      }
   }

   /// ワークフローインスタンスを作成する（下書き）
   ///
   /// ## 処理フロー
   ///
   /// 1. ワークフロー定義が存在するか確認
   /// 2. 公開済み (published) であるか確認
   /// 3. WorkflowInstance を draft として作成
   /// 4. リポジトリに保存
   ///
   /// ## エラー
   ///
   /// - ワークフロー定義が見つからない場合
   /// - ワークフロー定義が公開されていない場合
   /// - データベースエラー
   pub async fn create_workflow(
      &self,
      input: CreateWorkflowInput,
      tenant_id: TenantId,
      user_id: UserId,
   ) -> Result<WorkflowInstance, CoreError> {
      // 1. ワークフロー定義を取得
      let definition = self
         .definition_repo
         .find_by_id(&input.definition_id, &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("定義の取得に失敗: {}", e)))?
         .ok_or_else(|| CoreError::NotFound("ワークフロー定義が見つかりません".to_string()))?;

      // 2. 公開済みであるか確認
      if definition.status() != ringiflow_domain::workflow::WorkflowDefinitionStatus::Published {
         return Err(CoreError::BadRequest(
            "公開されていないワークフロー定義です".to_string(),
         ));
      }

      // 3. WorkflowInstance を draft として作成
      let instance = WorkflowInstance::new(
         tenant_id,
         input.definition_id,
         definition.version(),
         input.title,
         input.form_data,
         user_id,
      );

      // 4. リポジトリに保存
      self
         .instance_repo
         .save(&instance)
         .await
         .map_err(|e| CoreError::Internal(format!("インスタンスの保存に失敗: {}", e)))?;

      Ok(instance)
   }

   /// ワークフローを申請する
   ///
   /// 下書き状態のワークフローを申請状態に遷移させ、
   /// ワークフロー定義に基づいてステップを作成する。
   ///
   /// ## 処理フロー
   ///
   /// 1. ワークフローインスタンスが存在するか確認
   /// 2. draft 状態であるか確認
   /// 3. ワークフロー定義を取得
   /// 4. 定義に基づいてステップを作成 (MVP では1段階承認のみ)
   /// 5. 最初のステップを active に設定
   /// 6. ワークフローインスタンスを pending → in_progress に遷移
   /// 7. インスタンスとステップをリポジトリに保存
   ///
   /// ## エラー
   ///
   /// - ワークフローインスタンスが見つからない場合
   /// - ワークフローインスタンスが draft でない場合
   /// - データベースエラー
   pub async fn submit_workflow(
      &self,
      input: SubmitWorkflowInput,
      instance_id: WorkflowInstanceId,
      tenant_id: TenantId,
   ) -> Result<WorkflowInstance, CoreError> {
      // 1. ワークフローインスタンスを取得
      let instance = self
         .instance_repo
         .find_by_id(&instance_id, &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("インスタンスの取得に失敗: {}", e)))?
         .ok_or_else(|| {
            CoreError::NotFound("ワークフローインスタンスが見つかりません".to_string())
         })?;

      // 2. draft 状態であるか確認
      if instance.status() != WorkflowInstanceStatus::Draft {
         return Err(CoreError::BadRequest(
            "下書き状態のワークフローのみ申請できます".to_string(),
         ));
      }

      // 3. ワークフロー定義を取得（ステップ定義の取得のため）
      let _definition = self
         .definition_repo
         .find_by_id(instance.definition_id(), &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("定義の取得に失敗: {}", e)))?
         .ok_or_else(|| CoreError::NotFound("ワークフロー定義が見つかりません".to_string()))?;

      // 4. ステップを作成 (MVP では1段階承認のみ)
      let step = WorkflowStep::new(
         instance_id.clone(),
         "approval".to_string(),
         "承認".to_string(),
         "approval".to_string(),
         Some(input.assigned_to),
      );

      // 5. ステップを active に設定
      let active_step = step.activated();

      // 6. ワークフローインスタンスを申請済みに遷移
      let submitted_instance = instance
         .submitted()
         .map_err(|e| CoreError::BadRequest(e.to_string()))?;

      // 7. current_step_id を設定して in_progress に遷移
      let in_progress_instance = submitted_instance.with_current_step("approval".to_string());

      // 8. インスタンスとステップを保存
      self
         .instance_repo
         .save(&in_progress_instance)
         .await
         .map_err(|e| CoreError::Internal(format!("インスタンスの保存に失敗: {}", e)))?;

      self
         .step_repo
         .save(&active_step)
         .await
         .map_err(|e| CoreError::Internal(format!("ステップの保存に失敗: {}", e)))?;

      Ok(in_progress_instance)
   }

   // ===== 承認/却下系メソッド =====

   /// ワークフローステップを承認する
   ///
   /// ## 処理フロー
   ///
   /// 1. ステップを取得
   /// 2. 権限チェック（担当者のみ承認可能）
   /// 3. 楽観的ロック（バージョン一致チェック）
   /// 4. ステップを承認
   /// 5. インスタンスを完了に遷移
   /// 6. 保存
   ///
   /// ## エラー
   ///
   /// - ステップが見つからない場合: 404
   /// - 権限がない場合: 403
   /// - Active 以外の場合: 400
   /// - バージョン不一致の場合: 409
   pub async fn approve_step(
      &self,
      input: ApproveRejectInput,
      step_id: WorkflowStepId,
      tenant_id: TenantId,
      user_id: UserId,
   ) -> Result<(), CoreError> {
      // 1. ステップを取得
      let step = self
         .step_repo
         .find_by_id(&step_id, &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?
         .ok_or_else(|| CoreError::NotFound("ステップが見つかりません".to_string()))?;

      // 2. 権限チェック
      if step.assigned_to() != Some(&user_id) {
         return Err(CoreError::Forbidden(
            "このステップを承認する権限がありません".to_string(),
         ));
      }

      // 3. 楽観的ロック（バージョン一致チェック）
      if step.version() != input.version {
         return Err(CoreError::Conflict(
            "ステップは既に更新されています。最新の情報を取得してください。".to_string(),
         ));
      }

      // 4. ステップを承認
      let approved_step = step
         .approve(input.comment)
         .map_err(|e| CoreError::BadRequest(e.to_string()))?;

      // 5. インスタンスを取得して完了に遷移
      let instance = self
         .instance_repo
         .find_by_id(approved_step.instance_id(), &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("インスタンスの取得に失敗: {}", e)))?
         .ok_or_else(|| CoreError::NotFound("インスタンスが見つかりません".to_string()))?;

      let completed_instance = instance
         .complete_with_approval()
         .map_err(|e| CoreError::BadRequest(e.to_string()))?;

      // 6. 保存
      self
         .step_repo
         .save(&approved_step)
         .await
         .map_err(|e| CoreError::Internal(format!("ステップの保存に失敗: {}", e)))?;

      self
         .instance_repo
         .save(&completed_instance)
         .await
         .map_err(|e| CoreError::Internal(format!("インスタンスの保存に失敗: {}", e)))?;

      Ok(())
   }

   /// ワークフローステップを却下する
   ///
   /// ## 処理フロー
   ///
   /// approve_step と同様だが、却下判定で完了する。
   ///
   /// ## エラー
   ///
   /// - ステップが見つからない場合: 404
   /// - 権限がない場合: 403
   /// - Active 以外の場合: 400
   /// - バージョン不一致の場合: 409
   pub async fn reject_step(
      &self,
      input: ApproveRejectInput,
      step_id: WorkflowStepId,
      tenant_id: TenantId,
      user_id: UserId,
   ) -> Result<(), CoreError> {
      // 1. ステップを取得
      let step = self
         .step_repo
         .find_by_id(&step_id, &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?
         .ok_or_else(|| CoreError::NotFound("ステップが見つかりません".to_string()))?;

      // 2. 権限チェック
      if step.assigned_to() != Some(&user_id) {
         return Err(CoreError::Forbidden(
            "このステップを却下する権限がありません".to_string(),
         ));
      }

      // 3. 楽観的ロック（バージョン一致チェック）
      if step.version() != input.version {
         return Err(CoreError::Conflict(
            "ステップは既に更新されています。最新の情報を取得してください。".to_string(),
         ));
      }

      // 4. ステップを却下
      let rejected_step = step
         .reject(input.comment)
         .map_err(|e| CoreError::BadRequest(e.to_string()))?;

      // 5. インスタンスを取得して却下完了に遷移
      let instance = self
         .instance_repo
         .find_by_id(rejected_step.instance_id(), &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("インスタンスの取得に失敗: {}", e)))?
         .ok_or_else(|| CoreError::NotFound("インスタンスが見つかりません".to_string()))?;

      let completed_instance = instance
         .complete_with_rejection()
         .map_err(|e| CoreError::BadRequest(e.to_string()))?;

      // 6. 保存
      self
         .step_repo
         .save(&rejected_step)
         .await
         .map_err(|e| CoreError::Internal(format!("ステップの保存に失敗: {}", e)))?;

      self
         .instance_repo
         .save(&completed_instance)
         .await
         .map_err(|e| CoreError::Internal(format!("インスタンスの保存に失敗: {}", e)))?;

      Ok(())
   }

   // ===== GET 系メソッド =====

   /// 公開済みワークフロー定義一覧を取得する
   ///
   /// フロントエンドのワークフロー申請フォームで、ユーザーが選択可能な
   /// ワークフロー定義の一覧を返す。
   ///
   /// ## 引数
   ///
   /// - `tenant_id`: テナント ID
   ///
   /// ## 戻り値
   ///
   /// - `Ok(Vec<WorkflowDefinition>)`: 公開済み定義の一覧
   /// - `Err(_)`: データベースエラー
   pub async fn list_workflow_definitions(
      &self,
      tenant_id: TenantId,
   ) -> Result<Vec<WorkflowDefinition>, CoreError> {
      self
         .definition_repo
         .find_published_by_tenant(&tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("定義一覧の取得に失敗: {}", e)))
   }

   /// ワークフロー定義の詳細を取得する
   ///
   /// 指定された ID のワークフロー定義を取得する。
   /// 公開済み（published）でない定義も取得可能だが、
   /// フロントエンドでの利用を想定している。
   ///
   /// ## 引数
   ///
   /// - `id`: ワークフロー定義 ID
   /// - `tenant_id`: テナント ID
   ///
   /// ## 戻り値
   ///
   /// - `Ok(definition)`: ワークフロー定義
   /// - `Err(NotFound)`: 定義が見つからない場合
   /// - `Err(_)`: データベースエラー
   pub async fn get_workflow_definition(
      &self,
      id: WorkflowDefinitionId,
      tenant_id: TenantId,
   ) -> Result<WorkflowDefinition, CoreError> {
      self
         .definition_repo
         .find_by_id(&id, &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("定義の取得に失敗: {}", e)))?
         .ok_or_else(|| CoreError::NotFound("ワークフロー定義が見つかりません".to_string()))
   }

   /// 自分の申請一覧を取得する
   ///
   /// ログインユーザーが申請したワークフローインスタンスの一覧を返す。
   ///
   /// ## 引数
   ///
   /// - `tenant_id`: テナント ID
   /// - `user_id`: ユーザー ID
   ///
   /// ## 戻り値
   ///
   /// - `Ok(Vec<WorkflowInstance>)`: 申請一覧
   /// - `Err(_)`: データベースエラー
   pub async fn list_my_workflows(
      &self,
      tenant_id: TenantId,
      user_id: UserId,
   ) -> Result<Vec<WorkflowInstance>, CoreError> {
      self
         .instance_repo
         .find_by_initiated_by(&tenant_id, &user_id)
         .await
         .map_err(|e| CoreError::Internal(format!("申請一覧の取得に失敗: {}", e)))
   }

   /// ワークフローインスタンスの詳細を取得する
   ///
   /// 指定された ID のワークフローインスタンスを取得する。
   ///
   /// ## 引数
   ///
   /// - `id`: ワークフローインスタンス ID
   /// - `tenant_id`: テナント ID
   ///
   /// ## 戻り値
   ///
   /// - `Ok(instance)`: ワークフローインスタンス
   /// - `Err(NotFound)`: インスタンスが見つからない場合
   /// - `Err(_)`: データベースエラー
   pub async fn get_workflow(
      &self,
      id: WorkflowInstanceId,
      tenant_id: TenantId,
   ) -> Result<WorkflowInstance, CoreError> {
      self
         .instance_repo
         .find_by_id(&id, &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("インスタンスの取得に失敗: {}", e)))?
         .ok_or_else(|| CoreError::NotFound("ワークフローインスタンスが見つかりません".to_string()))
   }
}

#[cfg(test)]
mod tests {
   use std::sync::{Arc, Mutex};

   use ringiflow_domain::{
      value_objects::{Version, WorkflowName},
      workflow::{WorkflowDefinition, WorkflowDefinitionStatus},
   };
   use ringiflow_infra::error::InfraError;

   use super::*;

   // Mock リポジトリ

   #[derive(Clone)]
   struct MockWorkflowDefinitionRepository {
      definitions: Arc<Mutex<Vec<WorkflowDefinition>>>,
   }

   impl MockWorkflowDefinitionRepository {
      fn new() -> Self {
         Self {
            definitions: Arc::new(Mutex::new(Vec::new())),
         }
      }

      fn add_definition(&self, def: WorkflowDefinition) {
         self.definitions.lock().unwrap().push(def);
      }
   }

   #[async_trait::async_trait]
   impl WorkflowDefinitionRepository for MockWorkflowDefinitionRepository {
      async fn find_published_by_tenant(
         &self,
         tenant_id: &TenantId,
      ) -> Result<Vec<WorkflowDefinition>, InfraError> {
         Ok(self
            .definitions
            .lock()
            .unwrap()
            .iter()
            .filter(|d| {
               d.tenant_id() == tenant_id && d.status() == WorkflowDefinitionStatus::Published
            })
            .cloned()
            .collect())
      }

      async fn find_by_id(
         &self,
         id: &WorkflowDefinitionId,
         tenant_id: &TenantId,
      ) -> Result<Option<WorkflowDefinition>, InfraError> {
         Ok(self
            .definitions
            .lock()
            .unwrap()
            .iter()
            .find(|d| d.id() == id && d.tenant_id() == tenant_id)
            .cloned())
      }
   }

   #[derive(Clone)]
   struct MockWorkflowInstanceRepository {
      instances: Arc<Mutex<Vec<WorkflowInstance>>>,
   }

   impl MockWorkflowInstanceRepository {
      fn new() -> Self {
         Self {
            instances: Arc::new(Mutex::new(Vec::new())),
         }
      }
   }

   #[async_trait::async_trait]
   impl WorkflowInstanceRepository for MockWorkflowInstanceRepository {
      async fn save(&self, instance: &WorkflowInstance) -> Result<(), InfraError> {
         let mut instances = self.instances.lock().unwrap();
         if let Some(pos) = instances.iter().position(|i| i.id() == instance.id()) {
            instances[pos] = instance.clone();
         } else {
            instances.push(instance.clone());
         }
         Ok(())
      }

      async fn find_by_id(
         &self,
         id: &WorkflowInstanceId,
         tenant_id: &TenantId,
      ) -> Result<Option<WorkflowInstance>, InfraError> {
         Ok(self
            .instances
            .lock()
            .unwrap()
            .iter()
            .find(|i| i.id() == id && i.tenant_id() == tenant_id)
            .cloned())
      }

      async fn find_by_tenant(
         &self,
         tenant_id: &TenantId,
      ) -> Result<Vec<WorkflowInstance>, InfraError> {
         Ok(self
            .instances
            .lock()
            .unwrap()
            .iter()
            .filter(|i| i.tenant_id() == tenant_id)
            .cloned()
            .collect())
      }

      async fn find_by_initiated_by(
         &self,
         tenant_id: &TenantId,
         user_id: &UserId,
      ) -> Result<Vec<WorkflowInstance>, InfraError> {
         Ok(self
            .instances
            .lock()
            .unwrap()
            .iter()
            .filter(|i| i.tenant_id() == tenant_id && i.initiated_by() == user_id)
            .cloned()
            .collect())
      }
   }

   #[derive(Clone)]
   struct MockWorkflowStepRepository {
      steps: Arc<Mutex<Vec<WorkflowStep>>>,
   }

   impl MockWorkflowStepRepository {
      fn new() -> Self {
         Self {
            steps: Arc::new(Mutex::new(Vec::new())),
         }
      }
   }

   #[async_trait::async_trait]
   impl WorkflowStepRepository for MockWorkflowStepRepository {
      async fn save(&self, step: &WorkflowStep) -> Result<(), InfraError> {
         let mut steps = self.steps.lock().unwrap();
         if let Some(pos) = steps.iter().position(|s| s.id() == step.id()) {
            steps[pos] = step.clone();
         } else {
            steps.push(step.clone());
         }
         Ok(())
      }

      async fn find_by_id(
         &self,
         id: &ringiflow_domain::workflow::WorkflowStepId,
         _tenant_id: &TenantId,
      ) -> Result<Option<WorkflowStep>, InfraError> {
         // MockではテナントIDチェックを簡略化
         Ok(self
            .steps
            .lock()
            .unwrap()
            .iter()
            .find(|s| s.id() == id)
            .cloned())
      }

      async fn find_by_instance(
         &self,
         instance_id: &WorkflowInstanceId,
         _tenant_id: &TenantId,
      ) -> Result<Vec<WorkflowStep>, InfraError> {
         Ok(self
            .steps
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.instance_id() == instance_id)
            .cloned()
            .collect())
      }

      async fn find_by_assigned_to(
         &self,
         _tenant_id: &TenantId,
         user_id: &UserId,
      ) -> Result<Vec<WorkflowStep>, InfraError> {
         Ok(self
            .steps
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.assigned_to() == Some(user_id))
            .cloned()
            .collect())
      }
   }

   #[tokio::test]
   async fn test_create_workflow_正常系() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      // 公開済みの定義を追加
      let definition = WorkflowDefinition::new(
         tenant_id.clone(),
         WorkflowName::new("汎用申請").unwrap(),
         Some("テスト用定義".to_string()),
         serde_json::json!({"steps": []}),
         user_id.clone(),
      );
      let published_definition = definition.published().unwrap();
      definition_repo.add_definition(published_definition.clone());

      let usecase = WorkflowUseCaseImpl::new(definition_repo, instance_repo.clone(), step_repo);

      let input = CreateWorkflowInput {
         definition_id: published_definition.id().clone(),
         title:         "テスト申請".to_string(),
         form_data:     serde_json::json!({"note": "test"}),
      };

      // Act
      let result = usecase
         .create_workflow(input, tenant_id.clone(), user_id.clone())
         .await;

      // Assert
      assert!(result.is_ok());
      let instance = result.unwrap();
      assert_eq!(instance.status(), WorkflowInstanceStatus::Draft);
      assert_eq!(instance.title(), "テスト申請");
      assert_eq!(instance.initiated_by(), &user_id);

      // リポジトリに保存されていることを確認
      let saved = instance_repo
         .find_by_id(instance.id(), &tenant_id)
         .await
         .unwrap();
      assert!(saved.is_some());
   }

   #[tokio::test]
   async fn test_create_workflow_定義が見つからない() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      let usecase = WorkflowUseCaseImpl::new(definition_repo, instance_repo, step_repo);

      let input = CreateWorkflowInput {
         definition_id: WorkflowDefinitionId::new(),
         title:         "テスト申請".to_string(),
         form_data:     serde_json::json!({}),
      };

      // Act
      let result = usecase.create_workflow(input, tenant_id, user_id).await;

      // Assert
      assert!(matches!(result, Err(CoreError::NotFound(_))));
   }

   // ===== approve_step テスト =====

   #[tokio::test]
   async fn test_approve_step_正常系() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver_id = UserId::new();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      // InProgress のインスタンスを作成
      let instance = WorkflowInstance::new(
         tenant_id.clone(),
         WorkflowDefinitionId::new(),
         Version::initial(),
         "テスト申請".to_string(),
         serde_json::json!({}),
         user_id.clone(),
      )
      .submitted()
      .unwrap()
      .with_current_step("approval".to_string());
      instance_repo.save(&instance).await.unwrap();

      // Active なステップを作成
      let step = WorkflowStep::new(
         instance.id().clone(),
         "approval".to_string(),
         "承認".to_string(),
         "approval".to_string(),
         Some(approver_id.clone()),
      )
      .activated();
      step_repo.save(&step).await.unwrap();

      let usecase =
         WorkflowUseCaseImpl::new(definition_repo, instance_repo.clone(), step_repo.clone());

      let input = ApproveRejectInput {
         version: step.version(),
         comment: Some("承認しました".to_string()),
      };

      // Act
      let result = usecase
         .approve_step(
            input,
            step.id().clone(),
            tenant_id.clone(),
            approver_id.clone(),
         )
         .await;

      // Assert
      assert!(result.is_ok());

      // ステップが Completed (Approved) になっていることを確認
      let updated_step = step_repo
         .find_by_id(step.id(), &tenant_id)
         .await
         .unwrap()
         .unwrap();
      assert_eq!(
         updated_step.status(),
         ringiflow_domain::workflow::WorkflowStepStatus::Completed
      );
      assert_eq!(
         updated_step.decision(),
         Some(ringiflow_domain::workflow::StepDecision::Approved)
      );

      // インスタンスが Approved になっていることを確認
      let updated_instance = instance_repo
         .find_by_id(instance.id(), &tenant_id)
         .await
         .unwrap()
         .unwrap();
      assert_eq!(updated_instance.status(), WorkflowInstanceStatus::Approved);
   }

   #[tokio::test]
   async fn test_approve_step_未割り当てユーザーは403() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver_id = UserId::new();
      let other_user_id = UserId::new(); // 別のユーザー

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      let instance = WorkflowInstance::new(
         tenant_id.clone(),
         WorkflowDefinitionId::new(),
         Version::initial(),
         "テスト申請".to_string(),
         serde_json::json!({}),
         user_id.clone(),
      )
      .submitted()
      .unwrap()
      .with_current_step("approval".to_string());
      instance_repo.save(&instance).await.unwrap();

      let step = WorkflowStep::new(
         instance.id().clone(),
         "approval".to_string(),
         "承認".to_string(),
         "approval".to_string(),
         Some(approver_id.clone()), // approver_id に割り当て
      )
      .activated();
      step_repo.save(&step).await.unwrap();

      let usecase = WorkflowUseCaseImpl::new(definition_repo, instance_repo, step_repo);

      let input = ApproveRejectInput {
         version: step.version(),
         comment: None,
      };

      // Act: 別のユーザーで承認を試みる
      let result = usecase
         .approve_step(input, step.id().clone(), tenant_id, other_user_id)
         .await;

      // Assert
      assert!(matches!(result, Err(CoreError::Forbidden(_))));
   }

   #[tokio::test]
   async fn test_approve_step_active以外は400() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver_id = UserId::new();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      let instance = WorkflowInstance::new(
         tenant_id.clone(),
         WorkflowDefinitionId::new(),
         Version::initial(),
         "テスト申請".to_string(),
         serde_json::json!({}),
         user_id.clone(),
      )
      .submitted()
      .unwrap()
      .with_current_step("approval".to_string());
      instance_repo.save(&instance).await.unwrap();

      // Pending 状態のステップ（Active ではない）
      let step = WorkflowStep::new(
         instance.id().clone(),
         "approval".to_string(),
         "承認".to_string(),
         "approval".to_string(),
         Some(approver_id.clone()),
      );
      // activated() を呼ばないので Pending のまま
      step_repo.save(&step).await.unwrap();

      let usecase = WorkflowUseCaseImpl::new(definition_repo, instance_repo, step_repo);

      let input = ApproveRejectInput {
         version: step.version(),
         comment: None,
      };

      // Act
      let result = usecase
         .approve_step(input, step.id().clone(), tenant_id, approver_id)
         .await;

      // Assert
      assert!(matches!(result, Err(CoreError::BadRequest(_))));
   }

   #[tokio::test]
   async fn test_approve_step_バージョン不一致で409() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver_id = UserId::new();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      let instance = WorkflowInstance::new(
         tenant_id.clone(),
         WorkflowDefinitionId::new(),
         Version::initial(),
         "テスト申請".to_string(),
         serde_json::json!({}),
         user_id.clone(),
      )
      .submitted()
      .unwrap()
      .with_current_step("approval".to_string());
      instance_repo.save(&instance).await.unwrap();

      let step = WorkflowStep::new(
         instance.id().clone(),
         "approval".to_string(),
         "承認".to_string(),
         "approval".to_string(),
         Some(approver_id.clone()),
      )
      .activated();
      step_repo.save(&step).await.unwrap();

      let usecase = WorkflowUseCaseImpl::new(definition_repo, instance_repo, step_repo);

      // 不一致バージョンを指定（ステップの version は 1 だが、2 を指定）
      let wrong_version = Version::initial().next();
      let input = ApproveRejectInput {
         version: wrong_version,
         comment: None,
      };

      // Act
      let result = usecase
         .approve_step(input, step.id().clone(), tenant_id, approver_id)
         .await;

      // Assert
      assert!(matches!(result, Err(CoreError::Conflict(_))));
   }

   // ===== reject_step テスト =====

   #[tokio::test]
   async fn test_reject_step_正常系() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver_id = UserId::new();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      let instance = WorkflowInstance::new(
         tenant_id.clone(),
         WorkflowDefinitionId::new(),
         Version::initial(),
         "テスト申請".to_string(),
         serde_json::json!({}),
         user_id.clone(),
      )
      .submitted()
      .unwrap()
      .with_current_step("approval".to_string());
      instance_repo.save(&instance).await.unwrap();

      let step = WorkflowStep::new(
         instance.id().clone(),
         "approval".to_string(),
         "承認".to_string(),
         "approval".to_string(),
         Some(approver_id.clone()),
      )
      .activated();
      step_repo.save(&step).await.unwrap();

      let usecase =
         WorkflowUseCaseImpl::new(definition_repo, instance_repo.clone(), step_repo.clone());

      let input = ApproveRejectInput {
         version: step.version(),
         comment: Some("却下理由".to_string()),
      };

      // Act
      let result = usecase
         .reject_step(
            input,
            step.id().clone(),
            tenant_id.clone(),
            approver_id.clone(),
         )
         .await;

      // Assert
      assert!(result.is_ok());

      // ステップが Completed (Rejected) になっていることを確認
      let updated_step = step_repo
         .find_by_id(step.id(), &tenant_id)
         .await
         .unwrap()
         .unwrap();
      assert_eq!(
         updated_step.status(),
         ringiflow_domain::workflow::WorkflowStepStatus::Completed
      );
      assert_eq!(
         updated_step.decision(),
         Some(ringiflow_domain::workflow::StepDecision::Rejected)
      );

      // インスタンスが Rejected になっていることを確認
      let updated_instance = instance_repo
         .find_by_id(instance.id(), &tenant_id)
         .await
         .unwrap()
         .unwrap();
      assert_eq!(updated_instance.status(), WorkflowInstanceStatus::Rejected);
   }

   #[tokio::test]
   async fn test_submit_workflow_正常系() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver_id = UserId::new();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      // 公開済みの定義を追加
      let definition = WorkflowDefinition::new(
         tenant_id.clone(),
         WorkflowName::new("汎用申請").unwrap(),
         Some("テスト用定義".to_string()),
         serde_json::json!({"steps": []}),
         user_id.clone(),
      );
      let published_definition = definition.published().unwrap();
      definition_repo.add_definition(published_definition.clone());

      // 下書きのインスタンスを作成
      let instance = WorkflowInstance::new(
         tenant_id.clone(),
         published_definition.id().clone(),
         Version::initial(),
         "テスト申請".to_string(),
         serde_json::json!({}),
         user_id.clone(),
      );
      instance_repo.save(&instance).await.unwrap();

      let usecase =
         WorkflowUseCaseImpl::new(definition_repo, instance_repo.clone(), step_repo.clone());

      let input = SubmitWorkflowInput {
         assigned_to: approver_id.clone(),
      };

      // Act
      let result = usecase
         .submit_workflow(input, instance.id().clone(), tenant_id.clone())
         .await;

      // Assert
      assert!(result.is_ok());
      let submitted = result.unwrap();
      assert_eq!(submitted.status(), WorkflowInstanceStatus::InProgress);
      assert_eq!(submitted.current_step_id(), Some("approval"));
      assert!(submitted.submitted_at().is_some());

      // ステップが作成されていることを確認
      let steps = step_repo
         .find_by_instance(submitted.id(), &tenant_id)
         .await
         .unwrap();
      assert_eq!(steps.len(), 1);
      assert_eq!(steps[0].assigned_to(), Some(&approver_id));
   }
}
