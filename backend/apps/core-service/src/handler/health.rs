//! # ヘルスチェックハンドラ
//!
//! Core API の稼働状態を確認するためのエンドポイント。
//!
//! - `/health` — Liveness Check（常に `"healthy"` を返す）
//! - `/health/ready` — Readiness Check（依存サービスの接続状態を確認）
//!
//! レスポンス型は [`ringiflow_shared::HealthResponse`] / [`ringiflow_shared::ReadinessResponse`] を参照。

use std::{collections::HashMap, sync::Arc, time::Duration};

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use ringiflow_shared::{CheckStatus, HealthResponse, ReadinessResponse, ReadinessStatus};
use sqlx::PgPool;

/// Core Service のヘルスチェックエンドポイント
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status:  "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Readiness Check 用の State
pub struct ReadinessState {
    pub pool: PgPool,
}

/// Core Service の Readiness Check エンドポイント
///
/// PostgreSQL への接続を確認し、結果を返す。
/// 全チェック OK → 200、1 つでも失敗 → 503。
pub async fn readiness_check(State(state): State<Arc<ReadinessState>>) -> impl IntoResponse {
    let db_check = check_database(&state.pool).await;

    let mut checks = HashMap::new();
    checks.insert("database".to_string(), db_check);

    let all_ok = checks.values().all(|s| matches!(s, CheckStatus::Ok));
    let status = if all_ok {
        ReadinessStatus::Ready
    } else {
        ReadinessStatus::NotReady
    };
    let http_status = if all_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (http_status, Json(ReadinessResponse { status, checks }))
}

/// PostgreSQL への接続を `SELECT 1` で確認する（タイムアウト: 5 秒）
async fn check_database(pool: &PgPool) -> CheckStatus {
    match tokio::time::timeout(
        Duration::from_secs(5),
        sqlx::query("SELECT 1").execute(pool),
    )
    .await
    {
        Ok(Ok(_)) => CheckStatus::Ok,
        _ => CheckStatus::Error,
    }
}
