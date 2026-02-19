//! # ダッシュボード API ハンドラ
//!
//! Core Service のダッシュボード関連エンドポイントを実装する。

use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use ringiflow_domain::{tenant::TenantId, user::UserId};
use ringiflow_shared::ApiResponse;
use serde::Serialize;

use crate::{error::CoreError, handler::workflow::UserQuery, usecase::dashboard::DashboardStats};

/// ダッシュボードハンドラーの State
pub struct DashboardState {
    pub usecase: crate::usecase::DashboardUseCaseImpl,
}

/// ダッシュボード統計 DTO
#[derive(Debug, Serialize)]
pub struct DashboardStatsDto {
    pub pending_tasks: i64,
    pub my_workflows_in_progress: i64,
    pub completed_today: i64,
}

impl From<DashboardStats> for DashboardStatsDto {
    fn from(stats: DashboardStats) -> Self {
        Self {
            pending_tasks: stats.pending_tasks,
            my_workflows_in_progress: stats.my_workflows_in_progress,
            completed_today: stats.completed_today,
        }
    }
}

/// ダッシュボード統計を取得する
///
/// ## エンドポイント
/// GET /internal/dashboard/stats?tenant_id={tenant_id}&user_id={user_id}
#[tracing::instrument(skip_all)]
pub async fn get_dashboard_stats(
    State(state): State<Arc<DashboardState>>,
    Query(query): Query<UserQuery>,
) -> Result<Response, CoreError> {
    let tenant_id = TenantId::from_uuid(query.tenant_id);
    let user_id = UserId::from_uuid(query.user_id);

    let stats = state
        .usecase
        .get_stats(tenant_id, user_id, Utc::now())
        .await?;

    let response = ApiResponse::new(DashboardStatsDto::from(stats));

    Ok((StatusCode::OK, Json(response)).into_response())
}
