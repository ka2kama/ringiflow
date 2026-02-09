//! # ユーザー API ハンドラ
//!
//! BFF のユーザー関連エンドポイントを提供する。
//!
//! ## エンドポイント
//!
//! - `GET /api/v1/users` - テナント内のアクティブユーザー一覧

use std::sync::Arc;

use axum::{
   Json,
   extract::State,
   http::{HeaderMap, StatusCode},
   response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use ringiflow_shared::ApiResponse;
use serde::Serialize;

use super::workflow::WorkflowState;
use crate::error::{extract_tenant_id, get_session, internal_error_response};

// --- レスポンス型 ---

/// ユーザー一覧の要素データ
#[derive(Debug, Serialize)]
pub struct UserItemData {
   pub id: String,
   pub display_id: String,
   pub display_number: i64,
   pub name: String,
   pub email: String,
}

impl From<crate::client::UserItemDto> for UserItemData {
   fn from(dto: crate::client::UserItemDto) -> Self {
      Self {
         id: dto.id.to_string(),
         display_id: dto.display_id,
         display_number: dto.display_number,
         name: dto.name,
         email: dto.email,
      }
   }
}

// --- ハンドラ ---

/// GET /api/v1/users
///
/// テナント内のアクティブユーザー一覧を取得する
pub async fn list_users(
   State(state): State<Arc<WorkflowState>>,
   headers: HeaderMap,
   jar: CookieJar,
) -> impl IntoResponse {
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   let session_data = match get_session(state.session_manager.as_ref(), &jar, tenant_id).await {
      Ok(data) => data,
      Err(response) => return response,
   };

   match state
      .core_service_client
      .list_users(*session_data.tenant_id().as_uuid())
      .await
   {
      Ok(core_response) => {
         let response = ApiResponse::new(
            core_response
               .data
               .into_iter()
               .map(UserItemData::from)
               .collect::<Vec<_>>(),
         );
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(e) => {
         tracing::error!("ユーザー一覧取得で内部エラー: {}", e);
         internal_error_response()
      }
   }
}
