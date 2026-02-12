//! ワークフローユースケースの状態変更操作

use ringiflow_domain::{
   tenant::TenantId,
   user::UserId,
   value_objects::{DisplayIdEntityType, DisplayNumber},
   workflow::{
      CommentBody,
      NewWorkflowComment,
      NewWorkflowInstance,
      NewWorkflowStep,
      WorkflowComment,
      WorkflowCommentId,
      WorkflowInstance,
      WorkflowInstanceId,
      WorkflowInstanceStatus,
      WorkflowStep,
      WorkflowStepId,
      WorkflowStepStatus,
   },
};
use ringiflow_infra::InfraError;

use super::{
   ApproveRejectInput,
   CreateWorkflowInput,
   PostCommentInput,
   ResubmitWorkflowInput,
   SubmitWorkflowInput,
   WorkflowUseCaseImpl,
   WorkflowWithSteps,
};
use crate::error::CoreError;

impl WorkflowUseCaseImpl {
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
      let now = self.clock.now();
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
   /// ワークフロー定義に基づいて複数の承認ステップを作成する。
   ///
   /// ## 処理フロー
   ///
   /// 1. ワークフローインスタンスが存在するか確認
   /// 2. draft 状態であるか確認
   /// 3. ワークフロー定義を取得
   /// 4. 定義から承認ステップを抽出し、approvers との整合性を検証
   /// 5. 各承認ステップを作成（最初を Active、残りを Pending）
   /// 6. ワークフローインスタンスを pending → in_progress に遷移
   /// 7. インスタンスとステップをリポジトリに保存
   ///
   /// ## エラー
   ///
   /// - ワークフローインスタンスが見つからない場合
   /// - ワークフローインスタンスが draft でない場合
   /// - approvers と定義のステップが一致しない場合
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

