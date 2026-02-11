//! # ロールハンドラ
//!
//! Core API のロール管理内部 API を提供する。
//!
//! ## エンドポイント
//!
//! - `GET /internal/roles` - テナントのロール一覧（ユーザー数付き）
//! - `GET /internal/roles/{role_id}` - ロール詳細
//! - `POST /internal/roles` - カスタムロール作成
//! - `PATCH /internal/roles/{role_id}` - カスタムロール更新
//! - `DELETE /internal/roles/{role_id}` - カスタムロール削除

use std::sync::Arc;

use axum::{
   Json,
   extract::{Path, Query, State},
   http::StatusCode,
   response::IntoResponse,
};
use ringiflow_domain::{role::RoleId, tenant::TenantId};
use ringiflow_infra::repository::RoleRepository;
use ringiflow_shared::ApiResponse;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
   error::CoreError,
   usecase::role::{CreateRoleInput, RoleUseCaseImpl, UpdateRoleInput},
};

/// ロール API の共有状態
pub struct RoleState {
   pub role_repository: Arc<dyn RoleRepository>,
   pub usecase:         RoleUseCaseImpl,
}

// --- リクエスト/レスポンス型 ---

/// テナント ID クエリパラメータ
#[derive(Debug, Deserialize)]
pub struct RoleTenantQuery {
   pub tenant_id: Uuid,
}

/// ロール一覧の要素 DTO
#[derive(Debug, Serialize)]
pub struct RoleItemDto {
   pub id:          Uuid,
   pub name:        String,
   pub description: Option<String>,
   pub permissions: Vec<String>,
   pub is_system:   bool,
   pub user_count:  i64,
}

/// ロール詳細 DTO
#[derive(Debug, Serialize)]
pub struct RoleDetailDto {
   pub id:          Uuid,
   pub name:        String,
   pub description: Option<String>,
   pub permissions: Vec<String>,
   pub is_system:   bool,
   pub created_at:  String,
   pub updated_at:  String,
}

/// ロール作成リクエスト
#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
   pub tenant_id:   Uuid,
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

// --- ハンドラ ---

/// GET /internal/roles
///
/// テナントで利用可能なロール一覧をユーザー数付きで取得する。
/// system_admin は除外される。
pub async fn list_roles(
   State(state): State<Arc<RoleState>>,
   Query(query): Query<RoleTenantQuery>,
) -> Result<impl IntoResponse, CoreError> {
   let tenant_id = TenantId::from_uuid(query.tenant_id);

   let roles_with_counts = state
      .role_repository
      .find_all_by_tenant_with_user_count(&tenant_id)
      .await?;

   let items: Vec<RoleItemDto> = roles_with_counts
      .into_iter()
      .map(|(role, user_count)| RoleItemDto {
         id: *role.id().as_uuid(),
         name: role.name().to_string(),
         description: role.description().map(|s| s.to_string()),
         permissions: role.permissions().iter().map(|p| p.to_string()).collect(),
         is_system: role.is_system(),
         user_count,
      })
      .collect();

   let response = ApiResponse::new(items);
   Ok((StatusCode::OK, Json(response)))
}

/// GET /internal/roles/{role_id}
///
/// ロール詳細を取得する。
/// テナント分離: システムロールは全テナントからアクセス可能、
/// テナントロールは所属テナントのみアクセス可能。
pub async fn get_role(
   State(state): State<Arc<RoleState>>,
   Path(role_id): Path<Uuid>,
   Query(query): Query<RoleTenantQuery>,
) -> Result<impl IntoResponse, CoreError> {
   let role_id = RoleId::from_uuid(role_id);
   let tenant_id = TenantId::from_uuid(query.tenant_id);

   let role = state
      .role_repository
      .find_by_id(&role_id)
      .await?
      .ok_or_else(|| CoreError::NotFound("ロールが見つかりません".to_string()))?;

   // テナント分離: テナントロールは所属テナントのみアクセス可能
   if !role.is_system() && role.tenant_id() != Some(&tenant_id) {
      return Err(CoreError::NotFound("ロールが見つかりません".to_string()));
   }

   let response = ApiResponse::new(RoleDetailDto {
      id:          *role.id().as_uuid(),
      name:        role.name().to_string(),
      description: role.description().map(|s| s.to_string()),
      permissions: role.permissions().iter().map(|p| p.to_string()).collect(),
      is_system:   role.is_system(),
      created_at:  role.created_at().to_rfc3339(),
      updated_at:  role.updated_at().to_rfc3339(),
   });

   Ok((StatusCode::OK, Json(response)))
}

/// POST /internal/roles
///
/// カスタムロールを作成する。
///
/// ## レスポンス
///
/// - `201 Created`: 作成されたロール
/// - `400 Bad Request`: 権限が空
/// - `409 Conflict`: 同名ロール重複
pub async fn create_role(
   State(state): State<Arc<RoleState>>,
   Json(req): Json<CreateRoleRequest>,
) -> Result<impl IntoResponse, CoreError> {
   let input = CreateRoleInput {
      tenant_id:   TenantId::from_uuid(req.tenant_id),
      name:        req.name,
      description: req.description,
      permissions: req.permissions,
   };

   let role = state.usecase.create_role(input).await?;

   let response = ApiResponse::new(RoleDetailDto {
      id:          *role.id().as_uuid(),
      name:        role.name().to_string(),
      description: role.description().map(|s| s.to_string()),
      permissions: role.permissions().iter().map(|p| p.to_string()).collect(),
      is_system:   role.is_system(),
      created_at:  role.created_at().to_rfc3339(),
      updated_at:  role.updated_at().to_rfc3339(),
   });

   Ok((StatusCode::CREATED, Json(response)))
}

/// PATCH /internal/roles/{role_id}
///
/// カスタムロールを更新する。
///
/// ## レスポンス
///
/// - `200 OK`: 更新後のロール
/// - `400 Bad Request`: システムロール、権限空
/// - `404 Not Found`: ロールが見つからない
pub async fn update_role(
   State(state): State<Arc<RoleState>>,
   Path(role_id): Path<Uuid>,
   Json(req): Json<UpdateRoleRequest>,
) -> Result<impl IntoResponse, CoreError> {
   let input = UpdateRoleInput {
      role_id:     RoleId::from_uuid(role_id),
      name:        req.name,
      description: req.description,
      permissions: req.permissions,
   };

   let role = state.usecase.update_role(input).await?;

   let response = ApiResponse::new(RoleDetailDto {
      id:          *role.id().as_uuid(),
      name:        role.name().to_string(),
      description: role.description().map(|s| s.to_string()),
      permissions: role.permissions().iter().map(|p| p.to_string()).collect(),
      is_system:   role.is_system(),
      created_at:  role.created_at().to_rfc3339(),
      updated_at:  role.updated_at().to_rfc3339(),
   });

   Ok((StatusCode::OK, Json(response)))
}

/// DELETE /internal/roles/{role_id}
///
/// カスタムロールを削除する。
///
/// ## レスポンス
///
/// - `204 No Content`: 削除成功
/// - `400 Bad Request`: システムロール
/// - `404 Not Found`: ロールが見つからない
/// - `409 Conflict`: ユーザー割り当てあり
pub async fn delete_role(
   State(state): State<Arc<RoleState>>,
   Path(role_id): Path<Uuid>,
) -> Result<impl IntoResponse, CoreError> {
   let role_id = RoleId::from_uuid(role_id);

   state.usecase.delete_role(&role_id).await?;

   Ok(StatusCode::NO_CONTENT)
}
