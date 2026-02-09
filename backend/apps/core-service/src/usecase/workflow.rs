//! # ワークフローユースケース
//!
//! ワークフローの作成・取得・申請に関するビジネスロジックを実装する。

use std::{collections::HashMap, sync::Arc};

use itertools::Itertools;
use ringiflow_domain::{
   tenant::TenantId,
   user::UserId,
   value_objects::{DisplayIdEntityType, DisplayNumber, Version},
   workflow::{
      NewWorkflowInstance,
      NewWorkflowStep,
      WorkflowDefinition,
      WorkflowDefinitionId,
      WorkflowInstance,
      WorkflowInstanceId,
      WorkflowInstanceStatus,
      WorkflowStep,
      WorkflowStepId,
   },
};
use ringiflow_infra::{
   InfraError,
   repository::{
      DisplayIdCounterRepository,
      UserRepository,
      WorkflowDefinitionRepository,
      WorkflowInstanceRepository,
      WorkflowStepRepository,
   },
};
use serde_json::Value as JsonValue;

use crate::error::CoreError;

/// ユースケースの出力: ワークフローインスタンスとステップの集約
///
/// ドメインモデル (`WorkflowInstance`, `WorkflowStep`) を変更せず、
/// ユースケースの出力として集約する。詳細取得や承認/却下の結果を
/// ハンドラに返す際に使用する。
pub struct WorkflowWithSteps {
   pub instance: WorkflowInstance,
   pub steps:    Vec<WorkflowStep>,
}

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

/// WorkflowInstance + Steps からユーザー ID を収集する
///
/// ワークフローの initiated_by と各ステップの assigned_to を
/// 重複排除して返す。ユーザー名一括解決の前処理として使用する。
pub(crate) fn collect_user_ids_from_workflow(
   instance: &WorkflowInstance,
   steps: &[WorkflowStep],
) -> Vec<UserId> {
   std::iter::once(instance.initiated_by().clone())
      .chain(steps.iter().filter_map(|s| s.assigned_to().cloned()))
      .unique()
      .collect()
}

/// ワークフローユースケース実装
///
/// ワークフローの作成・申請に関するビジネスロジックを実装する。
pub struct WorkflowUseCaseImpl {
   definition_repo: Arc<dyn WorkflowDefinitionRepository>,
   instance_repo:   Arc<dyn WorkflowInstanceRepository>,
   step_repo:       Arc<dyn WorkflowStepRepository>,
   user_repo:       Arc<dyn UserRepository>,
   counter_repo:    Arc<dyn DisplayIdCounterRepository>,
}

impl WorkflowUseCaseImpl {
   /// 新しいワークフローユースケースを作成
   pub fn new(
      definition_repo: Arc<dyn WorkflowDefinitionRepository>,
      instance_repo: Arc<dyn WorkflowInstanceRepository>,
      step_repo: Arc<dyn WorkflowStepRepository>,
      user_repo: Arc<dyn UserRepository>,
      counter_repo: Arc<dyn DisplayIdCounterRepository>,
   ) -> Self {
      Self {
         definition_repo,
         instance_repo,
         step_repo,
         user_repo,
         counter_repo,
      }
   }

