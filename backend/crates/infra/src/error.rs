//! # インフラ層エラー定義
//!
//! データベースや外部サービスとの通信で発生するエラーを表現する。
//!
//! ## 設計方針
//!
//! - **エラーの変換**: sqlx::Error, redis::RedisError などをラップ
//! - **ドメインエラーとの分離**: インフラ固有のエラーを明示
//! - **ログ可能性**: Debug derive によりログ出力時に詳細情報を表示

use thiserror::Error;

/// インフラ層で発生するエラー
///
/// データベースクエリ、Redis 操作、外部 API 呼び出しなどで発生するエラーを表現する。
/// API 層でこのエラーを受け取り、適切な HTTP レスポンスに変換する。
#[derive(Debug, Error)]
pub enum InfraError {
   /// データベースエラー
   ///
   /// SQLクエリの実行失敗、接続エラー、制約違反など。
   #[error("データベースエラー: {0}")]
   Database(#[from] sqlx::Error),

   /// Redis エラー
   ///
   /// Redis への接続失敗、コマンド実行エラーなど。
   #[error("Redis エラー: {0}")]
   Redis(#[from] redis::RedisError),

   /// シリアライズ/デシリアライズエラー
   ///
   /// JSON の変換に失敗した場合に使用する。
   #[error("シリアライズエラー: {0}")]
   Serialization(#[from] serde_json::Error),

   /// 予期しないエラー
   ///
   /// 上記に分類できない予期しないエラー。
   #[error("予期しないエラー: {0}")]
   Unexpected(String),
}
