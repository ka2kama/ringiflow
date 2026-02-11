//! # ロール管理 API ハンドラ
//!
//! BFF のロール管理エンドポイントを提供する。
//!
//! ## エンドポイント
//!
//! - `GET /api/v1/roles` - テナント内のロール一覧（ユーザー数付き）
//! - `POST /api/v1/roles` - カスタムロール作成
//! - `GET /api/v1/roles/{role_id}` - ロール詳細
//! - `PATCH /api/v1/roles/{role_id}` - カスタムロール更新
//! - `DELETE /api/v1/roles/{role_id}` - カスタムロール削除

use std::sync::Arc;

use axum::{
   Json,
   extract::{Path, State},
   http::{HeaderMap, StatusCode},
   response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use ringiflow_infra::SessionManager;
use ringiflow_shared::ApiResponse;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
   client::{
      CoreServiceError,
      CoreServiceRoleClient,
      CreateRoleCoreRequest,
      UpdateRoleCoreRequest,
   },
   error::{
      conflict_response,
      extract_tenant_id,
      get_session,
      internal_error_response,
      not_found_response,
      validation_error_response,
   },
};

/// ロール管理 API の共有状態
pub struct RoleState {
   pub core_service_client: Arc<dyn CoreServiceRoleClient>,
   pub session_manager:     Arc<dyn SessionManager>,
}

// --- リクエスト型 ---

/// ロール作成リクエスト
#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
   pub name:        String,
   pub description: Option<String>,
   pub permissions: Vec<String>,
}

/// ロール更新リクエスト
#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
   pub name:        Option<String>,
   pub description: Option<String>,
   pub permissions: Option<Vec<String>>,
}

// --- レスポンス型 ---

/// ロール一覧の要素データ
#[derive(Debug, Serialize)]
pub struct RoleItemData {
   pub id:          String,
   pub name:        String,
   pub description: Option<String>,
   pub permissions: Vec<String>,
   pub is_system:   bool,
   pub user_count:  i64,
}

/// ロール詳細データ
#[derive(Debug, Serialize)]
pub struct RoleDetailData {
   pub id:          String,
   pub name:        String,
   pub description: Option<String>,
   pub permissions: Vec<String>,
   pub is_system:   bool,
   pub created_at:  String,
   pub updated_at:  String,
}

// --- ハンドラ ---