   /// ユーザー ID のリストからユーザー名を一括解決する
   pub async fn resolve_user_names(
      &self,
      user_ids: &[UserId],
   ) -> Result<HashMap<UserId, String>, CoreError> {
      crate::usecase::resolve_user_names(self.user_repo.as_ref(), user_ids).await
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
      let now = chrono::Utc::now();
      let display_number = self
         .counter_repo
         .next_display_number(&tenant_id, DisplayIdEntityType::WorkflowInstance)
         .await
         .map_err(|e| CoreError::Internal(format!("採番に失敗: {}", e)))?;
      let instance = WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id,
         definition_id: input.definition_id,
         definition_version: definition.version(),
         display_number,
         title: input.title,
         form_data: input.form_data,
         initiated_by: user_id,
         now,
      });

      // 4. リポジトリに保存
      self
         .instance_repo
         .insert(&instance)
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
      let now = chrono::Utc::now();
      let display_number = self
         .counter_repo
         .next_display_number(&tenant_id, DisplayIdEntityType::WorkflowStep)
         .await
         .map_err(|e| CoreError::Internal(format!("採番に失敗: {}", e)))?;
      let step = WorkflowStep::new(NewWorkflowStep {
         id: WorkflowStepId::new(),
         instance_id: instance_id.clone(),
         display_number,
         step_id: "approval".to_string(),
         step_name: "承認".to_string(),
         step_type: "approval".to_string(),
         assigned_to: Some(input.assigned_to),
         now,
      });

      // 5. ステップを active に設定
      let active_step = step.activated(now);

      // 6. ワークフローインスタンスを申請済みに遷移
      let now = chrono::Utc::now();
      let expected_version = instance.version();
      let submitted_instance = instance
         .submitted(now)
         .map_err(|e| CoreError::BadRequest(e.to_string()))?;

      // 7. current_step_id を設定して in_progress に遷移
      let in_progress_instance = submitted_instance.with_current_step("approval".to_string(), now);

      // 8. インスタンスとステップを保存
      self
         .instance_repo
         .update_with_version_check(&in_progress_instance, expected_version)
         .await
         .map_err(|e| match e {
            InfraError::Conflict { .. } => CoreError::Conflict(
               "インスタンスは既に更新されています。最新の情報を取得してください。".to_string(),
            ),
            other => CoreError::Internal(format!("インスタンスの保存に失敗: {}", other)),
         })?;

      self
         .step_repo
         .insert(&active_step)
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
   ) -> Result<WorkflowWithSteps, CoreError> {
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

      // 3. 楽観的ロック（バージョン一致チェック — 早期フェイル）
      if step.version() != input.version {
         return Err(CoreError::Conflict(
            "ステップは既に更新されています。最新の情報を取得してください。".to_string(),
         ));
      }

      // 4. ステップを承認
      let now = chrono::Utc::now();
      let step_expected_version = step.version();
      let approved_step = step
         .approve(input.comment, now)
         .map_err(|e| CoreError::BadRequest(e.to_string()))?;

      // 5. インスタンスを取得して完了に遷移
      let instance = self
         .instance_repo
         .find_by_id(approved_step.instance_id(), &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("インスタンスの取得に失敗: {}", e)))?
         .ok_or_else(|| CoreError::NotFound("インスタンスが見つかりません".to_string()))?;

      let instance_expected_version = instance.version();
      let completed_instance = instance
         .complete_with_approval(now)
         .map_err(|e| CoreError::BadRequest(e.to_string()))?;

      // 6. 楽観的ロック付きで保存
      self
         .step_repo
         .update_with_version_check(&approved_step, step_expected_version)
         .await
         .map_err(|e| match e {
            InfraError::Conflict { .. } => CoreError::Conflict(
               "ステップは既に更新されています。最新の情報を取得してください。".to_string(),
            ),
            other => CoreError::Internal(format!("ステップの保存に失敗: {}", other)),
         })?;

      self
         .instance_repo
         .update_with_version_check(&completed_instance, instance_expected_version)
         .await
         .map_err(|e| match e {
            InfraError::Conflict { .. } => CoreError::Conflict(
               "インスタンスは既に更新されています。最新の情報を取得してください。".to_string(),
            ),
            other => CoreError::Internal(format!("インスタンスの保存に失敗: {}", other)),
         })?;

      // 7. 保存後のステップ一覧を取得して返却
      let steps = self
         .step_repo
         .find_by_instance(completed_instance.id(), &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?;

      Ok(WorkflowWithSteps {
         instance: completed_instance,
         steps,
      })
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
   ) -> Result<WorkflowWithSteps, CoreError> {
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

      // 3. 楽観的ロック（バージョン一致チェック — 早期フェイル）
      if step.version() != input.version {
         return Err(CoreError::Conflict(
            "ステップは既に更新されています。最新の情報を取得してください。".to_string(),
         ));
      }

      // 4. ステップを却下
      let now = chrono::Utc::now();
      let step_expected_version = step.version();
      let rejected_step = step
         .reject(input.comment, now)
         .map_err(|e| CoreError::BadRequest(e.to_string()))?;

      // 5. インスタンスを取得して却下完了に遷移
      let instance = self
         .instance_repo
         .find_by_id(rejected_step.instance_id(), &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("インスタンスの取得に失敗: {}", e)))?
         .ok_or_else(|| CoreError::NotFound("インスタンスが見つかりません".to_string()))?;

      let instance_expected_version = instance.version();
      let completed_instance = instance
         .complete_with_rejection(now)
         .map_err(|e| CoreError::BadRequest(e.to_string()))?;

      // 6. 楽観的ロック付きで保存
      self
         .step_repo
         .update_with_version_check(&rejected_step, step_expected_version)
         .await
         .map_err(|e| match e {
            InfraError::Conflict { .. } => CoreError::Conflict(
               "ステップは既に更新されています。最新の情報を取得してください。".to_string(),
            ),
            other => CoreError::Internal(format!("ステップの保存に失敗: {}", other)),
         })?;

      self
         .instance_repo
         .update_with_version_check(&completed_instance, instance_expected_version)
         .await
         .map_err(|e| match e {
            InfraError::Conflict { .. } => CoreError::Conflict(
               "インスタンスは既に更新されています。最新の情報を取得してください。".to_string(),
            ),
            other => CoreError::Internal(format!("インスタンスの保存に失敗: {}", other)),
         })?;

      // 7. 保存後のステップ一覧を取得して返却
      let steps = self
         .step_repo
         .find_by_instance(completed_instance.id(), &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?;

      Ok(WorkflowWithSteps {
         instance: completed_instance,
         steps,
      })
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
   ) -> Result<WorkflowWithSteps, CoreError> {
      let instance = self
         .instance_repo
         .find_by_id(&id, &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("インスタンスの取得に失敗: {}", e)))?
         .ok_or_else(|| {
            CoreError::NotFound("ワークフローインスタンスが見つかりません".to_string())
         })?;

      let steps = self
         .step_repo
         .find_by_instance(&id, &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?;

      Ok(WorkflowWithSteps { instance, steps })
   }

   // ===== display_number 対応メソッド =====

   /// display_number でワークフローインスタンスの詳細を取得する
   ///
   /// BFF が公開 API で display_number を使う場合に、
   /// 1回の呼び出しでワークフロー詳細を返す。
   ///
   /// ## 引数
   ///
   /// - `display_number`: 表示用連番
   /// - `tenant_id`: テナント ID
   ///
   /// ## 戻り値
   ///
   /// - `Ok(workflow)`: ワークフロー詳細（インスタンス + ステップ）
   /// - `Err(NotFound)`: インスタンスが見つからない場合
   /// - `Err(_)`: データベースエラー
   pub async fn get_workflow_by_display_number(
      &self,
      display_number: DisplayNumber,
      tenant_id: TenantId,
   ) -> Result<WorkflowWithSteps, CoreError> {
      let instance = self
         .instance_repo
         .find_by_display_number(display_number, &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("インスタンスの取得に失敗: {}", e)))?
         .ok_or_else(|| {
            CoreError::NotFound("ワークフローインスタンスが見つかりません".to_string())
         })?;

      let steps = self
         .step_repo
         .find_by_instance(instance.id(), &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?;

      Ok(WorkflowWithSteps { instance, steps })
   }

   /// display_number でワークフローを申請する
   ///
   /// BFF が公開 API で display_number を使う場合に、
   /// 1回の呼び出しで申請を完了する。
   ///
   /// ## 引数
   ///
   /// - `input`: 申請入力
   /// - `display_number`: 表示用連番
   /// - `tenant_id`: テナント ID
   ///
   /// ## 戻り値
   ///
   /// - `Ok(instance)`: 申請後のワークフローインスタンス
   /// - `Err(NotFound)`: インスタンスが見つからない場合
   /// - `Err(_)`: データベースエラー
   pub async fn submit_workflow_by_display_number(
      &self,
      input: SubmitWorkflowInput,
      display_number: DisplayNumber,
      tenant_id: TenantId,
   ) -> Result<WorkflowInstance, CoreError> {
      // display_number → WorkflowInstanceId を解決
      let instance = self
         .instance_repo
         .find_by_display_number(display_number, &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("インスタンスの取得に失敗: {}", e)))?
         .ok_or_else(|| {
            CoreError::NotFound("ワークフローインスタンスが見つかりません".to_string())
         })?;

      // 既存の submit_workflow を呼び出し
      self
         .submit_workflow(input, instance.id().clone(), tenant_id)
         .await
   }

   /// display_number でワークフローステップを承認する
   ///
   /// BFF が公開 API で display_number を使う場合に、
   /// 1回の呼び出しでステップ承認を完了する。
   ///
   /// ## 引数
   ///
   /// - `input`: 承認入力
   /// - `workflow_display_number`: ワークフローの表示用連番
   /// - `step_display_number`: ステップの表示用連番
   /// - `tenant_id`: テナント ID
   /// - `user_id`: 操作ユーザー ID
   ///
   /// ## 戻り値
   ///
   /// - `Ok(workflow)`: 承認後のワークフロー詳細
   /// - `Err(NotFound)`: インスタンスまたはステップが見つからない場合
   /// - `Err(_)`: データベースエラー
   pub async fn approve_step_by_display_number(
      &self,
      input: ApproveRejectInput,
      workflow_display_number: DisplayNumber,
      step_display_number: DisplayNumber,
      tenant_id: TenantId,
      user_id: UserId,
   ) -> Result<WorkflowWithSteps, CoreError> {
      // display_number → WorkflowInstanceId を解決
      let instance = self
         .instance_repo
         .find_by_display_number(workflow_display_number, &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("インスタンスの取得に失敗: {}", e)))?
         .ok_or_else(|| {
            CoreError::NotFound("ワークフローインスタンスが見つかりません".to_string())
         })?;

      // display_number → WorkflowStepId を解決
      let step = self
         .step_repo
         .find_by_display_number(step_display_number, instance.id(), &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?
         .ok_or_else(|| CoreError::NotFound("ステップが見つかりません".to_string()))?;

      // 既存の approve_step を呼び出し
      self
         .approve_step(input, step.id().clone(), tenant_id, user_id)
         .await
   }

   /// display_number でワークフローステップを却下する
   ///
   /// BFF が公開 API で display_number を使う場合に、
   /// 1回の呼び出しでステップ却下を完了する。
   ///
   /// ## 引数
   ///
   /// - `input`: 却下入力
   /// - `workflow_display_number`: ワークフローの表示用連番
   /// - `step_display_number`: ステップの表示用連番
   /// - `tenant_id`: テナント ID
   /// - `user_id`: 操作ユーザー ID
   ///
   /// ## 戻り値
   ///
   /// - `Ok(workflow)`: 却下後のワークフロー詳細
   /// - `Err(NotFound)`: インスタンスまたはステップが見つからない場合
   /// - `Err(_)`: データベースエラー
   pub async fn reject_step_by_display_number(
      &self,
      input: ApproveRejectInput,
      workflow_display_number: DisplayNumber,
      step_display_number: DisplayNumber,
      tenant_id: TenantId,
      user_id: UserId,
   ) -> Result<WorkflowWithSteps, CoreError> {
      // display_number → WorkflowInstanceId を解決
      let instance = self
         .instance_repo
         .find_by_display_number(workflow_display_number, &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("インスタンスの取得に失敗: {}", e)))?
         .ok_or_else(|| {
            CoreError::NotFound("ワークフローインスタンスが見つかりません".to_string())
         })?;

      // display_number → WorkflowStepId を解決
      let step = self
         .step_repo
         .find_by_display_number(step_display_number, instance.id(), &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?
         .ok_or_else(|| CoreError::NotFound("ステップが見つかりません".to_string()))?;

      // 既存の reject_step を呼び出し
      self
         .reject_step(input, step.id().clone(), tenant_id, user_id)
         .await
   }
}