      // 3. ワークフロー定義を取得
      let definition = self
         .definition_repo
         .find_by_id(instance.definition_id(), &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("定義の取得に失敗: {}", e)))?
         .ok_or_else(|| CoreError::NotFound("ワークフロー定義が見つかりません".to_string()))?;

      // 4. 定義から承認ステップを抽出
      let approval_step_defs = definition
         .extract_approval_steps()
         .map_err(|e| CoreError::BadRequest(e.to_string()))?;

      // approvers と定義のステップの整合性を検証
      if input.approvers.len() != approval_step_defs.len() {
         return Err(CoreError::BadRequest(format!(
            "承認者の数({})が定義のステップ数({})と一致しません",
            input.approvers.len(),
            approval_step_defs.len()
         )));
      }

      for (approver, step_def) in input.approvers.iter().zip(&approval_step_defs) {
         if approver.step_id != step_def.id {
            return Err(CoreError::BadRequest(format!(
               "承認者のステップ ID({})が定義のステップ ID({})と一致しません",
               approver.step_id, step_def.id
            )));
         }
      }

      // 5. 各承認ステップを作成
      let now = self.clock.now();
      let mut steps = Vec::with_capacity(approval_step_defs.len());

      for (i, (step_def, approver)) in approval_step_defs.iter().zip(&input.approvers).enumerate() {
         let display_number = self
            .counter_repo
            .next_display_number(&tenant_id, DisplayIdEntityType::WorkflowStep)
            .await
            .map_err(|e| CoreError::Internal(format!("採番に失敗: {}", e)))?;

         let step = WorkflowStep::new(NewWorkflowStep {
            id: WorkflowStepId::new(),
            instance_id: instance_id.clone(),
            display_number,
            step_id: step_def.id.clone(),
            step_name: step_def.name.clone(),
            step_type: "approval".to_string(),
            assigned_to: Some(approver.assigned_to.clone()),
            now,
         });

         // 最初のステップのみ Active にする
         let step = if i == 0 { step.activated(now) } else { step };
         steps.push(step);
      }

      // 6. ワークフローインスタンスを申請済みに遷移
      let expected_version = instance.version();
      let first_step_id = approval_step_defs[0].id.clone();
      let submitted_instance = instance
         .submitted(now)
         .map_err(|e| CoreError::BadRequest(e.to_string()))?;

      // current_step_id を最初の承認ステップに設定して in_progress に遷移
      let in_progress_instance = submitted_instance.with_current_step(first_step_id, now);

      // 7. インスタンスとステップを保存
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

      for step in &steps {
         self
            .step_repo
            .insert(step, &tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("ステップの保存に失敗: {}", e)))?;
      }

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
   /// 5. 次ステップの判定:
   ///    - 次がある → 次ステップを Active 化、current_step_id を更新
   ///    - 次がない → インスタンスを Approved に遷移
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
      let now = self.clock.now();
      let step_expected_version = step.version();
      let current_step_id = step.step_id().to_string();
      let approved_step = step
         .approve(input.comment, now)
         .map_err(|e| CoreError::BadRequest(e.to_string()))?;

      // 5. インスタンスを取得
      let instance = self
         .instance_repo
         .find_by_id(approved_step.instance_id(), &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("インスタンスの取得に失敗: {}", e)))?
         .ok_or_else(|| CoreError::NotFound("インスタンスが見つかりません".to_string()))?;

      let instance_expected_version = instance.version();

      // 6. 定義から承認ステップの順序を取得し、次ステップを判定
      let definition = self
         .definition_repo
         .find_by_id(instance.definition_id(), &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("定義の取得に失敗: {}", e)))?
         .ok_or_else(|| CoreError::Internal("定義が見つかりません".to_string()))?;

      let approval_step_defs =
         ringiflow_domain::workflow::extract_approval_steps(definition.definition())
            .map_err(|e| CoreError::Internal(format!("定義の解析に失敗: {}", e)))?;

      // 現在のステップの位置を特定し、次のステップがあるか判定
      let current_index = approval_step_defs
         .iter()
         .position(|s| s.id == current_step_id);

      let next_step_def = current_index.and_then(|i| approval_step_defs.get(i + 1));

      // 7. 次ステップの有無でインスタンスの遷移を分岐
      let (updated_instance, next_step_to_activate) = if let Some(next_def) = next_step_def {
         // 次ステップあり → current_step_id を更新、InProgress のまま
         let advanced = instance
            .advance_to_next_step(next_def.id.clone(), now)
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;
         (advanced, Some(next_def.id.clone()))
      } else {
         // 最終ステップ → インスタンスを Approved に遷移
         let completed = instance
            .complete_with_approval(now)
            .map_err(|e| CoreError::BadRequest(e.to_string()))?;
         (completed, None)
      };

      // 8. 楽観的ロック付きでステップを保存
      self
         .step_repo
         .update_with_version_check(&approved_step, step_expected_version, &tenant_id)
         .await
         .map_err(|e| match e {
            InfraError::Conflict { .. } => CoreError::Conflict(
               "ステップは既に更新されています。最新の情報を取得してください。".to_string(),
            ),
            other => CoreError::Internal(format!("ステップの保存に失敗: {}", other)),
         })?;

      // 9. 次ステップがあれば Active 化して保存
      if let Some(next_step_id) = next_step_to_activate {
         // インスタンスに紐づくステップから次ステップを見つけて Active 化
         let all_steps = self
            .step_repo
            .find_by_instance(updated_instance.id(), &tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?;

         if let Some(next_step) = all_steps.into_iter().find(|s| s.step_id() == next_step_id) {
            let next_expected_version = next_step.version();
            let activated_step = next_step.activated(now);
            self
               .step_repo
               .update_with_version_check(&activated_step, next_expected_version, &tenant_id)
               .await
               .map_err(|e| match e {
                  InfraError::Conflict { .. } => CoreError::Conflict(
                     "ステップは既に更新されています。最新の情報を取得してください。".to_string(),
                  ),
                  other => CoreError::Internal(format!("ステップの保存に失敗: {}", other)),
               })?;
         }
      }

      // 10. インスタンスを保存
      self
         .instance_repo
         .update_with_version_check(&updated_instance, instance_expected_version)
         .await
         .map_err(|e| match e {
            InfraError::Conflict { .. } => CoreError::Conflict(
               "インスタンスは既に更新されています。最新の情報を取得してください。".to_string(),
            ),
            other => CoreError::Internal(format!("インスタンスの保存に失敗: {}", other)),
         })?;

      // 11. 保存後のステップ一覧を取得して返却
      let steps = self
         .step_repo
         .find_by_instance(updated_instance.id(), &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?;

      Ok(WorkflowWithSteps {
         instance: updated_instance,
         steps,
      })
   }

   /// ワークフローステップを却下する
   ///
   /// ## 処理フロー
   ///
   /// 1. ステップを取得
   /// 2. 権限チェック（担当者のみ却下可能）
   /// 3. 楽観的ロック（バージョン一致チェック）
   /// 4. ステップを却下
   /// 5. 残りの Pending ステップを Skipped に遷移
   /// 6. インスタンスを Rejected に遷移
   /// 7. 保存
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
      let now = self.clock.now();
      let step_expected_version = step.version();
      let rejected_step = step
         .reject(input.comment, now)
         .map_err(|e| CoreError::BadRequest(e.to_string()))?;

      // 5. 却下されたステップを保存
      self
         .step_repo
         .update_with_version_check(&rejected_step, step_expected_version, &tenant_id)
         .await
         .map_err(|e| match e {
            InfraError::Conflict { .. } => CoreError::Conflict(
               "ステップは既に更新されています。最新の情報を取得してください。".to_string(),
            ),
            other => CoreError::Internal(format!("ステップの保存に失敗: {}", other)),
         })?;

      // 6. 残りの Pending ステップを Skipped に遷移
      let all_steps = self
         .step_repo
         .find_by_instance(rejected_step.instance_id(), &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?;

      for pending_step in all_steps
         .into_iter()
         .filter(|s| s.status() == WorkflowStepStatus::Pending)
      {
         let pending_expected_version = pending_step.version();
         let skipped_step = pending_step
            .skipped(now)
            .map_err(|e| CoreError::Internal(format!("ステップのスキップに失敗: {}", e)))?;
         self
            .step_repo
            .update_with_version_check(&skipped_step, pending_expected_version, &tenant_id)
            .await
            .map_err(|e| match e {
               InfraError::Conflict { .. } => CoreError::Conflict(
                  "ステップは既に更新されています。最新の情報を取得してください。".to_string(),
               ),
               other => CoreError::Internal(format!("ステップの保存に失敗: {}", other)),
            })?;
      }

      // 7. インスタンスを取得して Rejected に遷移
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

      // 8. 保存後のステップ一覧を取得して返却
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

   /// ワークフローステップを差し戻す
   ///
   /// ## 処理フロー
   ///
   /// 1. ステップを取得
   /// 2. 権限チェック（担当者のみ差し戻し可能）
   /// 3. 楽観的ロック（バージョン一致チェック）
   /// 4. ステップを差し戻し
   /// 5. 残りの Pending ステップを Skipped に遷移
   /// 6. インスタンスを ChangesRequested に遷移
   /// 7. 保存
   ///
   /// ## エラー
   ///
   /// - ステップが見つからない場合: 404
   /// - 権限がない場合: 403
   /// - Active 以外の場合: 400
   /// - バージョン不一致の場合: 409
   pub async fn request_changes_step(
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
            "このステップを差し戻す権限がありません".to_string(),
         ));
      }

      // 3. 楽観的ロック（バージョン一致チェック — 早期フェイル）
      if step.version() != input.version {
         return Err(CoreError::Conflict(
            "ステップは既に更新されています。最新の情報を取得してください。".to_string(),
         ));
      }

      // 4. ステップを差し戻し
      let now = self.clock.now();
      let step_expected_version = step.version();
      let request_changes_step = step
         .request_changes(input.comment, now)
         .map_err(|e| CoreError::BadRequest(e.to_string()))?;

      // 5. 差し戻しステップを保存
      self
         .step_repo
         .update_with_version_check(&request_changes_step, step_expected_version, &tenant_id)
         .await
         .map_err(|e| match e {
            InfraError::Conflict { .. } => CoreError::Conflict(
               "ステップは既に更新されています。最新の情報を取得してください。".to_string(),
            ),
            other => CoreError::Internal(format!("ステップの保存に失敗: {}", other)),
         })?;

      // 6. 残りの Pending ステップを Skipped に遷移
      let all_steps = self
         .step_repo
         .find_by_instance(request_changes_step.instance_id(), &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?;

      for pending_step in all_steps
         .into_iter()
         .filter(|s| s.status() == WorkflowStepStatus::Pending)
      {
         let pending_expected_version = pending_step.version();
         let skipped_step = pending_step
            .skipped(now)
            .map_err(|e| CoreError::Internal(format!("ステップのスキップに失敗: {}", e)))?;
         self
            .step_repo
            .update_with_version_check(&skipped_step, pending_expected_version, &tenant_id)
            .await
            .map_err(|e| match e {
               InfraError::Conflict { .. } => CoreError::Conflict(
                  "ステップは既に更新されています。最新の情報を取得してください。".to_string(),
               ),
               other => CoreError::Internal(format!("ステップの保存に失敗: {}", other)),
            })?;
      }

      // 7. インスタンスを取得して ChangesRequested に遷移
      let instance = self
         .instance_repo
         .find_by_id(request_changes_step.instance_id(), &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("インスタンスの取得に失敗: {}", e)))?
         .ok_or_else(|| CoreError::NotFound("インスタンスが見つかりません".to_string()))?;

      let instance_expected_version = instance.version();
      let changes_requested_instance = instance
         .complete_with_request_changes(now)
         .map_err(|e| CoreError::BadRequest(e.to_string()))?;

      self
         .instance_repo
         .update_with_version_check(&changes_requested_instance, instance_expected_version)
         .await
         .map_err(|e| match e {
            InfraError::Conflict { .. } => CoreError::Conflict(
               "インスタンスは既に更新されています。最新の情報を取得してください。".to_string(),
            ),
            other => CoreError::Internal(format!("インスタンスの保存に失敗: {}", other)),
         })?;

      // 8. 保存後のステップ一覧を取得して返却
      let steps = self
         .step_repo
         .find_by_instance(changes_requested_instance.id(), &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?;

      Ok(WorkflowWithSteps {
         instance: changes_requested_instance,
         steps,
      })
   }

   /// ワークフローを再申請する
   ///
   /// ## 処理フロー
   ///
   /// 1. ワークフローインスタンスを取得
   /// 2. ChangesRequested 状態であるか確認
   /// 3. 権限チェック（申請者本人のみ再申請可能）
   /// 4. 楽観的ロック（バージョン一致チェック）
   /// 5. ワークフロー定義を取得し、承認ステップを抽出
   /// 6. approvers との整合性を検証
   /// 7. 新しい承認ステップを作成
   /// 8. インスタンスを InProgress に遷移（form_data 更新）
   /// 9. 保存
   ///
   /// ## エラー
   ///
   /// - インスタンスが見つからない場合: 404
   /// - ChangesRequested 以外の場合: 400
   /// - 申請者以外の場合: 403
   /// - バージョン不一致の場合: 409
   /// - approvers と定義が不一致の場合: 400
   pub async fn resubmit_workflow(
      &self,
      input: ResubmitWorkflowInput,
      instance_id: WorkflowInstanceId,
      tenant_id: TenantId,
      user_id: UserId,
   ) -> Result<WorkflowWithSteps, CoreError> {
      // 1. ワークフローインスタンスを取得
      let instance = self
         .instance_repo
         .find_by_id(&instance_id, &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("インスタンスの取得に失敗: {}", e)))?
         .ok_or_else(|| {
            CoreError::NotFound("ワークフローインスタンスが見つかりません".to_string())
         })?;

      // 2. ChangesRequested 状態であるか確認
      if instance.status() != WorkflowInstanceStatus::ChangesRequested {
         return Err(CoreError::BadRequest(
            "要修正状態のワークフローのみ再申請できます".to_string(),
         ));
      }

      // 3. 権限チェック（申請者本人のみ再申請可能）
      if instance.initiated_by() != &user_id {
         return Err(CoreError::Forbidden(
            "このワークフローを再申請する権限がありません".to_string(),
         ));
      }

      // 4. 楽観的ロック（バージョン一致チェック — 早期フェイル）
      if instance.version() != input.version {
         return Err(CoreError::Conflict(
            "インスタンスは既に更新されています。最新の情報を取得してください。".to_string(),
         ));
      }

      // 5. ワークフロー定義を取得
      let definition = self
         .definition_repo
         .find_by_id(instance.definition_id(), &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("定義の取得に失敗: {}", e)))?
         .ok_or_else(|| CoreError::NotFound("ワークフロー定義が見つかりません".to_string()))?;

      // 定義から承認ステップを抽出
      let approval_step_defs = definition
         .extract_approval_steps()
         .map_err(|e| CoreError::BadRequest(e.to_string()))?;

      // 6. approvers と定義のステップの整合性を検証
      if input.approvers.len() != approval_step_defs.len() {
         return Err(CoreError::BadRequest(format!(
            "承認者の数({})が定義のステップ数({})と一致しません",
            input.approvers.len(),
            approval_step_defs.len()
         )));
      }

      for (approver, step_def) in input.approvers.iter().zip(&approval_step_defs) {
         if approver.step_id != step_def.id {
            return Err(CoreError::BadRequest(format!(
               "承認者のステップ ID({})が定義のステップ ID({})と一致しません",
               approver.step_id, step_def.id
            )));
         }
      }

      // 7. 新しい承認ステップを作成
      let now = self.clock.now();
      let mut steps = Vec::with_capacity(approval_step_defs.len());

      for (i, (step_def, approver)) in approval_step_defs.iter().zip(&input.approvers).enumerate() {
         let display_number = self
            .counter_repo
            .next_display_number(&tenant_id, DisplayIdEntityType::WorkflowStep)
            .await
            .map_err(|e| CoreError::Internal(format!("採番に失敗: {}", e)))?;

         let step = WorkflowStep::new(NewWorkflowStep {
            id: WorkflowStepId::new(),
            instance_id: instance_id.clone(),
            display_number,
            step_id: step_def.id.clone(),
            step_name: step_def.name.clone(),
            step_type: "approval".to_string(),
            assigned_to: Some(approver.assigned_to.clone()),
            now,
         });

         // 最初のステップのみ Active にする
         let step = if i == 0 { step.activated(now) } else { step };
         steps.push(step);
      }

      // 8. インスタンスを InProgress に遷移
      let instance_expected_version = instance.version();
      let first_step_id = approval_step_defs[0].id.clone();
      let resubmitted_instance = instance
         .resubmitted(input.form_data, first_step_id, now)
         .map_err(|e| CoreError::BadRequest(e.to_string()))?;

      // 9. インスタンスとステップを保存
      self
         .instance_repo
         .update_with_version_check(&resubmitted_instance, instance_expected_version)
         .await
         .map_err(|e| match e {
            InfraError::Conflict { .. } => CoreError::Conflict(
               "インスタンスは既に更新されています。最新の情報を取得してください。".to_string(),
            ),
            other => CoreError::Internal(format!("インスタンスの保存に失敗: {}", other)),
         })?;

      for step in &steps {
         self
            .step_repo
            .insert(step, &tenant_id)
            .await
            .map_err(|e| CoreError::Internal(format!("ステップの保存に失敗: {}", e)))?;
      }

      Ok(WorkflowWithSteps {
         instance: resubmitted_instance,
         steps,
      })
   }

   // ===== display_number 対応メソッド（状態変更） =====

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

   /// display_number でワークフローステップを差し戻す
   pub async fn request_changes_step_by_display_number(
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

      // 既存の request_changes_step を呼び出し
      self
         .request_changes_step(input, step.id().clone(), tenant_id, user_id)
         .await
   }

   /// display_number でワークフローを再申請する
   pub async fn resubmit_workflow_by_display_number(
      &self,
      input: ResubmitWorkflowInput,
      display_number: DisplayNumber,
      tenant_id: TenantId,
      user_id: UserId,
   ) -> Result<WorkflowWithSteps, CoreError> {
      // display_number → WorkflowInstanceId を解決
      let instance = self
         .instance_repo
         .find_by_display_number(display_number, &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("インスタンスの取得に失敗: {}", e)))?
         .ok_or_else(|| {
            CoreError::NotFound("ワークフローインスタンスが見つかりません".to_string())
         })?;

      // 既存の resubmit_workflow を呼び出し
      self
         .resubmit_workflow(input, instance.id().clone(), tenant_id, user_id)
         .await
   }

   // ===== コメント系メソッド =====

   /// ワークフローにコメントを投稿する
   ///
   /// ## 処理フロー
   ///
   /// 1. display_number でワークフローインスタンスを取得
   /// 2. 権限チェック（関与者のみ投稿可能）
   /// 3. コメント本文のバリデーション
   /// 4. コメントを作成して保存
   ///
   /// ## 権限: 関与者
   ///
   /// - 申請者（`instance.initiated_by == user_id`）
   /// - いずれかのステップの承認者（`steps.any(|s| s.assigned_to() == Some(&user_id))`）
   ///
   /// ## エラー
   ///
   /// - ワークフローが見つからない場合: 404
   /// - 関与していないユーザーの場合: 403
   /// - コメント本文が無効な場合: 400
   pub async fn post_comment(
      &self,
      input: PostCommentInput,
      display_number: DisplayNumber,
      tenant_id: TenantId,
      user_id: UserId,
   ) -> Result<WorkflowComment, CoreError> {
      // 1. ワークフローインスタンスを取得
      let instance = self
         .instance_repo
         .find_by_display_number(display_number, &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("インスタンスの取得に失敗: {}", e)))?
         .ok_or_else(|| {
            CoreError::NotFound("ワークフローインスタンスが見つかりません".to_string())
         })?;

      // 2. 権限チェック
      if !self.is_participant(&instance, &user_id, &tenant_id).await? {
         return Err(CoreError::Forbidden(
            "このワークフローにコメントする権限がありません".to_string(),
         ));
      }

      // 3. コメント本文のバリデーション
      let body = CommentBody::new(input.body).map_err(|e| CoreError::BadRequest(e.to_string()))?;

      // 4. コメントを作成して保存
      let now = self.clock.now();
      let comment = WorkflowComment::new(NewWorkflowComment {
         id: WorkflowCommentId::new(),
         tenant_id: tenant_id.clone(),
         instance_id: instance.id().clone(),
         posted_by: user_id,
         body,
         now,
      });

      self
         .comment_repo
         .insert(&comment, &tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("コメントの保存に失敗: {}", e)))?;

      Ok(comment)
   }

   /// ユーザーがワークフローの関与者かチェックする
   ///
   /// 関与者 = 申請者 OR いずれかのステップの承認者
   async fn is_participant(
      &self,
      instance: &WorkflowInstance,
      user_id: &UserId,
      tenant_id: &TenantId,
   ) -> Result<bool, CoreError> {
      // 申請者チェック
      if instance.initiated_by() == user_id {
         return Ok(true);
      }

      // 承認者チェック
      let steps = self
         .step_repo
         .find_by_instance(instance.id(), tenant_id)
         .await
         .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?;

      Ok(steps.iter().any(|s| s.assigned_to() == Some(user_id)))
   }
}

