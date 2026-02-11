//! # ユーザー管理 API ハンドラ
//!
//! BFF のユーザー管理エンドポイントを提供する。
//!
//! ## エンドポイント
//!
//! - `GET /api/v1/users` - テナント内のユーザー一覧
//! - `POST /api/v1/users` - ユーザー作成
//! - `GET /api/v1/users/{display_number}` - ユーザー詳細
//! - `PATCH /api/v1/users/{display_number}` - ユーザー更新
//! - `PATCH /api/v1/users/{display_number}/status` - ステータス変更

use std::sync::Arc;

use axum::{
   Json,
   extract::{Path, Query, State},
   http::{HeaderMap, StatusCode},
   response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use ringiflow_domain::audit_log::{AuditAction, AuditLog};
use ringiflow_infra::{SessionManager, repository::AuditLogRepository};
use ringiflow_shared::ApiResponse;
use serde::{Deserialize, Serialize};

use crate::{
   client::{
      AuthServiceClient,
      CoreServiceError,
      CoreServiceUserClient,
      CreateUserCoreRequest,
      UpdateUserCoreRequest,
      UpdateUserStatusCoreRequest,
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

/// ユーザー管理 API の共有状態
pub struct UserState {
   pub core_service_client:  Arc<dyn CoreServiceUserClient>,
   pub auth_service_client:  Arc<dyn AuthServiceClient>,
   pub session_manager:      Arc<dyn SessionManager>,
   pub audit_log_repository: Arc<dyn AuditLogRepository>,
}

// --- リクエスト型 ---

/// ユーザー一覧クエリパラメータ
#[derive(Debug, Deserialize)]
pub struct ListUsersQuery {
   pub status: Option<String>,
}

/// ユーザー作成リクエスト
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
   pub email:     String,
   pub name:      String,
   pub role_name: String,
}

/// ユーザー更新リクエスト
#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
   pub name:      Option<String>,
   pub role_name: Option<String>,
}

/// ユーザーステータス変更リクエスト
#[derive(Debug, Deserialize)]
pub struct UpdateUserStatusRequest {
   pub status: String,
}

// --- レスポンス型 ---

/// ユーザー一覧の要素データ
#[derive(Debug, Serialize)]
pub struct UserItemData {
   pub id: String,
   pub display_id: String,
   pub display_number: i64,
   pub name: String,
   pub email: String,
   pub status: String,
   pub roles: Vec<String>,
}

impl From<crate::client::UserItemDto> for UserItemData {
   fn from(dto: crate::client::UserItemDto) -> Self {
      Self {
         id: dto.id.to_string(),
         display_id: dto.display_id,
         display_number: dto.display_number,
         name: dto.name,
         email: dto.email,
         status: dto.status,
         roles: dto.roles,
      }
   }
}

/// ユーザー作成レスポンス（初期パスワード付き）
#[derive(Debug, Serialize)]
pub struct CreateUserResponseData {
   pub id: String,
   pub display_id: String,
   pub display_number: i64,
   pub name: String,
   pub email: String,
   pub role: String,
   pub initial_password: String,
}

/// ユーザー詳細レスポンス
#[derive(Debug, Serialize)]
pub struct UserDetailData {
   pub id: String,
   pub display_id: String,
   pub display_number: i64,
   pub name: String,
   pub email: String,
   pub status: String,
   pub roles: Vec<String>,
   pub permissions: Vec<String>,
   pub tenant_name: String,
}

/// ユーザー簡易レスポンス（更新・ステータス変更用）
#[derive(Debug, Serialize)]
pub struct UserResponseData {
   pub id:     String,
   pub name:   String,
   pub email:  String,
   pub status: String,
}

// --- ヘルパー関数 ---

/// 初期パスワードを生成する（16文字、英数字 + 特殊文字）
fn generate_initial_password() -> String {
   use rand::Rng;
   const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%&*";
   let mut rng = rand::rng();
   (0..16)
      .map(|_| {
         let idx = rng.random_range(0..CHARSET.len());
         CHARSET[idx] as char
      })
      .collect()
}

// --- ハンドラ ---

/// GET /api/v1/users
///
/// テナント内のユーザー一覧を取得する。
/// ステータスフィルタに対応（省略時は deleted 以外すべて）。
pub async fn list_users(
   State(state): State<Arc<UserState>>,
   headers: HeaderMap,
   jar: CookieJar,
   Query(query): Query<ListUsersQuery>,
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
      .list_users(*session_data.tenant_id().as_uuid(), query.status.as_deref())
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

/// POST /api/v1/users
///
/// ユーザーを作成する。
///
/// ## フロー
///
/// 1. 入力バリデーション
/// 2. 初期パスワード生成
/// 3. Core Service でユーザー作成
/// 4. Auth Service で認証情報作成
/// 5. 初期パスワード付きレスポンス返却
pub async fn create_user(
   State(state): State<Arc<UserState>>,
   headers: HeaderMap,
   jar: CookieJar,
   Json(req): Json<CreateUserRequest>,
) -> impl IntoResponse {
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   let session_data = match get_session(state.session_manager.as_ref(), &jar, tenant_id).await {
      Ok(data) => data,
      Err(response) => return response,
   };

   // 初期パスワード生成
   let initial_password = generate_initial_password();

   // Core Service でユーザー作成
   let core_request = CreateUserCoreRequest {
      tenant_id: *session_data.tenant_id().as_uuid(),
      email:     req.email,
      name:      req.name,
      role_name: req.role_name,
   };

   let core_response = match state.core_service_client.create_user(&core_request).await {
      Ok(response) => response,
      Err(CoreServiceError::ValidationError(msg)) => {
         return validation_error_response(&msg);
      }
      Err(CoreServiceError::EmailAlreadyExists) => {
         return conflict_response("このメールアドレスは既に使用されています");
      }
      Err(CoreServiceError::Conflict(_)) => {
         return conflict_response("このメールアドレスは既に使用されています");
      }
      Err(e) => {
         tracing::error!("ユーザー作成で内部エラー: {}", e);
         return internal_error_response();
      }
   };

   let user_data = core_response.data;

   // Auth Service で認証情報作成
   if let Err(e) = state
      .auth_service_client
      .create_credentials(
         *session_data.tenant_id().as_uuid(),
         user_data.id,
         "password",
         &initial_password,
      )
      .await
   {
      tracing::error!("認証情報の作成に失敗しました（ユーザーは作成済み）: {}", e);
      return internal_error_response();
   }

   // 監査ログ記録
   let audit_log = AuditLog::new_success(
      session_data.tenant_id().clone(),
      session_data.user_id().clone(),
      session_data.name().to_string(),
      AuditAction::UserCreate,
      "user",
      user_data.id.to_string(),
      Some(serde_json::json!({
         "email": &user_data.email,
         "name": &user_data.name,
         "role": &user_data.role,
      })),
      None,
   );
   if let Err(e) = state.audit_log_repository.record(&audit_log).await {
      tracing::error!("監査ログ記録に失敗: {}", e);
   }

   let response = ApiResponse::new(CreateUserResponseData {
      id: user_data.id.to_string(),
      display_id: user_data.display_id,
      display_number: user_data.display_number,
      name: user_data.name,
      email: user_data.email,
      role: user_data.role,
      initial_password,
   });

   (StatusCode::CREATED, Json(response)).into_response()
}

/// GET /api/v1/users/{display_number}
///
/// 表示用連番でユーザー詳細を取得する。
pub async fn get_user_detail(
   State(state): State<Arc<UserState>>,
   headers: HeaderMap,
   jar: CookieJar,
   Path(display_number): Path<i64>,
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
      .get_user_by_display_number(*session_data.tenant_id().as_uuid(), display_number)
      .await
   {
      Ok(core_response) => {
         let data = core_response.data;
         let response = ApiResponse::new(UserDetailData {
            id: data.user.id.to_string(),
            display_id: format!("USR-{:06}", display_number),
            display_number,
            name: data.user.name,
            email: data.user.email,
            status: data.user.status,
            roles: data.roles,
            permissions: data.permissions,
            tenant_name: data.tenant_name,
         });
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(CoreServiceError::UserNotFound) => not_found_response(
         "user-not-found",
         "User Not Found",
         "ユーザーが見つかりません",
      ),
      Err(e) => {
         tracing::error!("ユーザー詳細取得で内部エラー: {}", e);
         internal_error_response()
      }
   }
}

/// PATCH /api/v1/users/{display_number}
///
/// ユーザー情報を更新する（名前、ロール）。
pub async fn update_user(
   State(state): State<Arc<UserState>>,
   headers: HeaderMap,
   jar: CookieJar,
   Path(display_number): Path<i64>,
   Json(req): Json<UpdateUserRequest>,
) -> impl IntoResponse {
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   let session_data = match get_session(state.session_manager.as_ref(), &jar, tenant_id).await {
      Ok(data) => data,
      Err(response) => return response,
   };

   // display_number でユーザーを取得して UUID を解決する
   let user_data = match state
      .core_service_client
      .get_user_by_display_number(*session_data.tenant_id().as_uuid(), display_number)
      .await
   {
      Ok(response) => response.data,
      Err(CoreServiceError::UserNotFound) => {
         return not_found_response(
            "user-not-found",
            "User Not Found",
            "ユーザーが見つかりません",
         );
      }
      Err(e) => {
         tracing::error!("ユーザー取得で内部エラー: {}", e);
         return internal_error_response();
      }
   };

   let core_request = UpdateUserCoreRequest {
      name:      req.name,
      role_name: req.role_name,
   };

   match state
      .core_service_client
      .update_user(user_data.user.id, &core_request)
      .await
   {
      Ok(core_response) => {
         let user = core_response.data;

         // 監査ログ記録
         let audit_log = AuditLog::new_success(
            session_data.tenant_id().clone(),
            session_data.user_id().clone(),
            session_data.name().to_string(),
            AuditAction::UserUpdate,
            "user",
            user.id.to_string(),
            Some(serde_json::json!({
               "name": &user.name,
            })),
            None,
         );
         if let Err(e) = state.audit_log_repository.record(&audit_log).await {
            tracing::error!("監査ログ記録に失敗: {}", e);
         }

         let response = ApiResponse::new(UserResponseData {
            id:     user.id.to_string(),
            name:   user.name,
            email:  user.email,
            status: user.status,
         });
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(CoreServiceError::UserNotFound) => not_found_response(
         "user-not-found",
         "User Not Found",
         "ユーザーが見つかりません",
      ),
      Err(CoreServiceError::ValidationError(msg)) => validation_error_response(&msg),
      Err(e) => {
         tracing::error!("ユーザー更新で内部エラー: {}", e);
         internal_error_response()
      }
   }
}

/// PATCH /api/v1/users/{display_number}/status
///
/// ユーザーステータスを変更する。
/// セッションから自身の user_id を取得し、Core Service に渡す。
pub async fn update_user_status(
   State(state): State<Arc<UserState>>,
   headers: HeaderMap,
   jar: CookieJar,
   Path(display_number): Path<i64>,
   Json(req): Json<UpdateUserStatusRequest>,
) -> impl IntoResponse {
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   let session_data = match get_session(state.session_manager.as_ref(), &jar, tenant_id).await {
      Ok(data) => data,
      Err(response) => return response,
   };

   // display_number でユーザーを取得して UUID を解決する
   let user_data = match state
      .core_service_client
      .get_user_by_display_number(*session_data.tenant_id().as_uuid(), display_number)
      .await
   {
      Ok(response) => response.data,
      Err(CoreServiceError::UserNotFound) => {
         return not_found_response(
            "user-not-found",
            "User Not Found",
            "ユーザーが見つかりません",
         );
      }
      Err(e) => {
         tracing::error!("ユーザー取得で内部エラー: {}", e);
         return internal_error_response();
      }
   };

   let core_request = UpdateUserStatusCoreRequest {
      status:       req.status,
      tenant_id:    *session_data.tenant_id().as_uuid(),
      requester_id: *session_data.user_id().as_uuid(),
   };

   match state
      .core_service_client
      .update_user_status(user_data.user.id, &core_request)
      .await
   {
      Ok(core_response) => {
         let user = core_response.data;

         // 監査ログ記録: ステータスに応じて Deactivate / Activate を使い分ける
         let action = if user.status == "inactive" {
            AuditAction::UserDeactivate
         } else {
            AuditAction::UserActivate
         };
         let audit_log = AuditLog::new_success(
            session_data.tenant_id().clone(),
            session_data.user_id().clone(),
            session_data.name().to_string(),
            action,
            "user",
            user.id.to_string(),
            Some(serde_json::json!({
               "user_name": &user.name,
            })),
            None,
         );
         if let Err(e) = state.audit_log_repository.record(&audit_log).await {
            tracing::error!("監査ログ記録に失敗: {}", e);
         }

         let response = ApiResponse::new(UserResponseData {
            id:     user.id.to_string(),
            name:   user.name,
            email:  user.email,
            status: user.status,
         });
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(CoreServiceError::UserNotFound) => not_found_response(
         "user-not-found",
         "User Not Found",
         "ユーザーが見つかりません",
      ),
      Err(CoreServiceError::ValidationError(msg)) => validation_error_response(&msg),
      Err(e) => {
         tracing::error!("ユーザーステータス変更で内部エラー: {}", e);
         internal_error_response()
      }
   }
}
