//! Core Service クライアントのエラー型

use thiserror::Error;

/// Core Service クライアントエラー
#[derive(Debug, Clone, Error)]
pub enum CoreServiceError {
   /// ユーザーが見つからない（404）
   #[error("ユーザーが見つかりません")]
   UserNotFound,

   /// ワークフロー定義が見つからない（404）
   #[error("ワークフロー定義が見つかりません")]
   WorkflowDefinitionNotFound,

   /// ワークフローインスタンスが見つからない（404）
   #[error("ワークフローインスタンスが見つかりません")]
   WorkflowInstanceNotFound,

   /// ステップが見つからない（404）
   #[error("ステップが見つかりません")]
   StepNotFound,

   /// バリデーションエラー（400）
   #[error("バリデーションエラー: {0}")]
   ValidationError(String),

   /// 権限不足（403）
   #[error("権限がありません: {0}")]
   Forbidden(String),

   /// メールアドレスが既に使用されている（409）
   #[error("メールアドレスは既に使用されています")]
   EmailAlreadyExists,

   /// 競合（409）
   #[error("競合が発生しました: {0}")]
   Conflict(String),

   /// ネットワークエラー
   #[error("ネットワークエラー: {0}")]
   Network(String),

   /// 予期しないエラー
   #[error("予期しないエラー: {0}")]
   Unexpected(String),
}

impl From<reqwest::Error> for CoreServiceError {
   fn from(err: reqwest::Error) -> Self {
      CoreServiceError::Network(err.to_string())
   }
}
