//! # 監査ログ
//!
//! ユーザー操作の監査証跡を記録するドメインモデル。
//!
//! ## 設計方針
//!
//! - **不変性**: 監査ログは一度作成されたら変更されない
//! - **テナント分離**: すべての監査ログは `tenant_id` をキーとして分離
//! - **TTL**: 作成から1年後に自動削除（DynamoDB TTL）
//!
//! ## アクション体系
//!
//! アクションは `リソース.操作` 形式の文字列に変換される:
//!
//! | バリアント | 文字列表現 |
//! |-----------|-----------|
//! | `UserCreate` | `user.create` |
//! | `UserUpdate` | `user.update` |
//! | `UserDeactivate` | `user.deactivate` |
//! | `UserActivate` | `user.activate` |
//! | `RoleCreate` | `role.create` |
//! | `RoleUpdate` | `role.update` |
//! | `RoleDelete` | `role.delete` |

use std::{fmt, str::FromStr};

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::{tenant::TenantId, user::UserId};

/// 監査対象のアクション
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditAction {
   UserCreate,
   UserUpdate,
   UserDeactivate,
   UserActivate,
   RoleCreate,
   RoleUpdate,
   RoleDelete,
}

impl fmt::Display for AuditAction {
   fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      let s = match self {
         Self::UserCreate => "user.create",
         Self::UserUpdate => "user.update",
         Self::UserDeactivate => "user.deactivate",
         Self::UserActivate => "user.activate",
         Self::RoleCreate => "role.create",
         Self::RoleUpdate => "role.update",
         Self::RoleDelete => "role.delete",
      };
      write!(f, "{s}")
   }
}

impl FromStr for AuditAction {
   type Err = String;

   fn from_str(s: &str) -> Result<Self, Self::Err> {
      match s {
         "user.create" => Ok(Self::UserCreate),
         "user.update" => Ok(Self::UserUpdate),
         "user.deactivate" => Ok(Self::UserDeactivate),
         "user.activate" => Ok(Self::UserActivate),
         "role.create" => Ok(Self::RoleCreate),
         "role.update" => Ok(Self::RoleUpdate),
         "role.delete" => Ok(Self::RoleDelete),
         _ => Err(format!("不明な監査アクション: {s}")),
      }
   }
}

/// 監査ログの結果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditResult {
   Success,
   Failure,
}

impl fmt::Display for AuditResult {
   fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      match self {
         Self::Success => write!(f, "success"),
         Self::Failure => write!(f, "failure"),
      }
   }
}

impl FromStr for AuditResult {
   type Err = String;

   fn from_str(s: &str) -> Result<Self, Self::Err> {
      match s {
         "success" => Ok(Self::Success),
         "failure" => Ok(Self::Failure),
         _ => Err(format!("不明な監査結果: {s}")),
      }
   }
}

/// TTL 期間（1年）
const TTL_DURATION_DAYS: i64 = 365;

/// 監査ログエンティティ
///
/// ユーザー操作の監査証跡を表現する不変のエンティティ。
/// DynamoDB に格納され、テナント管理者が閲覧可能。
#[derive(Debug, Clone)]
pub struct AuditLog {
   pub id: Uuid,
   pub tenant_id: TenantId,
   pub actor_id: UserId,
   pub actor_name: String,
   pub action: AuditAction,
   pub result: AuditResult,
   pub resource_type: String,
   pub resource_id: String,
   pub detail: Option<serde_json::Value>,
   pub source_ip: Option<String>,
   pub created_at: DateTime<Utc>,
   /// TTL（epoch seconds）。created_at + 1年。
   pub ttl: i64,
}

impl AuditLog {
   /// 成功時の監査ログを作成する
   ///
   /// `created_at` は現在時刻、`ttl` は `created_at + 1年` で自動計算される。
   #[allow(clippy::too_many_arguments)]
   pub fn new_success(
      tenant_id: TenantId,
      actor_id: UserId,
      actor_name: String,
      action: AuditAction,
      resource_type: impl Into<String>,
      resource_id: impl Into<String>,
      detail: Option<serde_json::Value>,
      source_ip: Option<String>,
   ) -> Self {
      let now = Utc::now();
      let ttl = (now + Duration::days(TTL_DURATION_DAYS)).timestamp();

      Self {
         id: Uuid::now_v7(),
         tenant_id,
         actor_id,
         actor_name,
         action,
         result: AuditResult::Success,
         resource_type: resource_type.into(),
         resource_id: resource_id.into(),
         detail,
         source_ip,
         created_at: now,
         ttl,
      }
   }

   /// DynamoDB の Sort Key を生成する
   ///
   /// 形式: `{ISO8601_timestamp}#{uuid}`
   ///
   /// - ISO 8601 はレキシカル順でソート可能 → 時系列クエリに最適
   /// - UUID サフィックスで同一ミリ秒のエントリも一意性を保証
   pub fn sort_key(&self) -> String {
      format!("{}#{}", self.created_at.to_rfc3339(), self.id)
   }

