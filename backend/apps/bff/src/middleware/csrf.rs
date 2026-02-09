//! # CSRF 検証ミドルウェア
//!
//! 状態変更リクエスト（POST/PUT/PATCH/DELETE）で CSRF トークンを検証する。
//!
//! 詳細: [07_認証機能設計.md](../../../../docs/03_詳細設計書/07_認証機能設計.md)

use std::sync::Arc;

use axum::{
   Json,
   body::Body,
   extract::State,
   http::{Method, Request, StatusCode},
   middleware::Next,
   response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use ringiflow_domain::tenant::TenantId;
use ringiflow_infra::SessionManager;
use ringiflow_shared::ErrorResponse;
use subtle::ConstantTimeEq;
use uuid::Uuid;

/// CSRF 検証用のヘッダー名
const CSRF_HEADER: &str = "X-CSRF-Token";

/// セッション Cookie 名
const SESSION_COOKIE_NAME: &str = "session_id";

/// CSRF 検証をスキップするパス
const CSRF_SKIP_PATHS: &[&str] = &["/api/v1/auth/login", "/api/v1/auth/csrf", "/health"];

/// CSRF 検証の状態
#[derive(Clone)]
pub struct CsrfState {
   pub session_manager: Arc<dyn SessionManager>,
}

fn csrf_error_response(detail: &str) -> Response {
   (
      StatusCode::FORBIDDEN,
      Json(ErrorResponse::new(
         "csrf-validation-failed",
         "CSRF Validation Failed",
         403,
         detail,
      )),
   )
      .into_response()
}

/// CSRF 検証が必要なメソッドかどうか
fn requires_csrf_validation(method: &Method) -> bool {
   matches!(
      *method,
      Method::POST | Method::PUT | Method::PATCH | Method::DELETE
   )
}

/// CSRF 検証をスキップするパスかどうか
fn should_skip_csrf(path: &str) -> bool {
   CSRF_SKIP_PATHS.contains(&path)
}

/// CSRF 検証ミドルウェア
pub async fn csrf_middleware(
   State(state): State<CsrfState>,
   jar: CookieJar,
   request: Request<Body>,
   next: Next,
) -> Response {
   let method = request.method().clone();
   let path = request.uri().path().to_string();

   // CSRF 検証が不要な場合はスキップ
   if !requires_csrf_validation(&method) || should_skip_csrf(&path) {
      return next.run(request).await;
   }

   // X-Tenant-ID ヘッダーを取得
   let tenant_id = match request
      .headers()
      .get("X-Tenant-ID")
      .and_then(|v| v.to_str().ok())
      .and_then(|s| Uuid::parse_str(s).ok())
   {
      Some(id) => TenantId::from_uuid(id),
      None => return csrf_error_response("テナント ID が必要です"),
   };

   // Cookie からセッション ID を取得
   let session_id = match jar.get(SESSION_COOKIE_NAME) {
      Some(cookie) => cookie.value().to_string(),
      None => return csrf_error_response("セッションが必要です"),
   };

   // X-CSRF-Token ヘッダーを取得
   let provided_token = match request
      .headers()
      .get(CSRF_HEADER)
      .and_then(|v| v.to_str().ok())
   {
      Some(token) => token.to_string(),
      None => return csrf_error_response("CSRF トークンが必要です"),
   };

   // Redis から CSRF トークンを取得して検証
   // タイミング攻撃対策として定数時間比較を使用
   match state
      .session_manager
      .get_csrf_token(&tenant_id, &session_id)
      .await
   {
      Ok(Some(stored_token)) => {
         let is_valid: bool = stored_token
            .as_bytes()
            .ct_eq(provided_token.as_bytes())
            .into();
         if !is_valid {
            return csrf_error_response("CSRF トークンが無効です");
         }
      }
      Ok(None) => return csrf_error_response("CSRF トークンが無効です"),
      Err(e) => {
         tracing::error!("CSRF トークン取得で内部エラー: {}", e);
         return csrf_error_response("内部エラーが発生しました");
      }
   }

   next.run(request).await
}
