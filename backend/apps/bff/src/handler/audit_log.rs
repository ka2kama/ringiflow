//! # 監査ログ閲覧 API ハンドラ
//!
//! BFF の監査ログ閲覧エンドポイントを提供する。
//!
//! ## エンドポイント
//!
//! - `GET /api/v1/audit-logs` - テナント内の監査ログ一覧（カーソルベースページネーション）

use std::sync::Arc;

use axum::{
   Json,
   extract::{Query, State},
   http::{HeaderMap, StatusCode},
   response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use chrono::DateTime;
use ringiflow_domain::{
   audit_log::{AuditAction, AuditResult},
   user::UserId,
};
use ringiflow_infra::{
   SessionManager,
   repository::audit_log_repository::{AuditLogFilter, AuditLogRepository},
};
use ringiflow_shared::{ErrorResponse, PaginatedResponse};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::error::{extract_tenant_id, get_session, internal_error_response};

/// 監査ログ閲覧 API の共有状態
pub struct AuditLogState {
   pub audit_log_repository: Arc<dyn AuditLogRepository>,
   pub session_manager:      Arc<dyn SessionManager>,
}

/// 監査ログ一覧クエリパラメータ
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListAuditLogsQuery {
   /// カーソル（次ページ取得用、opaque 文字列）
   pub cursor:   Option<String>,
   /// 取得件数（デフォルト 50、最大 100）
   pub limit:    Option<i32>,
   /// 開始日時（ISO 8601）
   pub from:     Option<String>,
   /// 終了日時（ISO 8601）
   pub to:       Option<String>,
   /// 操作者 ID でフィルタ
   pub actor_id: Option<Uuid>,
   /// アクションでフィルタ（カンマ区切りで複数指定可）
   pub action:   Option<String>,
   /// 結果でフィルタ（success / failure）
   pub result:   Option<String>,
}

/// 監査ログ一覧の要素データ
#[derive(Debug, Serialize, ToSchema)]
pub struct AuditLogItemData {
   pub id: String,
   pub actor_id: String,
   pub actor_name: String,
   pub action: String,
   pub result: String,
   pub resource_type: String,
   pub resource_id: String,
   pub detail: Option<serde_json::Value>,
   pub source_ip: Option<String>,
   pub created_at: String,
}

/// GET /api/v1/audit-logs
///
/// テナント内の監査ログ一覧を取得する（新しい順）。
/// カーソルベースページネーション対応。
#[utoipa::path(
   get,
   path = "/api/v1/audit-logs",
   tag = "audit-logs",
   security(("session_auth" = [])),
   params(ListAuditLogsQuery),
   responses(
      (status = 200, description = "監査ログ一覧", body = PaginatedResponse<AuditLogItemData>),
      (status = 401, description = "認証エラー", body = ErrorResponse)
   )
)]
pub async fn list_audit_logs(
   State(state): State<Arc<AuditLogState>>,
   headers: HeaderMap,
   jar: CookieJar,
   Query(query): Query<ListAuditLogsQuery>,
) -> impl IntoResponse {
   let tenant_id = match extract_tenant_id(&headers) {
      Ok(id) => id,
      Err(e) => return e.into_response(),
   };

   let session_data = match get_session(state.session_manager.as_ref(), &jar, tenant_id).await {
      Ok(data) => data,
      Err(response) => return response,
   };

   // limit のバリデーション（デフォルト 50、最大 100）
   let limit = query.limit.unwrap_or(50).clamp(1, 100);

   // フィルタの構築
   let from = query
      .from
      .as_deref()
      .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
      .map(|dt| dt.with_timezone(&chrono::Utc));

   let to = query
      .to
      .as_deref()
      .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
      .map(|dt| dt.with_timezone(&chrono::Utc));

   let actor_id = query.actor_id.map(UserId::from_uuid);

   let actions: Option<Vec<AuditAction>> = query
      .action
      .as_deref()
      .map(|s| s.split(',').filter_map(|a| a.trim().parse().ok()).collect());

   let result: Option<AuditResult> = query.result.as_deref().and_then(|s| s.parse().ok());

   let filter = AuditLogFilter {
      from,
      to,
      actor_id,
      actions,
      result,
   };

   match state
      .audit_log_repository
      .find_by_tenant(
         session_data.tenant_id(),
         query.cursor.as_deref(),
         limit,
         &filter,
      )
      .await
   {
      Ok(page) => {
         let items: Vec<AuditLogItemData> = page
            .items
            .into_iter()
            .map(|log| AuditLogItemData {
               id: log.id.to_string(),
               actor_id: log.actor_id.as_uuid().to_string(),
               actor_name: log.actor_name,
               action: log.action.to_string(),
               result: log.result.to_string(),
               resource_type: log.resource_type,
               resource_id: log.resource_id,
               detail: log.detail,
               source_ip: log.source_ip,
               created_at: log.created_at.to_rfc3339(),
            })
            .collect();

         let response = PaginatedResponse {
            data:        items,
            next_cursor: page.next_cursor,
         };
         (StatusCode::OK, Json(response)).into_response()
      }
      Err(e) => {
         tracing::error!("監査ログの検索に失敗: {}", e);
         internal_error_response()
      }
   }
}
