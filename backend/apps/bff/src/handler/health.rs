//! # ヘルスチェックハンドラ
//!
//! BFF の稼働状態を確認するためのエンドポイント。
//!
//! - `/health` — Liveness Check（常に `"healthy"` を返す）
//! - `/health/ready` — Readiness Check（Redis / Core Service / DB の接続状態を確認）
//!
//! レスポンス型は [`ringiflow_shared::HealthResponse`] / [`ringiflow_shared::ReadinessResponse`] を参照。

use std::{collections::HashMap, sync::Arc, time::Duration};

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use redis::aio::ConnectionManager;
use ringiflow_shared::{CheckStatus, HealthResponse, ReadinessResponse, ReadinessStatus};

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

/// Readiness Check 用の State
pub struct ReadinessState {
    pub redis_conn:       ConnectionManager,
    pub core_service_url: String,
    pub http_client:      reqwest::Client,
}

/// BFF の Readiness Check エンドポイント
///
/// Redis と Core Service（DB 含む）の接続状態を並行チェックする。
/// 全チェック OK → 200、1 つでも失敗 → 503。
#[utoipa::path(
   get,
   path = "/health/ready",
   tag = "health",
   responses(
      (status = 200, description = "全依存サービス稼働中", body = ReadinessResponse),
      (status = 503, description = "一部の依存サービスが利用不可", body = ReadinessResponse)
   )
)]
#[tracing::instrument(skip_all)]
pub async fn readiness_check(State(state): State<Arc<ReadinessState>>) -> impl IntoResponse {
    // Redis と Core Service を並行チェック
    let (redis_result, core_result) = tokio::join!(
        check_redis(state.redis_conn.clone()),
        check_core_service(&state.http_client, &state.core_service_url),
    );

    let mut checks = HashMap::new();
    checks.insert("redis".to_string(), redis_result);
    checks.insert("core_api".to_string(), core_result.core_api);
    checks.insert("database".to_string(), core_result.database);

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

/// Redis への接続を PING で確認する（タイムアウト: 5 秒）
async fn check_redis(mut conn: ConnectionManager) -> CheckStatus {
    match tokio::time::timeout(
        Duration::from_secs(5),
        redis::cmd("PING").query_async::<String>(&mut conn),
    )
    .await
    {
        Ok(Ok(_)) => CheckStatus::Ok,
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "readiness check: redis ping failed");
            CheckStatus::Error
        }
        Err(_) => {
            tracing::warn!("readiness check: redis check timed out");
            CheckStatus::Error
        }
    }
}

/// Core Service チェックの結果
struct CoreCheckResult {
    core_api: CheckStatus,
    database: CheckStatus,
}

/// Core Service の `/health/ready` を呼び、結果をマッピングする（タイムアウト: 5 秒）
///
/// Core Service が 503 を返す場合でもボディをパースし、
/// `database` チェック結果を取得する。
async fn check_core_service(client: &reqwest::Client, base_url: &str) -> CoreCheckResult {
    let url = format!("{base_url}/health/ready");
    match tokio::time::timeout(Duration::from_secs(5), client.get(&url).send()).await {
        Ok(Ok(response)) => {
            // 503 でもボディをパースする（Core は 503 + ReadinessResponse を返す）
            match response.json::<ReadinessResponse>().await {
                Ok(body) => CoreCheckResult {
                    core_api: CheckStatus::Ok,
                    database: body
                        .checks
                        .get("database")
                        .cloned()
                        .unwrap_or(CheckStatus::Error),
                },
                Err(e) => {
                    tracing::warn!(error = %e, "readiness check: core service response parse failed");
                    CoreCheckResult {
                        core_api: CheckStatus::Error,
                        database: CheckStatus::Error,
                    }
                }
            }
        }
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "readiness check: core service request failed");
            CoreCheckResult {
                core_api: CheckStatus::Error,
                database: CheckStatus::Error,
            }
        }
        Err(_) => {
            tracing::warn!("readiness check: core service check timed out");
            CoreCheckResult {
                core_api: CheckStatus::Error,
                database: CheckStatus::Error,
            }
        }
    }
}
