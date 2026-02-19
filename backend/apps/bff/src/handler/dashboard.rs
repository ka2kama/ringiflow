//! # ダッシュボード API ハンドラ
//!
//! BFF のダッシュボード関連エンドポイントを提供する。
//!
//! ## エンドポイント
//!
//! - `GET /api/v1/dashboard/stats` - ダッシュボード統計情報

use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use ringiflow_shared::{ApiResponse, ErrorResponse};
use serde::Serialize;
use utoipa::ToSchema;

use super::workflow::WorkflowState;
use crate::{
    client::DashboardStatsDto,
    error::{authenticate, log_and_convert_core_error},
};

// --- レスポンス型 ---

/// ダッシュボード統計データ
#[derive(Debug, Serialize, ToSchema)]
pub struct DashboardStatsData {
    pub pending_tasks: i64,
    pub my_workflows_in_progress: i64,
    pub completed_today: i64,
}

impl From<DashboardStatsDto> for DashboardStatsData {
    fn from(dto: DashboardStatsDto) -> Self {
        Self {
            pending_tasks: dto.pending_tasks,
            my_workflows_in_progress: dto.my_workflows_in_progress,
            completed_today: dto.completed_today,
        }
    }
}

// --- ハンドラ ---

/// GET /api/v1/dashboard/stats
///
/// ダッシュボード統計情報を取得する
#[utoipa::path(
   get,
   path = "/api/v1/dashboard/stats",
   tag = "dashboard",
   security(("session_auth" = [])),
   responses(
      (status = 200, description = "ダッシュボード統計", body = ApiResponse<DashboardStatsData>),
      (status = 401, description = "認証エラー", body = ErrorResponse)
   )
)]
#[tracing::instrument(skip_all)]
pub async fn get_dashboard_stats(
    State(state): State<Arc<WorkflowState>>,
    headers: HeaderMap,
    jar: CookieJar,
) -> Result<Response, Response> {
    let session_data = authenticate(state.session_manager.as_ref(), &headers, &jar).await?;

    let core_response = state
        .core_service_client
        .get_dashboard_stats(
            *session_data.tenant_id().as_uuid(),
            *session_data.user_id().as_uuid(),
        )
        .await
        .map_err(|e| log_and_convert_core_error("ダッシュボード統計取得", e))?;

    let response = ApiResponse::new(DashboardStatsData::from(core_response.data));
    Ok((StatusCode::OK, Json(response)).into_response())
}
