//! # BFF エラーハンドリング
//!
//! HTTP API のエラー定義と、axum レスポンスへの変換。
//!
//! BFF の各ハンドラが共通で使うエラー型とヘルパー関数を集約する。

use axum::{
   Json,
   http::{HeaderMap, StatusCode},
   response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use ringiflow_domain::tenant::TenantId;
use ringiflow_infra::SessionManager;
use ringiflow_shared::ErrorResponse;
use uuid::Uuid;

/// Cookie 名
const SESSION_COOKIE_NAME: &str = "session_id";

// --- エラー型 ---

/// テナント ID 抽出エラー
#[derive(Debug)]
pub enum TenantIdError {
   /// ヘッダーが存在しない
   Missing,
   /// UUID の形式が不正
   InvalidFormat,
}

impl IntoResponse for TenantIdError {
   fn into_response(self) -> Response {
      let detail = match self {
         TenantIdError::Missing => "X-Tenant-ID ヘッダーが必要です",
         TenantIdError::InvalidFormat => "X-Tenant-ID の形式が不正です",
      };
      (
         StatusCode::BAD_REQUEST,
         Json(ErrorResponse::validation_error(detail)),
      )
         .into_response()
   }
}

// --- 共通ヘルパー関数 ---

/// X-Tenant-ID ヘッダーからテナント ID を抽出する
pub fn extract_tenant_id(headers: &HeaderMap) -> Result<Uuid, TenantIdError> {
   let tenant_id_str = headers
      .get("X-Tenant-ID")
      .and_then(|v| v.to_str().ok())
      .ok_or(TenantIdError::Missing)?;

   Uuid::parse_str(tenant_id_str).map_err(|_| TenantIdError::InvalidFormat)
}

/// セッションを取得する
pub async fn get_session(
   session_manager: &dyn SessionManager,
   jar: &CookieJar,
   tenant_id: Uuid,
) -> Result<ringiflow_infra::SessionData, Response> {
   // Cookie からセッション ID を取得
   let session_id = jar
      .get(SESSION_COOKIE_NAME)
      .map(|cookie| cookie.value().to_string())
      .ok_or_else(unauthorized_response)?;

   let tenant_id = TenantId::from_uuid(tenant_id);

   // セッションを取得
   match session_manager.get(&tenant_id, &session_id).await {
      Ok(Some(data)) => Ok(data),
      Ok(None) => Err(unauthorized_response()),
      Err(e) => {
         tracing::error!("セッション取得で内部エラー: {}", e);
         Err(internal_error_response())
      }
   }
}

// --- レスポンスヘルパー ---

/// 認証失敗レスポンス
pub fn authentication_failed_response() -> Response {
   (
      StatusCode::UNAUTHORIZED,
      Json(ErrorResponse::new(
         "authentication-failed",
         "Authentication Failed",
         401,
         "メールアドレスまたはパスワードが正しくありません",
      )),
   )
      .into_response()
}

/// 未認証レスポンス
pub fn unauthorized_response() -> Response {
   (
      StatusCode::UNAUTHORIZED,
      Json(ErrorResponse::unauthorized("認証が必要です")),
   )
      .into_response()
}

/// 内部エラーレスポンス
pub fn internal_error_response() -> Response {
   (
      StatusCode::INTERNAL_SERVER_ERROR,
      Json(ErrorResponse::internal_error()),
   )
      .into_response()
}

/// Auth Service 利用不可レスポンス
pub fn service_unavailable_response() -> Response {
   (
      StatusCode::SERVICE_UNAVAILABLE,
      Json(ErrorResponse::service_unavailable(
         "認証サービスが一時的に利用できません",
      )),
   )
      .into_response()
}

/// 404 Not Found レスポンス
pub fn not_found_response(error_type_suffix: &str, title: &str, detail: &str) -> Response {
   (
      StatusCode::NOT_FOUND,
      Json(ErrorResponse::new(error_type_suffix, title, 404, detail)),
   )
      .into_response()
}

/// バリデーションエラーレスポンス
pub fn validation_error_response(detail: &str) -> Response {
   (
      StatusCode::BAD_REQUEST,
      Json(ErrorResponse::validation_error(detail)),
   )
      .into_response()
}

/// 403 Forbidden レスポンス
pub fn forbidden_response(detail: &str) -> Response {
   (
      StatusCode::FORBIDDEN,
      Json(ErrorResponse::forbidden(detail)),
   )
      .into_response()
}

/// 409 Conflict レスポンス
pub fn conflict_response(detail: &str) -> Response {
   (StatusCode::CONFLICT, Json(ErrorResponse::conflict(detail))).into_response()
}