#[cfg(test)]
mod tests {
   use std::sync::Arc;

   use ringiflow_domain::{
      clock::FixedClock,
      tenant::TenantId,
      user::UserId,
      value_objects::{DisplayNumber, Version, WorkflowName},
      workflow::{
         NewWorkflowDefinition,
         NewWorkflowInstance,
         NewWorkflowStep,
         WorkflowDefinition,
         WorkflowDefinitionId,
         WorkflowInstance,
         WorkflowInstanceId,
         WorkflowStep,
         WorkflowStepId,
      },
   };
   use ringiflow_infra::{
      mock::{
         MockDisplayIdCounterRepository,
         MockUserRepository,
         MockWorkflowCommentRepository,
         MockWorkflowDefinitionRepository,
         MockWorkflowInstanceRepository,
         MockWorkflowStepRepository,
      },
      repository::{WorkflowInstanceRepository, WorkflowStepRepository},
   };

   use super::{
      super::{ResubmitWorkflowInput, StepApprover},
      *,
   };

   /// テスト用の1段階承認定義 JSON
   fn single_approval_definition_json() -> serde_json::Value {
      serde_json::json!({
         "steps": [
            {"id": "start", "type": "start", "name": "開始"},
            {"id": "approval", "type": "approval", "name": "承認"},
            {"id": "end_approved", "type": "end", "name": "承認完了", "status": "approved"},
            {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
         ]
      })
   }

   /// テスト用の2段階承認定義 JSON
   fn two_step_approval_definition_json() -> serde_json::Value {
      serde_json::json!({
         "steps": [
            {"id": "start", "type": "start", "name": "開始"},
            {"id": "manager_approval", "type": "approval", "name": "上長承認"},
            {"id": "finance_approval", "type": "approval", "name": "経理承認"},
            {"id": "end_approved", "type": "end", "name": "承認完了", "status": "approved"},
            {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
         ]
      })
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
      let now = chrono::Utc::now();
      let definition = WorkflowDefinition::new(NewWorkflowDefinition {
         id: WorkflowDefinitionId::new(),
         tenant_id: tenant_id.clone(),
         name: WorkflowName::new("汎用申請").unwrap(),
         description: Some("テスト用定義".to_string()),
         definition: serde_json::json!({"steps": []}),
         created_by: user_id.clone(),
         now,
      });
      let published_definition = definition.published(now).unwrap();
      definition_repo.add_definition(published_definition.clone());

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo.clone()),
         Arc::new(step_repo),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
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
      let result = result.unwrap();

      // result の ID を使って expected を構築（ID は内部で UUID v7 生成されるため）
      let expected = WorkflowInstance::new(NewWorkflowInstance {
         id: result.id().clone(),
         tenant_id: tenant_id.clone(),
         definition_id: published_definition.id().clone(),
         definition_version: published_definition.version(),
         display_number: DisplayNumber::new(1).unwrap(),
         title: "テスト申請".to_string(),
         form_data: serde_json::json!({"note": "test"}),
         initiated_by: user_id.clone(),
         now,
      });
      assert_eq!(result, expected);

      // リポジトリに保存されていることを確認
      let saved = instance_repo
         .find_by_id(result.id(), &tenant_id)
         .await
         .unwrap();
      assert_eq!(saved, Some(expected));
   }

   #[tokio::test]
   async fn test_create_workflow_定義が見つからない() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      let now = chrono::Utc::now();
      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
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

      // 1段階承認の定義を追加
      let now = chrono::Utc::now();
      let definition = WorkflowDefinition::new(NewWorkflowDefinition {
         id: WorkflowDefinitionId::new(),
         tenant_id: tenant_id.clone(),
         name: WorkflowName::new("汎用申請").unwrap(),
         description: Some("テスト用定義".to_string()),
         definition: single_approval_definition_json(),
         created_by: user_id.clone(),
         now,
      })
      .published(now)
      .unwrap();
      definition_repo.add_definition(definition.clone());

      // InProgress のインスタンスを作成
      let instance = WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id: tenant_id.clone(),
         definition_id: definition.id().clone(),
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
         now,
      })
      .activated(now);
      step_repo.insert(&step, &tenant_id).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo.clone()),
         Arc::new(step_repo.clone()),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
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
      let result = result.unwrap();
      let expected = WorkflowWithSteps {
         instance: instance.complete_with_approval(now).unwrap(),
         steps:    vec![step.approve(Some("承認しました".to_string()), now).unwrap()],
      };
      assert_eq!(result, expected);
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
      step_repo.insert(&step, &tenant_id).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
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
      step_repo.insert(&step, &tenant_id).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
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
      step_repo.insert(&step, &tenant_id).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
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

   // ===== 多段階承認テスト =====

   /// 2段階承認用テストヘルパー: 定義・インスタンス・2ステップを作成
   ///
   /// 戻り値: (definition, instance, step1(Active), step2(Pending))
   fn setup_two_step_approval(
      tenant_id: &TenantId,
      user_id: &UserId,
      approver1_id: &UserId,
      approver2_id: &UserId,
      now: chrono::DateTime<chrono::Utc>,
   ) -> (
      WorkflowDefinition,
      WorkflowInstance,
      WorkflowStep,
      WorkflowStep,
   ) {
      let definition = WorkflowDefinition::new(NewWorkflowDefinition {
         id: WorkflowDefinitionId::new(),
         tenant_id: tenant_id.clone(),
         name: WorkflowName::new("2段階承認").unwrap(),
         description: Some("テスト用定義".to_string()),
         definition: two_step_approval_definition_json(),
         created_by: user_id.clone(),
         now,
      })
      .published(now)
      .unwrap();

      let instance = WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id: tenant_id.clone(),
         definition_id: definition.id().clone(),
         definition_version: Version::initial(),
         display_number: DisplayNumber::new(100).unwrap(),
         title: "テスト申請".to_string(),
         form_data: serde_json::json!({}),
         initiated_by: user_id.clone(),
         now,
      })
      .submitted(now)
      .unwrap()
      .with_current_step("manager_approval".to_string(), now);

      let step1 = WorkflowStep::new(NewWorkflowStep {
         id: WorkflowStepId::new(),
         instance_id: instance.id().clone(),
         display_number: DisplayNumber::new(1).unwrap(),
         step_id: "manager_approval".to_string(),
         step_name: "上長承認".to_string(),
         step_type: "approval".to_string(),
         assigned_to: Some(approver1_id.clone()),
         now,
      })
      .activated(now);

      let step2 = WorkflowStep::new(NewWorkflowStep {
         id: WorkflowStepId::new(),
         instance_id: instance.id().clone(),
         display_number: DisplayNumber::new(2).unwrap(),
         step_id: "finance_approval".to_string(),
         step_name: "経理承認".to_string(),
         step_type: "approval".to_string(),
         assigned_to: Some(approver2_id.clone()),
         now,
      });

      (definition, instance, step1, step2)
   }

   #[tokio::test]
   async fn test_approve_step_中間ステップ_次のステップがactiveになる() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver1_id = UserId::new();
      let approver2_id = UserId::new();
      let now = chrono::Utc::now();

      let (definition, instance, step1, step2) =
         setup_two_step_approval(&tenant_id, &user_id, &approver1_id, &approver2_id, now);

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      definition_repo.add_definition(definition);
      instance_repo.insert(&instance).await.unwrap();
      step_repo.insert(&step1, &tenant_id).await.unwrap();
      step_repo.insert(&step2, &tenant_id).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo.clone()),
         Arc::new(step_repo.clone()),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      let input = ApproveRejectInput {
         version: step1.version(),
         comment: Some("上長承認OK".to_string()),
      };

      // Act
      let result = sut
         .approve_step(
            input,
            step1.id().clone(),
            tenant_id.clone(),
            approver1_id.clone(),
         )
         .await;

      // Assert
      let result = result.unwrap();

      // インスタンスのステータスは InProgress のまま
      assert_eq!(
         result.instance.status(),
         ringiflow_domain::workflow::WorkflowInstanceStatus::InProgress
      );

      // current_step_id が次のステップ（finance_approval）に更新されている
      assert_eq!(result.instance.current_step_id(), Some("finance_approval"));

      // ステップ一覧の確認
      assert_eq!(result.steps.len(), 2);

      // ステップ1は承認済み
      let result_step1 = result
         .steps
         .iter()
         .find(|s| s.step_id() == "manager_approval")
         .unwrap();
      assert_eq!(
         result_step1.status(),
         ringiflow_domain::workflow::WorkflowStepStatus::Completed
      );

      // ステップ2は Active になっている
      let result_step2 = result
         .steps
         .iter()
         .find(|s| s.step_id() == "finance_approval")
         .unwrap();
      assert_eq!(
         result_step2.status(),
         ringiflow_domain::workflow::WorkflowStepStatus::Active
      );
   }

   #[tokio::test]
   async fn test_approve_step_最終ステップ_インスタンスがapprovedになる() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver1_id = UserId::new();
      let approver2_id = UserId::new();
      let now = chrono::Utc::now();

      let (definition, instance, step1, step2) =
         setup_two_step_approval(&tenant_id, &user_id, &approver1_id, &approver2_id, now);

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      definition_repo.add_definition(definition);

      // ステップ1は既に承認済み、current_step_id は finance_approval に移行済み
      let instance_at_step2 = instance
         .advance_to_next_step("finance_approval".to_string(), now)
         .unwrap();
      instance_repo.insert(&instance_at_step2).await.unwrap();

      let completed_step1 = step1.approve(Some("上長承認OK".to_string()), now).unwrap();
      let active_step2 = step2.activated(now);
      step_repo
         .insert(&completed_step1, &tenant_id)
         .await
         .unwrap();
      step_repo.insert(&active_step2, &tenant_id).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo.clone()),
         Arc::new(step_repo.clone()),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      let input = ApproveRejectInput {
         version: active_step2.version(),
         comment: Some("経理承認OK".to_string()),
      };

      // Act
      let result = sut
         .approve_step(
            input,
            active_step2.id().clone(),
            tenant_id.clone(),
            approver2_id.clone(),
         )
         .await;

      // Assert
      let result = result.unwrap();

      // インスタンスが Approved になっている
      assert_eq!(
         result.instance.status(),
         ringiflow_domain::workflow::WorkflowInstanceStatus::Approved
      );

      // ステップ2も承認済み
      let result_step2 = result
         .steps
         .iter()
         .find(|s| s.step_id() == "finance_approval")
         .unwrap();
      assert_eq!(
         result_step2.status(),
         ringiflow_domain::workflow::WorkflowStepStatus::Completed
      );
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
         now,
      })
      .activated(now);
      step_repo.insert(&step, &tenant_id).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo.clone()),
         Arc::new(step_repo.clone()),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
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
      let result = result.unwrap();
      let expected = WorkflowWithSteps {
         instance: instance.complete_with_rejection(now).unwrap(),
         steps:    vec![step.reject(Some("却下理由".to_string()), now).unwrap()],
      };
      assert_eq!(result, expected);
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
      step_repo.insert(&step, &tenant_id).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
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
      step_repo.insert(&step, &tenant_id).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
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
      step_repo.insert(&step, &tenant_id).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
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

   // ===== 多段階却下テスト =====

   #[tokio::test]
   async fn test_reject_step_中間ステップ_残りのpendingステップがskippedになる() {
      // Arrange: 2段階承認で、ステップ1を却下する
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver1_id = UserId::new();
      let approver2_id = UserId::new();
      let now = chrono::Utc::now();

      let (definition, instance, step1, step2) =
         setup_two_step_approval(&tenant_id, &user_id, &approver1_id, &approver2_id, now);

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      definition_repo.add_definition(definition);
      instance_repo.insert(&instance).await.unwrap();
      step_repo.insert(&step1, &tenant_id).await.unwrap();
      step_repo.insert(&step2, &tenant_id).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo.clone()),
         Arc::new(step_repo.clone()),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      let input = ApproveRejectInput {
         version: step1.version(),
         comment: Some("却下理由".to_string()),
      };

      // Act
      let result = sut
         .reject_step(
            input,
            step1.id().clone(),
            tenant_id.clone(),
            approver1_id.clone(),
         )
         .await;

      // Assert
      let result = result.unwrap();

      // インスタンスが Rejected になっている
      assert_eq!(
         result.instance.status(),
         ringiflow_domain::workflow::WorkflowInstanceStatus::Rejected
      );

      // ステップ1は却下済み
      let result_step1 = result
         .steps
         .iter()
         .find(|s| s.step_id() == "manager_approval")
         .unwrap();
      assert_eq!(
         result_step1.status(),
         ringiflow_domain::workflow::WorkflowStepStatus::Completed
      );
      assert_eq!(
         result_step1.decision(),
         Some(ringiflow_domain::workflow::StepDecision::Rejected)
      );

      // ステップ2は Skipped
      let result_step2 = result
         .steps
         .iter()
         .find(|s| s.step_id() == "finance_approval")
         .unwrap();
      assert_eq!(
         result_step2.status(),
         ringiflow_domain::workflow::WorkflowStepStatus::Skipped
      );
   }

   #[tokio::test]
   async fn test_reject_step_最終ステップ_インスタンスがrejectedになる() {
      // Arrange: 2段階承認で、ステップ1承認後にステップ2を却下する
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver1_id = UserId::new();
      let approver2_id = UserId::new();
      let now = chrono::Utc::now();

      let (definition, instance, step1, step2) =
         setup_two_step_approval(&tenant_id, &user_id, &approver1_id, &approver2_id, now);

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      definition_repo.add_definition(definition);

      // ステップ1は既に承認済み、current_step_id は finance_approval に移行済み
      let instance_at_step2 = instance
         .advance_to_next_step("finance_approval".to_string(), now)
         .unwrap();
      instance_repo.insert(&instance_at_step2).await.unwrap();

      let completed_step1 = step1.approve(Some("上長承認OK".to_string()), now).unwrap();
      let active_step2 = step2.activated(now);
      step_repo
         .insert(&completed_step1, &tenant_id)
         .await
         .unwrap();
      step_repo.insert(&active_step2, &tenant_id).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo.clone()),
         Arc::new(step_repo.clone()),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      let input = ApproveRejectInput {
         version: active_step2.version(),
         comment: Some("経理却下".to_string()),
      };

      // Act
      let result = sut
         .reject_step(
            input,
            active_step2.id().clone(),
            tenant_id.clone(),
            approver2_id.clone(),
         )
         .await;

      // Assert
      let result = result.unwrap();

      // インスタンスが Rejected になっている
      assert_eq!(
         result.instance.status(),
         ringiflow_domain::workflow::WorkflowInstanceStatus::Rejected
      );

      // ステップ2は却下済み（スキップ対象なし）
      let result_step2 = result
         .steps
         .iter()
         .find(|s| s.step_id() == "finance_approval")
         .unwrap();
      assert_eq!(
         result_step2.status(),
         ringiflow_domain::workflow::WorkflowStepStatus::Completed
      );
      assert_eq!(
         result_step2.decision(),
         Some(ringiflow_domain::workflow::StepDecision::Rejected)
      );
   }

   // ===== submit_workflow テスト =====

   #[tokio::test]
   async fn test_submit_workflow_1段階承認の正常系() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver_id = UserId::new();
      let now = chrono::Utc::now();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      // 1段階承認の定義を追加
      let definition = WorkflowDefinition::new(NewWorkflowDefinition {
         id: WorkflowDefinitionId::new(),
         tenant_id: tenant_id.clone(),
         name: WorkflowName::new("汎用申請").unwrap(),
         description: Some("テスト用定義".to_string()),
         definition: single_approval_definition_json(),
         created_by: user_id.clone(),
         now,
      });
      let published_definition = definition.published(now).unwrap();
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
         now,
      });
      instance_repo.insert(&instance).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo.clone()),
         Arc::new(step_repo.clone()),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      let input = SubmitWorkflowInput {
         approvers: vec![StepApprover {
            step_id:     "approval".to_string(),
            assigned_to: approver_id.clone(),
         }],
      };

      // Act
      let result = sut
         .submit_workflow(input, instance.id().clone(), tenant_id.clone())
         .await;

      // Assert
      let result = result.unwrap();
      let expected = instance
         .submitted(now)
         .unwrap()
         .with_current_step("approval".to_string(), now);
      assert_eq!(result, expected);

      // ステップが作成されていることを確認
      let steps = step_repo
         .find_by_instance(result.id(), &tenant_id)
         .await
         .unwrap();
      assert_eq!(steps.len(), 1);
      assert_eq!(steps[0].assigned_to(), Some(&approver_id));
      assert_eq!(
         steps[0].status(),
         ringiflow_domain::workflow::WorkflowStepStatus::Active
      );
   }

   #[tokio::test]
   async fn test_submit_workflow_2段階承認の正常系() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver1_id = UserId::new();
      let approver2_id = UserId::new();
      let now = chrono::Utc::now();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      // 2段階承認の定義を追加
      let definition = WorkflowDefinition::new(NewWorkflowDefinition {
         id: WorkflowDefinitionId::new(),
         tenant_id: tenant_id.clone(),
         name: WorkflowName::new("2段階承認").unwrap(),
         description: Some("テスト用定義".to_string()),
         definition: two_step_approval_definition_json(),
         created_by: user_id.clone(),
         now,
      });
      let published_definition = definition.published(now).unwrap();
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
         now,
      });
      instance_repo.insert(&instance).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo.clone()),
         Arc::new(step_repo.clone()),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      let input = SubmitWorkflowInput {
         approvers: vec![
            StepApprover {
               step_id:     "manager_approval".to_string(),
               assigned_to: approver1_id.clone(),
            },
            StepApprover {
               step_id:     "finance_approval".to_string(),
               assigned_to: approver2_id.clone(),
            },
         ],
      };

      // Act
      let result = sut
         .submit_workflow(input, instance.id().clone(), tenant_id.clone())
         .await;

      // Assert
      let result = result.unwrap();
      // current_step_id は最初の承認ステップ
      assert_eq!(result.current_step_id(), Some("manager_approval"));

      // 2つのステップが作成されていること
      let steps = step_repo
         .find_by_instance(result.id(), &tenant_id)
         .await
         .unwrap();
      assert_eq!(steps.len(), 2);

      // 最初のステップは Active
      assert_eq!(steps[0].step_id(), "manager_approval");
      assert_eq!(steps[0].assigned_to(), Some(&approver1_id));
      assert_eq!(
         steps[0].status(),
         ringiflow_domain::workflow::WorkflowStepStatus::Active
      );

      // 2番目のステップは Pending
      assert_eq!(steps[1].step_id(), "finance_approval");
      assert_eq!(steps[1].assigned_to(), Some(&approver2_id));
      assert_eq!(
         steps[1].status(),
         ringiflow_domain::workflow::WorkflowStepStatus::Pending
      );
   }

   #[tokio::test]
   async fn test_submit_workflow_approversと定義のステップが一致しない場合エラー() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver_id = UserId::new();
      let now = chrono::Utc::now();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      // 2段階承認の定義だが、1人しか指定しない
      let definition = WorkflowDefinition::new(NewWorkflowDefinition {
         id: WorkflowDefinitionId::new(),
         tenant_id: tenant_id.clone(),
         name: WorkflowName::new("2段階承認").unwrap(),
         description: Some("テスト用定義".to_string()),
         definition: two_step_approval_definition_json(),
         created_by: user_id.clone(),
         now,
      });
      let published_definition = definition.published(now).unwrap();
      definition_repo.add_definition(published_definition.clone());

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
      });
      instance_repo.insert(&instance).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      // 2段階定義に1人しか指定しない
      let input = SubmitWorkflowInput {
         approvers: vec![StepApprover {
            step_id:     "manager_approval".to_string(),
            assigned_to: approver_id.clone(),
         }],
      };

      // Act
      let result = sut
         .submit_workflow(input, instance.id().clone(), tenant_id)
         .await;

      // Assert
      assert!(matches!(result, Err(CoreError::BadRequest(_))));
   }

   // ===== post_comment テスト =====

   #[tokio::test]
   async fn test_post_comment_申請者がコメントを投稿できる() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let now = chrono::Utc::now();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();
      let comment_repo = MockWorkflowCommentRepository::new();

      // InProgress のインスタンスを作成（user_id が申請者）
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

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(comment_repo),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      let input = PostCommentInput {
         body: "テストコメント".to_string(),
      };

      // Act
      let result = sut
         .post_comment(input, DisplayNumber::new(100).unwrap(), tenant_id, user_id)
         .await;

      // Assert
      assert!(result.is_ok());
      let comment = result.unwrap();
      assert_eq!(comment.body().as_str(), "テストコメント");
   }

   #[tokio::test]
   async fn test_post_comment_承認者がコメントを投稿できる() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver_id = UserId::new();
      let now = chrono::Utc::now();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();
      let comment_repo = MockWorkflowCommentRepository::new();

      // InProgress のインスタンスを作成（user_id が申請者）
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

      // approver_id が承認者のステップを作成
      let step = WorkflowStep::new(NewWorkflowStep {
         id: WorkflowStepId::new(),
         instance_id: instance.id().clone(),
         display_number: DisplayNumber::new(1).unwrap(),
         step_id: "approval".to_string(),
         step_name: "承認".to_string(),
         step_type: "approval".to_string(),
         assigned_to: Some(approver_id.clone()),
         now,
      })
      .activated(now);
      step_repo.insert(&step, &tenant_id).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(comment_repo),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      let input = PostCommentInput {
         body: "承認者のコメント".to_string(),
      };

      // Act: 承認者がコメントを投稿
      let result = sut
         .post_comment(
            input,
            DisplayNumber::new(100).unwrap(),
            tenant_id,
            approver_id,
         )
         .await;

      // Assert
      assert!(result.is_ok());
      let comment = result.unwrap();
      assert_eq!(comment.body().as_str(), "承認者のコメント");
   }

   #[tokio::test]
   async fn test_post_comment_関与していないユーザーは403() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let other_user_id = UserId::new(); // 関与していないユーザー
      let now = chrono::Utc::now();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();
      let comment_repo = MockWorkflowCommentRepository::new();

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

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(comment_repo),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      let input = PostCommentInput {
         body: "無関係なコメント".to_string(),
      };

      // Act: 関与していないユーザーがコメントを試みる
      let result = sut
         .post_comment(
            input,
            DisplayNumber::new(100).unwrap(),
            tenant_id,
            other_user_id,
         )
         .await;

      // Assert
      assert!(matches!(result, Err(CoreError::Forbidden(_))));
   }

   #[tokio::test]
   async fn test_post_comment_ワークフローが見つからない場合404() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let now = chrono::Utc::now();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();
      let comment_repo = MockWorkflowCommentRepository::new();

      // インスタンスを作成しない

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(comment_repo),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      let input = PostCommentInput {
         body: "存在しないワークフローへのコメント".to_string(),
      };

      // Act
      let result = sut
         .post_comment(input, DisplayNumber::new(999).unwrap(), tenant_id, user_id)
         .await;

      // Assert
      assert!(matches!(result, Err(CoreError::NotFound(_))));
   }

   // ===== submit_workflow テスト =====

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
         definition:  single_approval_definition_json(),
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
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      let input = SubmitWorkflowInput {
         approvers: vec![StepApprover {
            step_id:     "approval".to_string(),
            assigned_to: approver_id.clone(),
         }],
      };

      // Act: InProgress 状態のインスタンスに対して申請を試みる
      let result = sut
         .submit_workflow(input, instance.id().clone(), tenant_id)
         .await;

      // Assert
      assert!(matches!(result, Err(CoreError::BadRequest(_))));
   }

   // ===== request_changes_step テスト =====

   #[tokio::test]
   async fn test_request_changes_step_正常系() {
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
         now,
      })
      .activated(now);
      step_repo.insert(&step, &tenant_id).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo.clone()),
         Arc::new(step_repo.clone()),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      let input = ApproveRejectInput {
         version: step.version(),
         comment: Some("金額を修正してください".to_string()),
      };

      // Act
      let result = sut
         .request_changes_step(
            input,
            step.id().clone(),
            tenant_id.clone(),
            approver_id.clone(),
         )
         .await;

      // Assert
      let result = result.unwrap();
      let expected = WorkflowWithSteps {
         instance: instance.complete_with_request_changes(now).unwrap(),
         steps:    vec![
            step
               .request_changes(Some("金額を修正してください".to_string()), now)
               .unwrap(),
         ],
      };
      assert_eq!(result, expected);
   }

   #[tokio::test]
   async fn test_request_changes_step_未割り当てユーザーは403() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver_id = UserId::new();
      let other_user_id = UserId::new();

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
         now,
      })
      .activated(now);
      step_repo.insert(&step, &tenant_id).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      let input = ApproveRejectInput {
         version: step.version(),
         comment: None,
      };

      // Act: 別のユーザーで差し戻しを試みる
      let result = sut
         .request_changes_step(input, step.id().clone(), tenant_id, other_user_id)
         .await;

      // Assert
      assert!(matches!(result, Err(CoreError::Forbidden(_))));
   }

   #[tokio::test]
   async fn test_request_changes_step_active以外は400() {
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

      // Pending 状態のステップ
      let step = WorkflowStep::new(NewWorkflowStep {
         id: WorkflowStepId::new(),
         instance_id: instance.id().clone(),
         display_number: DisplayNumber::new(1).unwrap(),
         step_id: "approval".to_string(),
         step_name: "承認".to_string(),
         step_type: "approval".to_string(),
         assigned_to: Some(approver_id.clone()),
         now,
      });
      step_repo.insert(&step, &tenant_id).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      let input = ApproveRejectInput {
         version: step.version(),
         comment: None,
      };

      // Act
      let result = sut
         .request_changes_step(input, step.id().clone(), tenant_id, approver_id)
         .await;

      // Assert
      assert!(matches!(result, Err(CoreError::BadRequest(_))));
   }

   #[tokio::test]
   async fn test_request_changes_step_バージョン不一致で409() {
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
         now,
      })
      .activated(now);
      step_repo.insert(&step, &tenant_id).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      let wrong_version = Version::initial().next();
      let input = ApproveRejectInput {
         version: wrong_version,
         comment: None,
      };

      // Act
      let result = sut
         .request_changes_step(input, step.id().clone(), tenant_id, approver_id)
         .await;

      // Assert
      assert!(matches!(result, Err(CoreError::Conflict(_))));
   }

   #[tokio::test]
   async fn test_request_changes_step_残りのpendingステップがskipped() {
      // Arrange: 2段階承認でステップ1を差し戻し → ステップ2は Skipped
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver1_id = UserId::new();
      let approver2_id = UserId::new();
      let now = chrono::Utc::now();

      let (definition, instance, step1, step2) =
         setup_two_step_approval(&tenant_id, &user_id, &approver1_id, &approver2_id, now);

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      definition_repo.add_definition(definition);
      instance_repo.insert(&instance).await.unwrap();
      step_repo.insert(&step1, &tenant_id).await.unwrap();
      step_repo.insert(&step2, &tenant_id).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo.clone()),
         Arc::new(step_repo.clone()),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      let input = ApproveRejectInput {
         version: step1.version(),
         comment: Some("修正してください".to_string()),
      };

      // Act
      let result = sut
         .request_changes_step(
            input,
            step1.id().clone(),
            tenant_id.clone(),
            approver1_id.clone(),
         )
         .await;

      // Assert
      let result = result.unwrap();

      // インスタンスが ChangesRequested になっている
      assert_eq!(
         result.instance.status(),
         ringiflow_domain::workflow::WorkflowInstanceStatus::ChangesRequested
      );

      // ステップ2は Skipped
      let result_step2 = result
         .steps
         .iter()
         .find(|s| s.step_id() == "finance_approval")
         .unwrap();
      assert_eq!(
         result_step2.status(),
         ringiflow_domain::workflow::WorkflowStepStatus::Skipped
      );
   }

   // ===== resubmit_workflow テスト =====

   #[tokio::test]
   async fn test_resubmit_workflow_正常系() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver_id = UserId::new();
      let now = chrono::Utc::now();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      // 1段階承認の定義を追加
      let definition = WorkflowDefinition::new(NewWorkflowDefinition {
         id: WorkflowDefinitionId::new(),
         tenant_id: tenant_id.clone(),
         name: WorkflowName::new("汎用申請").unwrap(),
         description: Some("テスト用定義".to_string()),
         definition: single_approval_definition_json(),
         created_by: user_id.clone(),
         now,
      })
      .published(now)
      .unwrap();
      definition_repo.add_definition(definition.clone());

      // ChangesRequested 状態のインスタンスを作成
      let instance = WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id: tenant_id.clone(),
         definition_id: definition.id().clone(),
         definition_version: Version::initial(),
         display_number: DisplayNumber::new(100).unwrap(),
         title: "テスト申請".to_string(),
         form_data: serde_json::json!({"note": "original"}),
         initiated_by: user_id.clone(),
         now,
      })
      .submitted(now)
      .unwrap()
      .with_current_step("approval".to_string(), now)
      .complete_with_request_changes(now)
      .unwrap();
      instance_repo.insert(&instance).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo.clone()),
         Arc::new(step_repo.clone()),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      let input = ResubmitWorkflowInput {
         form_data: serde_json::json!({"note": "updated"}),
         approvers: vec![StepApprover {
            step_id:     "approval".to_string(),
            assigned_to: approver_id.clone(),
         }],
         version:   instance.version(),
      };

      // Act
      let result = sut
         .resubmit_workflow(
            input,
            instance.id().clone(),
            tenant_id.clone(),
            user_id.clone(),
         )
         .await;

      // Assert
      let result = result.unwrap();

      // ステータスが InProgress に戻っている
      assert_eq!(
         result.instance.status(),
         ringiflow_domain::workflow::WorkflowInstanceStatus::InProgress
      );

      // form_data が更新されている
      assert_eq!(
         result.instance.form_data(),
         &serde_json::json!({"note": "updated"})
      );

      // 新しいステップが作成されている
      assert_eq!(result.steps.len(), 1);
      assert_eq!(result.steps[0].assigned_to(), Some(&approver_id));
      assert_eq!(
         result.steps[0].status(),
         ringiflow_domain::workflow::WorkflowStepStatus::Active
      );
   }

   #[tokio::test]
   async fn test_resubmit_workflow_要修正以外は400() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver_id = UserId::new();
      let now = chrono::Utc::now();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      // InProgress 状態のインスタンス（ChangesRequested ではない）
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

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      let input = ResubmitWorkflowInput {
         form_data: serde_json::json!({}),
         approvers: vec![StepApprover {
            step_id:     "approval".to_string(),
            assigned_to: approver_id.clone(),
         }],
         version:   instance.version(),
      };

      // Act
      let result = sut
         .resubmit_workflow(input, instance.id().clone(), tenant_id, user_id)
         .await;

      // Assert
      assert!(matches!(result, Err(CoreError::BadRequest(_))));
   }

   #[tokio::test]
   async fn test_resubmit_workflow_バージョン不一致で409() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver_id = UserId::new();
      let now = chrono::Utc::now();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      let definition = WorkflowDefinition::new(NewWorkflowDefinition {
         id: WorkflowDefinitionId::new(),
         tenant_id: tenant_id.clone(),
         name: WorkflowName::new("汎用申請").unwrap(),
         description: Some("テスト用定義".to_string()),
         definition: single_approval_definition_json(),
         created_by: user_id.clone(),
         now,
      })
      .published(now)
      .unwrap();
      definition_repo.add_definition(definition.clone());

      let instance = WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id: tenant_id.clone(),
         definition_id: definition.id().clone(),
         definition_version: Version::initial(),
         display_number: DisplayNumber::new(100).unwrap(),
         title: "テスト申請".to_string(),
         form_data: serde_json::json!({}),
         initiated_by: user_id.clone(),
         now,
      })
      .submitted(now)
      .unwrap()
      .with_current_step("approval".to_string(), now)
      .complete_with_request_changes(now)
      .unwrap();
      instance_repo.insert(&instance).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      let wrong_version = Version::initial(); // actual: initial.next().next() (submitted + request_changes)
      let input = ResubmitWorkflowInput {
         form_data: serde_json::json!({}),
         approvers: vec![StepApprover {
            step_id:     "approval".to_string(),
            assigned_to: approver_id.clone(),
         }],
         version:   wrong_version,
      };

      // Act
      let result = sut
         .resubmit_workflow(input, instance.id().clone(), tenant_id, user_id)
         .await;

      // Assert
      assert!(matches!(result, Err(CoreError::Conflict(_))));
   }

   #[tokio::test]
   async fn test_resubmit_workflow_approvers不一致でエラー() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let approver_id = UserId::new();
      let now = chrono::Utc::now();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      // 1段階承認の定義だが、2人指定する
      let definition = WorkflowDefinition::new(NewWorkflowDefinition {
         id: WorkflowDefinitionId::new(),
         tenant_id: tenant_id.clone(),
         name: WorkflowName::new("汎用申請").unwrap(),
         description: Some("テスト用定義".to_string()),
         definition: single_approval_definition_json(),
         created_by: user_id.clone(),
         now,
      })
      .published(now)
      .unwrap();
      definition_repo.add_definition(definition.clone());

      let instance = WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id: tenant_id.clone(),
         definition_id: definition.id().clone(),
         definition_version: Version::initial(),
         display_number: DisplayNumber::new(100).unwrap(),
         title: "テスト申請".to_string(),
         form_data: serde_json::json!({}),
         initiated_by: user_id.clone(),
         now,
      })
      .submitted(now)
      .unwrap()
      .with_current_step("approval".to_string(), now)
      .complete_with_request_changes(now)
      .unwrap();
      instance_repo.insert(&instance).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      // 1段階定義に2人指定
      let input = ResubmitWorkflowInput {
         form_data: serde_json::json!({}),
         approvers: vec![
            StepApprover {
               step_id:     "approval".to_string(),
               assigned_to: approver_id.clone(),
            },
            StepApprover {
               step_id:     "extra".to_string(),
               assigned_to: UserId::new(),
            },
         ],
         version:   instance.version(),
      };

      // Act
      let result = sut
         .resubmit_workflow(input, instance.id().clone(), tenant_id, user_id)
         .await;

      // Assert
      assert!(matches!(result, Err(CoreError::BadRequest(_))));
   }

   #[tokio::test]
   async fn test_resubmit_workflow_申請者以外は403() {
      // Arrange
      let tenant_id = TenantId::new();
      let user_id = UserId::new();
      let other_user_id = UserId::new();
      let approver_id = UserId::new();
      let now = chrono::Utc::now();

      let definition_repo = MockWorkflowDefinitionRepository::new();
      let instance_repo = MockWorkflowInstanceRepository::new();
      let step_repo = MockWorkflowStepRepository::new();

      let definition = WorkflowDefinition::new(NewWorkflowDefinition {
         id: WorkflowDefinitionId::new(),
         tenant_id: tenant_id.clone(),
         name: WorkflowName::new("汎用申請").unwrap(),
         description: Some("テスト用定義".to_string()),
         definition: single_approval_definition_json(),
         created_by: user_id.clone(),
         now,
      })
      .published(now)
      .unwrap();
      definition_repo.add_definition(definition.clone());

      let instance = WorkflowInstance::new(NewWorkflowInstance {
         id: WorkflowInstanceId::new(),
         tenant_id: tenant_id.clone(),
         definition_id: definition.id().clone(),
         definition_version: Version::initial(),
         display_number: DisplayNumber::new(100).unwrap(),
         title: "テスト申請".to_string(),
         form_data: serde_json::json!({}),
         initiated_by: user_id.clone(), // user_id が申請者
         now,
      })
      .submitted(now)
      .unwrap()
      .with_current_step("approval".to_string(), now)
      .complete_with_request_changes(now)
      .unwrap();
      instance_repo.insert(&instance).await.unwrap();

      let sut = WorkflowUseCaseImpl::new(
         Arc::new(definition_repo),
         Arc::new(instance_repo),
         Arc::new(step_repo),
         Arc::new(MockWorkflowCommentRepository::new()),
         Arc::new(MockUserRepository),
         Arc::new(MockDisplayIdCounterRepository::new()),
         Arc::new(FixedClock::new(now)),
      );

      let input = ResubmitWorkflowInput {
         form_data: serde_json::json!({}),
         approvers: vec![StepApprover {
            step_id:     "approval".to_string(),
            assigned_to: approver_id.clone(),
         }],
         version:   instance.version(),
      };

      // Act: 別のユーザーで再申請を試みる
      let result = sut
         .resubmit_workflow(
            input,
            instance.id().clone(),
            tenant_id,
            other_user_id, // 申請者ではない
         )
         .await;

      // Assert
      assert!(matches!(result, Err(CoreError::Forbidden(_))));
   }
}