#[cfg(test)]
mod tests {
   use std::sync::{Arc, Mutex};

   use ringiflow_domain::{
      user::User,
      value_objects::{DisplayNumber, Version, WorkflowName},
      workflow::{
         NewWorkflowDefinition,
         NewWorkflowInstance,
         NewWorkflowStep,
         WorkflowDefinition,
         WorkflowDefinitionStatus,
      },
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
      async fn insert(&self, instance: &WorkflowInstance) -> Result<(), InfraError> {
         let mut instances = self.instances.lock().unwrap();
         instances.push(instance.clone());
         Ok(())
      }

      async fn update_with_version_check(
         &self,
         instance: &WorkflowInstance,
         expected_version: Version,
      ) -> Result<(), InfraError> {
         let mut instances = self.instances.lock().unwrap();
         if let Some(pos) = instances.iter().position(|i| i.id() == instance.id()) {
            if instances[pos].version() != expected_version {
               return Err(InfraError::Conflict {
                  entity: "WorkflowInstance".to_string(),
                  id:     instance.id().as_uuid().to_string(),
               });
            }
            instances[pos] = instance.clone();
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

      async fn find_by_ids(
         &self,
         ids: &[WorkflowInstanceId],
         tenant_id: &TenantId,
      ) -> Result<Vec<WorkflowInstance>, InfraError> {
         Ok(self
            .instances
            .lock()
            .unwrap()
            .iter()
            .filter(|i| ids.contains(i.id()) && i.tenant_id() == tenant_id)
            .cloned()
            .collect())
      }

      async fn find_by_display_number(
         &self,
         display_number: DisplayNumber,
         tenant_id: &TenantId,
      ) -> Result<Option<WorkflowInstance>, InfraError> {
         Ok(self
            .instances
            .lock()
            .unwrap()
            .iter()
            .find(|i| i.display_number() == display_number && i.tenant_id() == tenant_id)
            .cloned())
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
      async fn insert(&self, step: &WorkflowStep) -> Result<(), InfraError> {
         let mut steps = self.steps.lock().unwrap();
         steps.push(step.clone());
         Ok(())
      }

      async fn update_with_version_check(
         &self,
         step: &WorkflowStep,
         expected_version: Version,
      ) -> Result<(), InfraError> {
         let mut steps = self.steps.lock().unwrap();
         if let Some(pos) = steps.iter().position(|s| s.id() == step.id()) {
            if steps[pos].version() != expected_version {
               return Err(InfraError::Conflict {
                  entity: "WorkflowStep".to_string(),
                  id:     step.id().as_uuid().to_string(),
               });
            }
            steps[pos] = step.clone();
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

      async fn find_by_display_number(
         &self,
         display_number: DisplayNumber,
         instance_id: &WorkflowInstanceId,
         _tenant_id: &TenantId,
      ) -> Result<Option<WorkflowStep>, InfraError> {
         Ok(self
            .steps
            .lock()
            .unwrap()
            .iter()
            .find(|s| s.display_number() == display_number && s.instance_id() == instance_id)
            .cloned())
      }
   }

   /// テスト用のモック UserRepository
   ///
   /// ユーザー名解決テストが必要な場合に使用する。
   /// ワークフローユースケースのテストでは直接利用しないが、型パラメータを満たすために必要。
   #[derive(Clone)]
   struct MockUserRepository;

   #[async_trait::async_trait]
   impl ringiflow_infra::repository::UserRepository for MockUserRepository {
      async fn find_by_email(
         &self,
         _tenant_id: &TenantId,
         _email: &ringiflow_domain::user::Email,
      ) -> Result<Option<User>, InfraError> {
         Ok(None)
      }

      async fn find_by_id(&self, _id: &UserId) -> Result<Option<User>, InfraError> {
         Ok(None)
      }

      async fn find_with_roles(
         &self,
         _id: &UserId,
      ) -> Result<Option<(User, Vec<ringiflow_domain::role::Role>)>, InfraError> {
         Ok(None)
      }

      async fn find_by_ids(&self, _ids: &[UserId]) -> Result<Vec<User>, InfraError> {
         Ok(Vec::new())
      }

      async fn find_all_active_by_tenant(
         &self,
         _tenant_id: &TenantId,
      ) -> Result<Vec<User>, InfraError> {
         Ok(Vec::new())
      }

      async fn update_last_login(&self, _id: &UserId) -> Result<(), InfraError> {
         Ok(())
      }
   }

   /// テスト用のモック DisplayIdCounterRepository
   ///
   /// 呼び出しごとにカウンターをインクリメントして返す。
   #[derive(Clone)]
   struct MockDisplayIdCounterRepository {
      counter: Arc<Mutex<i64>>,
   }

   impl MockDisplayIdCounterRepository {
      fn new() -> Self {
         Self {
            counter: Arc::new(Mutex::new(0)),
         }
      }
   }

   #[async_trait::async_trait]
   impl DisplayIdCounterRepository for MockDisplayIdCounterRepository {
      async fn next_display_number(
         &self,
         _tenant_id: &TenantId,
         _entity_type: DisplayIdEntityType,
      ) -> Result<DisplayNumber, InfraError> {
         let mut counter = self.counter.lock().unwrap();
         *counter += 1;
         Ok(DisplayNumber::new(*counter).unwrap())
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
      let definition = WorkflowDefinition::new(NewWorkflowDefinition {
         id:          WorkflowDefinitionId::new(),
         tenant_id:   tenant_id.clone(),
         name:        WorkflowName::new("汎用申請").unwrap(),
         description: Some("テスト用定義".to_string()),
         definition:  serde_json::json!({"steps": []}),
         created_by:  user_id.clone(),
         now:         chrono::Utc::now(),
      });
      let published_definition = definition.published(chrono::Utc::now()).unwrap();
      definition_repo.add_definition(published_definition.clone());

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo.clone()),
         Arc::new(step_repo),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
      );

      let input = CreateWorkflowInput {
         definition_id: published_definition.id().clone(),
         title:         "テスト申請".to_string(),
         form_data:     serde_json::json!({"note": "test"}),
      };

      // Act
      let result = sut
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

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
      );

      let input = CreateWorkflowInput {
         definition_id: WorkflowDefinitionId::new(),
         title:         "テスト申請".to_string(),
         form_data:     serde_json::json!({}),
      };

      // Act
      let result = sut.create_workflow(input, tenant_id, user_id).await;

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
      let now = chrono::Utc::now();
      let instance = WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id: tenant_id.clone(),
         definition_id: WorkflowDefinitionId::new(),
         definition_version: Version::initial(),
         display_number: DisplayNumber::new(100).unwrap(),
         title: "テスト申請".to_string(),
         form_data: serde_json::json!({}),
         initiated_by: user_id.clone(),
         now,
      })
      .submitted(now)
      .unwrap()
      .with_current_step("approval".to_string(), now);
      instance_repo.insert(&instance).await.unwrap();

      // Active なステップを作成
      let step = WorkflowStep::new(NewWorkflowStep {
         id: WorkflowStepId::new(),
         instance_id: instance.id().clone(),
         display_number: DisplayNumber::new(1).unwrap(),
         step_id: "approval".to_string(),
         step_name: "承認".to_string(),
         step_type: "approval".to_string(),
         assigned_to: Some(approver_id.clone()),
         now: chrono::Utc::now(),
      })
      .activated(chrono::Utc::now());
      step_repo.insert(&step).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo.clone()),
         Arc::new(step_repo.clone()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
      );

      let input = ApproveRejectInput {
         version: step.version(),
         comment: Some("承認しました".to_string()),
      };

      // Act
      let result = sut
         .approve_step(
            input,
            step.id().clone(),
            tenant_id.clone(),
            approver_id.clone(),
         )
         .await;

      // Assert
      assert!(result.is_ok());
      let workflow_with_steps = result.unwrap();

      // 返却されたインスタンスが Approved になっていることを確認
      assert_eq!(
         workflow_with_steps.instance.status(),
         WorkflowInstanceStatus::Approved
      );

      // 返却されたステップが Completed (Approved) になっていることを確認
      assert_eq!(workflow_with_steps.steps.len(), 1);
      assert_eq!(
         workflow_with_steps.steps[0].status(),
         ringiflow_domain::workflow::WorkflowStepStatus::Completed
      );
      assert_eq!(
         workflow_with_steps.steps[0].decision(),
         Some(ringiflow_domain::workflow::StepDecision::Approved)
      );
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

      let now = chrono::Utc::now();
      let instance = WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id: tenant_id.clone(),
         definition_id: WorkflowDefinitionId::new(),
         definition_version: Version::initial(),
         display_number: DisplayNumber::new(100).unwrap(),
         title: "テスト申請".to_string(),
         form_data: serde_json::json!({}),
         initiated_by: user_id.clone(),
         now,
      })
      .submitted(now)
      .unwrap()
      .with_current_step("approval".to_string(), now);
      instance_repo.insert(&instance).await.unwrap();

      let step = WorkflowStep::new(NewWorkflowStep {
         id: WorkflowStepId::new(),
         instance_id: instance.id().clone(),
         display_number: DisplayNumber::new(1).unwrap(),
         step_id: "approval".to_string(),
         step_name: "承認".to_string(),
         step_type: "approval".to_string(),
         assigned_to: Some(approver_id.clone()), // approver_id に割り当て
         now: chrono::Utc::now(),
      })
      .activated(chrono::Utc::now());
      step_repo.insert(&step).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
      );

      let input = ApproveRejectInput {
         version: step.version(),
         comment: None,
      };

      // Act: 別のユーザーで承認を試みる
      let result = sut
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

      let now = chrono::Utc::now();
      let instance = WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id: tenant_id.clone(),
         definition_id: WorkflowDefinitionId::new(),
         definition_version: Version::initial(),
         display_number: DisplayNumber::new(100).unwrap(),
         title: "テスト申請".to_string(),
         form_data: serde_json::json!({}),
         initiated_by: user_id.clone(),
         now,
      })
      .submitted(now)
      .unwrap()
      .with_current_step("approval".to_string(), now);
      instance_repo.insert(&instance).await.unwrap();

      // Pending 状態のステップ（Active ではない）
      let step = WorkflowStep::new(NewWorkflowStep {
         id: WorkflowStepId::new(),
         instance_id: instance.id().clone(),
         display_number: DisplayNumber::new(1).unwrap(),
         step_id: "approval".to_string(),
         step_name: "承認".to_string(),
         step_type: "approval".to_string(),
         assigned_to: Some(approver_id.clone()),
         now: chrono::Utc::now(),
      });
      // activated() を呼ばないので Pending のまま
      step_repo.insert(&step).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
      );

      let input = ApproveRejectInput {
         version: step.version(),
         comment: None,
      };

      // Act
      let result = sut
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

      let now = chrono::Utc::now();
      let instance = WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id: tenant_id.clone(),
         definition_id: WorkflowDefinitionId::new(),
         definition_version: Version::initial(),
         display_number: DisplayNumber::new(100).unwrap(),
         title: "テスト申請".to_string(),
         form_data: serde_json::json!({}),
         initiated_by: user_id.clone(),
         now,
      })
      .submitted(now)
      .unwrap()
      .with_current_step("approval".to_string(), now);
      instance_repo.insert(&instance).await.unwrap();

      let step = WorkflowStep::new(NewWorkflowStep {
         id: WorkflowStepId::new(),
         instance_id: instance.id().clone(),
         display_number: DisplayNumber::new(1).unwrap(),
         step_id: "approval".to_string(),
         step_name: "承認".to_string(),
         step_type: "approval".to_string(),
         assigned_to: Some(approver_id.clone()),
         now: chrono::Utc::now(),
      })
      .activated(chrono::Utc::now());
      step_repo.insert(&step).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
      );

      // 不一致バージョンを指定（ステップの version は 1 だが、2 を指定）
      let wrong_version = Version::initial().next();
      let input = ApproveRejectInput {
         version: wrong_version,
         comment: None,
      };

      // Act
      let result = sut
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

      let now = chrono::Utc::now();
      let instance = WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id: tenant_id.clone(),
         definition_id: WorkflowDefinitionId::new(),
         definition_version: Version::initial(),
         display_number: DisplayNumber::new(100).unwrap(),
         title: "テスト申請".to_string(),
         form_data: serde_json::json!({}),
         initiated_by: user_id.clone(),
         now,
      })
      .submitted(now)
      .unwrap()
      .with_current_step("approval".to_string(), now);
      instance_repo.insert(&instance).await.unwrap();

      let step = WorkflowStep::new(NewWorkflowStep {
         id: WorkflowStepId::new(),
         instance_id: instance.id().clone(),
         display_number: DisplayNumber::new(1).unwrap(),
         step_id: "approval".to_string(),
         step_name: "承認".to_string(),
         step_type: "approval".to_string(),
         assigned_to: Some(approver_id.clone()),
         now: chrono::Utc::now(),
      })
      .activated(chrono::Utc::now());
      step_repo.insert(&step).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo.clone()),
         Arc::new(step_repo.clone()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
      );

      let input = ApproveRejectInput {
         version: step.version(),
         comment: Some("却下理由".to_string()),
      };

      // Act
      let result = sut
         .reject_step(
            input,
            step.id().clone(),
            tenant_id.clone(),
            approver_id.clone(),
         )
         .await;

      // Assert
      assert!(result.is_ok());
      let workflow_with_steps = result.unwrap();

      // 返却されたインスタンスが Rejected になっていることを確認
      assert_eq!(
         workflow_with_steps.instance.status(),
         WorkflowInstanceStatus::Rejected
      );

      // 返却されたステップが Completed (Rejected) になっていることを確認
      assert_eq!(workflow_with_steps.steps.len(), 1);
      assert_eq!(
         workflow_with_steps.steps[0].status(),
         ringiflow_domain::workflow::WorkflowStepStatus::Completed
      );
      assert_eq!(
         workflow_with_steps.steps[0].decision(),
         Some(ringiflow_domain::workflow::StepDecision::Rejected)
      );
   }

   #[tokio::test]
   async fn test_reject_step_未割り当てユーザーは403() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver_id = UserId::new();
      let other_user_id = UserId::new(); // 別のユーザー

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      let now = chrono::Utc::now();
      let instance = WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id: tenant_id.clone(),
         definition_id: WorkflowDefinitionId::new(),
         definition_version: Version::initial(),
         display_number: DisplayNumber::new(100).unwrap(),
         title: "テスト申請".to_string(),
         form_data: serde_json::json!({}),
         initiated_by: user_id.clone(),
         now,
      })
      .submitted(now)
      .unwrap()
      .with_current_step("approval".to_string(), now);
      instance_repo.insert(&instance).await.unwrap();

      let step = WorkflowStep::new(NewWorkflowStep {
         id: WorkflowStepId::new(),
         instance_id: instance.id().clone(),
         display_number: DisplayNumber::new(1).unwrap(),
         step_id: "approval".to_string(),
         step_name: "承認".to_string(),
         step_type: "approval".to_string(),
         assigned_to: Some(approver_id.clone()), // approver_id に割り当て
         now: chrono::Utc::now(),
      })
      .activated(chrono::Utc::now());
      step_repo.insert(&step).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
      );

      let input = ApproveRejectInput {
         version: step.version(),
         comment: None,
      };

      // Act: 別のユーザーで却下を試みる
      let result = sut
         .reject_step(input, step.id().clone(), tenant_id, other_user_id)
         .await;

      // Assert
      assert!(matches!(result, Err(CoreError::Forbidden(_))));
   }

   #[tokio::test]
   async fn test_reject_step_active以外は400() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver_id = UserId::new();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      let now = chrono::Utc::now();
      let instance = WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id: tenant_id.clone(),
         definition_id: WorkflowDefinitionId::new(),
         definition_version: Version::initial(),
         display_number: DisplayNumber::new(100).unwrap(),
         title: "テスト申請".to_string(),
         form_data: serde_json::json!({}),
         initiated_by: user_id.clone(),
         now,
      })
      .submitted(now)
      .unwrap()
      .with_current_step("approval".to_string(), now);
      instance_repo.insert(&instance).await.unwrap();

      // Pending 状態のステップ（Active ではない）
      let step = WorkflowStep::new(NewWorkflowStep {
         id: WorkflowStepId::new(),
         instance_id: instance.id().clone(),
         display_number: DisplayNumber::new(1).unwrap(),
         step_id: "approval".to_string(),
         step_name: "承認".to_string(),
         step_type: "approval".to_string(),
         assigned_to: Some(approver_id.clone()),
         now: chrono::Utc::now(),
      });
      // activated() を呼ばないので Pending のまま
      step_repo.insert(&step).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
      );

      let input = ApproveRejectInput {
         version: step.version(),
         comment: None,
      };

      // Act
      let result = sut
         .reject_step(input, step.id().clone(), tenant_id, approver_id)
         .await;

      // Assert
      assert!(matches!(result, Err(CoreError::BadRequest(_))));
   }

   #[tokio::test]
   async fn test_reject_step_バージョン不一致で409() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver_id = UserId::new();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      let now = chrono::Utc::now();
      let instance = WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id: tenant_id.clone(),
         definition_id: WorkflowDefinitionId::new(),
         definition_version: Version::initial(),
         display_number: DisplayNumber::new(100).unwrap(),
         title: "テスト申請".to_string(),
         form_data: serde_json::json!({}),
         initiated_by: user_id.clone(),
         now,
      })
      .submitted(now)
      .unwrap()
      .with_current_step("approval".to_string(), now);
      instance_repo.insert(&instance).await.unwrap();

      let step = WorkflowStep::new(NewWorkflowStep {
         id: WorkflowStepId::new(),
         instance_id: instance.id().clone(),
         display_number: DisplayNumber::new(1).unwrap(),
         step_id: "approval".to_string(),
         step_name: "承認".to_string(),
         step_type: "approval".to_string(),
         assigned_to: Some(approver_id.clone()),
         now: chrono::Utc::now(),
      })
      .activated(chrono::Utc::now());
      step_repo.insert(&step).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
      );

      // 不一致バージョンを指定（ステップの version は 1 だが、2 を指定）
      let wrong_version = Version::initial().next();
      let input = ApproveRejectInput {
         version: wrong_version,
         comment: None,
      };

      // Act
      let result = sut
         .reject_step(input, step.id().clone(), tenant_id, approver_id)
         .await;

      // Assert
      assert!(matches!(result, Err(CoreError::Conflict(_))));
   }

   // ===== submit_workflow テスト =====

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
      let definition = WorkflowDefinition::new(NewWorkflowDefinition {
         id:          WorkflowDefinitionId::new(),
         tenant_id:   tenant_id.clone(),
         name:        WorkflowName::new("汎用申請").unwrap(),
         description: Some("テスト用定義".to_string()),
         definition:  serde_json::json!({"steps": []}),
         created_by:  user_id.clone(),
         now:         chrono::Utc::now(),
      });
      let published_definition = definition.published(chrono::Utc::now()).unwrap();
      definition_repo.add_definition(published_definition.clone());

      // 下書きのインスタンスを作成
      let instance = WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id: tenant_id.clone(),
         definition_id: published_definition.id().clone(),
         definition_version: Version::initial(),
         display_number: DisplayNumber::new(100).unwrap(),
         title: "テスト申請".to_string(),
         form_data: serde_json::json!({}),
         initiated_by: user_id.clone(),
         now: chrono::Utc::now(),
      });
      instance_repo.insert(&instance).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo.clone()),
         Arc::new(step_repo.clone()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
      );

      let input = SubmitWorkflowInput {
         assigned_to: approver_id.clone(),
      };

      // Act
      let result = sut
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

   #[tokio::test]
   async fn test_submit_workflow_draft以外は400() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver_id = UserId::new();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      // 公開済みの定義を追加
      let definition = WorkflowDefinition::new(NewWorkflowDefinition {
         id:          WorkflowDefinitionId::new(),
         tenant_id:   tenant_id.clone(),
         name:        WorkflowName::new("汎用申請").unwrap(),
         description: Some("テスト用定義".to_string()),
         definition:  serde_json::json!({"steps": []}),
         created_by:  user_id.clone(),
         now:         chrono::Utc::now(),
      });
      let published_definition = definition.published(chrono::Utc::now()).unwrap();
      definition_repo.add_definition(published_definition.clone());

      // InProgress 状態のインスタンスを作成（Draft ではない）
      let now = chrono::Utc::now();
      let instance = WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id: tenant_id.clone(),
         definition_id: published_definition.id().clone(),
         definition_version: Version::initial(),
         display_number: DisplayNumber::new(100).unwrap(),
         title: "テスト申請".to_string(),
         form_data: serde_json::json!({}),
         initiated_by: user_id.clone(),
         now,
      })
      .submitted(now)
      .unwrap()
      .with_current_step("approval".to_string(), now);
      instance_repo.insert(&instance).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
      );

      let input = SubmitWorkflowInput {
         assigned_to: approver_id.clone(),
      };

      // Act: InProgress 状態のインスタンスに対して申請を試みる
      let result = sut
         .submit_workflow(input, instance.id().clone(), tenant_id)
         .await;

      // Assert
      assert!(matches!(result, Err(CoreError::BadRequest(_))));
   }
}
