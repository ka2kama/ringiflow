//! # ヘルスチェックハンドラ
//!
//! アプリケーションの稼働状態を確認するためのエンドポイント。
//!
//! ## 用途
//!
//! - **ロードバランサー**: ALB/NLB のターゲットグループヘルスチェック
//! - **コンテナオーケストレーター**: ECS/Kubernetes の liveness/readiness probe
//! - **監視システム**: 外部監視サービスからの死活監視
//!
//! ## エンドポイント
//!
//! ```text
//! GET /health
//! ```
//!
//! ## レスポンス例
//!
//! ```json
//! {
//!   "status": "healthy",
//!   "version": "0.1.0"
//! }
//! ```
//!
//! ## 将来の拡張
//!
//! - データベース接続状態の確認
//! - Redis 接続状態の確認
//! - 依存サービスの状態確認
//! - 詳細なメトリクス（メモリ使用量、CPU 使用率など）

use axum::Json;
use serde::Serialize;

/// ヘルスチェックレスポンス
///
/// アプリケーションの稼働状態を表現する。
/// 監視システムやロードバランサーがこのレスポンスを解析して
/// サービスの可用性を判断する。
#[derive(Debug, Serialize)]
pub struct HealthResponse {
   /// 稼働状態（`"healthy"` または `"unhealthy"`）
   pub status:  String,
   /// アプリケーションバージョン（Cargo.toml から取得）
   pub version: String,
}

/// ヘルスチェックエンドポイント
///
/// サーバーが正常に稼働していることを確認するためのエンドポイント。
/// データベースや外部サービスへの接続は確認せず、アプリケーション自体の
/// 起動状態のみを返す。
///
/// # レスポンス
///
/// 常に 200 OK を返す。レスポンスボディには以下を含む:
///
/// - `status`: `"healthy"`（固定）
/// - `version`: `Cargo.toml` で定義されたバージョン
///
/// # 使用例
///
/// ```text
/// $ curl http://localhost:13000/health
/// {"status":"healthy","version":"0.1.0"}
/// ```
///
/// # AWS ALB での設定例
///
/// ```text
/// HealthCheckPath: /health
/// HealthCheckIntervalSeconds: 30
/// HealthyThresholdCount: 2
/// UnhealthyThresholdCount: 3
/// ```
pub async fn health_check() -> Json<HealthResponse> {
   Json(HealthResponse {
      status:  "healthy".to_string(),
      version: env!("CARGO_PKG_VERSION").to_string(),
   })
}
