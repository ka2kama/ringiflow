//! # ヘルスチェックハンドラ
//!
//! BFF の稼働状態を確認するためのエンドポイント。
//!
//! レスポンス型は [`ringiflow_shared::HealthResponse`] を参照。

use axum::Json;
use ringiflow_shared::HealthResponse;

/// BFF のヘルスチェックエンドポイント
#[utoipa::path(
   get,
   path = "/health",
   tag = "health",
   responses(
      (status = 200, description = "サーバー稼働中", body = HealthResponse)
   )
)]
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status:  "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}
