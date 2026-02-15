//! # ヘルスチェックハンドラ
//!
//! Core API の稼働状態を確認するためのエンドポイント。
//!
//! ## 用途
//!
//! - **ロードバランサー**: ALB/NLB のターゲットグループヘルスチェック
//! - **コンテナオーケストレーター**: ECS/Kubernetes の liveness/readiness probe
//! - **BFF からの死活確認**: BFF が Core API の状態を確認
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

use axum::Json;
use serde::Serialize;

/// ヘルスチェックレスポンス
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
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status:  "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}