   /// Sort Key とデータからエンティティを復元する（リポジトリ用）
   #[allow(clippy::too_many_arguments)]
   pub fn from_stored(
      tenant_id: TenantId,
      sk: &str,
      actor_id: UserId,
      actor_name: String,
      action: AuditAction,
      result: AuditResult,
      resource_type: String,
      resource_id: String,
      detail: Option<serde_json::Value>,
      source_ip: Option<String>,
      ttl: i64,
   ) -> Result<Self, String> {
      // SK 形式: "{timestamp}#{uuid}"
      let (timestamp_str, id_str) = sk
         .rsplit_once('#')
         .ok_or_else(|| format!("不正な Sort Key 形式: {sk}"))?;

      let created_at = DateTime::parse_from_rfc3339(timestamp_str)
         .map_err(|e| format!("タイムスタンプのパースに失敗: {e}"))?
         .with_timezone(&Utc);

      let id = Uuid::parse_str(id_str).map_err(|e| format!("UUID のパースに失敗: {e}"))?;

      Ok(Self {
         id,
         tenant_id,
         actor_id,
         actor_name,
         action,
         result,
         resource_type,
         resource_id,
         detail,
         source_ip,
         created_at,
         ttl,
      })
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn test_audit_actionの各バリアントがドット区切り文字列に変換される() {
      assert_eq!(AuditAction::UserCreate.to_string(), "user.create");
      assert_eq!(AuditAction::UserUpdate.to_string(), "user.update");
      assert_eq!(AuditAction::UserDeactivate.to_string(), "user.deactivate");
      assert_eq!(AuditAction::UserActivate.to_string(), "user.activate");
      assert_eq!(AuditAction::RoleCreate.to_string(), "role.create");
      assert_eq!(AuditAction::RoleUpdate.to_string(), "role.update");
      assert_eq!(AuditAction::RoleDelete.to_string(), "role.delete");
   }

   #[test]
   fn test_audit_actionが文字列からパースできる() {
      assert_eq!(
         "user.create".parse::<AuditAction>().unwrap(),
         AuditAction::UserCreate
      );
      assert_eq!(
         "role.delete".parse::<AuditAction>().unwrap(),
         AuditAction::RoleDelete
      );
   }

   #[test]
   fn test_audit_actionの不明な文字列はエラーになる() {
      assert!("unknown.action".parse::<AuditAction>().is_err());
   }

   #[test]
   fn test_audit_logのttlがcreated_atから1年後に設定される() {
      let log = AuditLog::new_success(
         TenantId::new(),
         UserId::new(),
         "Test User".to_string(),
         AuditAction::UserCreate,
         "user",
         "user-123",
         None,
         None,
      );

      // TTL は created_at + 365 日のタイムスタンプ
      let expected_ttl = (log.created_at + Duration::days(365)).timestamp();
      assert_eq!(log.ttl, expected_ttl);
   }

   #[test]
   fn test_sort_keyがtimestamp_uuid形式で生成される() {
      let log = AuditLog::new_success(
         TenantId::new(),
         UserId::new(),
         "Test User".to_string(),
         AuditAction::RoleCreate,
         "role",
         "role-456",
         None,
         None,
      );

      let sk = log.sort_key();
      assert!(sk.contains('#'), "Sort Key に '#' が含まれるべき");

      let parts: Vec<&str> = sk.rsplitn(2, '#').collect();
      assert_eq!(parts.len(), 2);

      // UUID 部分がパース可能
      Uuid::parse_str(parts[0]).expect("UUID 部分がパースできるべき");

      // タイムスタンプ部分がパース可能
      DateTime::parse_from_rfc3339(parts[1]).expect("タイムスタンプ部分がパースできるべき");
   }

   #[test]
   fn test_from_storedでsort_keyからエンティティを復元できる() {
      let original = AuditLog::new_success(
         TenantId::new(),
         UserId::new(),
         "Test User".to_string(),
         AuditAction::UserUpdate,
         "user",
         "user-789",
         Some(serde_json::json!({"name": "updated"})),
         None,
      );

      let sk = original.sort_key();

      let restored = AuditLog::from_stored(
         original.tenant_id.clone(),
         &sk,
         original.actor_id.clone(),
         original.actor_name.clone(),
         AuditAction::UserUpdate,
         AuditResult::Success,
         original.resource_type.clone(),
         original.resource_id.clone(),
         original.detail.clone(),
         None,
         original.ttl,
      )
      .expect("復元に成功するべき");

      assert_eq!(restored.id, original.id);
      assert_eq!(restored.created_at, original.created_at);
   }
}
