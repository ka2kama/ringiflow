//! ワークフローユースケースの読み取り操作

use ringiflow_domain::{
   tenant::TenantId,
   user::UserId,
   value_objects::DisplayNumber,
   workflow::{WorkflowDefinition, WorkflowDefinitionId, WorkflowInstanceId},
};

use super::{WorkflowUseCaseImpl, WorkflowWithSteps};
use crate::error::CoreError;

impl WorkflowUseCaseImpl {
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
   ) -> Result<Vec<ringiflow_domain::workflow::WorkflowInstance>, CoreError> {
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

   // ===== display_number 対応メソッド（読み取り） =====

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
}