/// GET /api/v1/roles
///
/// テナント内のロール一覧をユーザー数付きで取得する。
/// system_admin は除外される。
pub async fn list_roles(
   State(state): State<Arc<RoleState>>,
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
      .list_roles(*session_data.tenant_id().as_uuid())
      .await
   {
      Ok(core_response) => {
         let items: Vec<RoleItemData> = core_response
            .data
            .into_iter()
            .map(|dto| RoleItemData {
               id:          dto.id.to_string(),
               name:        dto.name,
               description: dto.description,
               permissions: dto.permissions,
               is_system:   dto.is_system,
               user_count:  dto.user_count,
            })
            .collect();
         let response = ApiResponse::new(items);
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(e) => {
         tracing::error!("ロール一覧取得で内部エラー: {}", e);
         internal_error_response()
      }
   }
}

/// GET /api/v1/roles/{role_id}
///
/// ロール詳細を取得する。
pub async fn get_role(
   State(state): State<Arc<RoleState>>,
   headers: HeaderMap,
   jar: CookieJar,
   Path(role_id): Path<Uuid>,
) -> impl IntoResponse {
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   let _session_data = match get_session(state.session_manager.as_ref(), &jar, tenant_id).await {
      Ok(data) => data,
      Err(response) => return response,
   };

   match state.core_service_client.get_role(role_id).await {
      Ok(core_response) => {
         let dto = core_response.data;
         let response = ApiResponse::new(RoleDetailData {
            id:          dto.id.to_string(),
            name:        dto.name,
            description: dto.description,
            permissions: dto.permissions,
            is_system:   dto.is_system,
            created_at:  dto.created_at,
            updated_at:  dto.updated_at,
         });
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(CoreServiceError::RoleNotFound) => {
         not_found_response("role-not-found", "Role Not Found", "ロールが見つかりません")
      }
      Err(e) => {
         tracing::error!("ロール詳細取得で内部エラー: {}", e);
         internal_error_response()
      }
   }
}

/// POST /api/v1/roles
///
/// カスタムロールを作成する。
pub async fn create_role(
   State(state): State<Arc<RoleState>>,
   headers: HeaderMap,
   jar: CookieJar,
   Json(req): Json<CreateRoleRequest>,
) -> impl IntoResponse {
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   let session_data = match get_session(state.session_manager.as_ref(), &jar, tenant_id).await {
      Ok(data) => data,
      Err(response) => return response,
   };

   let core_request = CreateRoleCoreRequest {
      tenant_id:   *session_data.tenant_id().as_uuid(),
      name:        req.name,
      description: req.description,
      permissions: req.permissions,
   };

   match state.core_service_client.create_role(&core_request).await {
      Ok(core_response) => {
         let dto = core_response.data;
         let response = ApiResponse::new(RoleDetailData {
            id:          dto.id.to_string(),
            name:        dto.name,
            description: dto.description,
            permissions: dto.permissions,
            is_system:   dto.is_system,
            created_at:  dto.created_at,
            updated_at:  dto.updated_at,
         });
         (StatusCode::CREATED, Json(response)).into_response()
      }
      Err(CoreServiceError::ValidationError(msg)) => validation_error_response(&msg),
      Err(CoreServiceError::Conflict(msg)) => conflict_response(&msg),
      Err(e) => {
         tracing::error!("ロール作成で内部エラー: {}", e);
         internal_error_response()
      }
   }
}

/// PATCH /api/v1/roles/{role_id}
///
/// カスタムロールを更新する。
pub async fn update_role(
   State(state): State<Arc<RoleState>>,
   headers: HeaderMap,
   jar: CookieJar,
   Path(role_id): Path<Uuid>,
   Json(req): Json<UpdateRoleRequest>,
) -> impl IntoResponse {
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   let _session_data = match get_session(state.session_manager.as_ref(), &jar, tenant_id).await {
      Ok(data) => data,
      Err(response) => return response,
   };

   let core_request = UpdateRoleCoreRequest {
      name:        req.name,
      description: req.description,
      permissions: req.permissions,
   };

   match state
      .core_service_client
      .update_role(role_id, &core_request)
      .await
   {
      Ok(core_response) => {
         let dto = core_response.data;
         let response = ApiResponse::new(RoleDetailData {
            id:          dto.id.to_string(),
            name:        dto.name,
            description: dto.description,
            permissions: dto.permissions,
            is_system:   dto.is_system,
            created_at:  dto.created_at,
            updated_at:  dto.updated_at,
         });
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(CoreServiceError::RoleNotFound) => {
         not_found_response("role-not-found", "Role Not Found", "ロールが見つかりません")
      }
      Err(CoreServiceError::ValidationError(msg)) => validation_error_response(&msg),
      Err(e) => {
         tracing::error!("ロール更新で内部エラー: {}", e);
         internal_error_response()
      }
   }
}

/// DELETE /api/v1/roles/{role_id}
///
/// カスタムロールを削除する。
pub async fn delete_role(
   State(state): State<Arc<RoleState>>,
   headers: HeaderMap,
   jar: CookieJar,
   Path(role_id): Path<Uuid>,
) -> impl IntoResponse {
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   let _session_data = match get_session(state.session_manager.as_ref(), &jar, tenant_id).await {
      Ok(data) => data,
      Err(response) => return response,
   };

   match state.core_service_client.delete_role(role_id).await {
      Ok(()) => StatusCode::NO_CONTENT.into_response(),
      Err(CoreServiceError::RoleNotFound) => {
         not_found_response("role-not-found", "Role Not Found", "ロールが見つかりません")
      }
      Err(CoreServiceError::ValidationError(msg)) => validation_error_response(&msg),
      Err(CoreServiceError::Conflict(msg)) => conflict_response(&msg),
      Err(e) => {
         tracing::error!("ロール削除で内部エラー: {}", e);
         internal_error_response()
      }
   }
}
