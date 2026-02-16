//! # ヘルスチェックハンドラ
//!
//! Auth Service の稼働状態を確認するためのエンドポイント。
//!
//! レスポンス型は [`ringiflow_shared::HealthResponse`] を参照。

use axum::Json;
use ringiflow_shared::HealthResponse;

/// Auth Service のヘルスチェックエンドポイント
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status:  "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}
