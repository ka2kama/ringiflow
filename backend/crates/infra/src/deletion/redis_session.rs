//! # RedisSessionDeleter
//!
//! テナントのセッションと CSRF トークンを Redis から削除する。
//!
//! ## キーパターン
//!
//! - `session:{tenant_id}:*` — セッションデータ
//! - `csrf:{tenant_id}:*` — CSRF トークン

use async_trait::async_trait;
use redis::{AsyncCommands, aio::ConnectionManager};
use ringiflow_domain::tenant::TenantId;

use super::{DeletionResult, TenantDeleter};
use crate::error::InfraError;

/// Redis セッション Deleter
pub struct RedisSessionDeleter {
   conn: ConnectionManager,
}

impl RedisSessionDeleter {
   pub fn new(conn: ConnectionManager) -> Self {
      Self { conn }
   }

   /// SCAN でパターンにマッチするキーの数を数える
   async fn scan_count(&self, pattern: &str) -> Result<u64, InfraError> {
      let mut conn = self.conn.clone();
      let mut count: u64 = 0;
      let mut cursor = 0u64;

      loop {
         let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(pattern)
            .arg("COUNT")
            .arg(100)
            .query_async(&mut conn)
            .await?;

         count += keys.len() as u64;

         cursor = next_cursor;
         if cursor == 0 {
            break;
         }
      }

      Ok(count)
   }

   /// SCAN でパターンにマッチするキーを全て削除し、削除件数を返す
   async fn scan_and_delete(&self, pattern: &str) -> Result<u64, InfraError> {
      let mut conn = self.conn.clone();
      let mut deleted: u64 = 0;
      let mut cursor = 0u64;

      loop {
         let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(pattern)
            .arg("COUNT")
            .arg(100)
            .query_async(&mut conn)
            .await?;

         if !keys.is_empty() {
            deleted += keys.len() as u64;
            let _: () = conn.del(&keys).await?;
         }

         cursor = next_cursor;
         if cursor == 0 {
            break;
         }
      }

      Ok(deleted)
   }
}

#[async_trait]
impl TenantDeleter for RedisSessionDeleter {
   fn name(&self) -> &'static str {
      "redis:sessions"
   }

   async fn delete(&self, tenant_id: &TenantId) -> Result<DeletionResult, InfraError> {
      let session_pattern = format!("session:{}:*", tenant_id.as_uuid());
      let csrf_pattern = format!("csrf:{}:*", tenant_id.as_uuid());

      let session_count = self.scan_and_delete(&session_pattern).await?;
      let csrf_count = self.scan_and_delete(&csrf_pattern).await?;

      Ok(DeletionResult {
         deleted_count: session_count + csrf_count,
      })
   }

   async fn count(&self, tenant_id: &TenantId) -> Result<u64, InfraError> {
      let session_pattern = format!("session:{}:*", tenant_id.as_uuid());
      let csrf_pattern = format!("csrf:{}:*", tenant_id.as_uuid());

      let session_count = self.scan_count(&session_pattern).await?;
      let csrf_count = self.scan_count(&csrf_pattern).await?;

      Ok(session_count + csrf_count)
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn test_send_syncを満たす() {
      fn assert_send_sync<T: Send + Sync>() {}
      assert_send_sync::<RedisSessionDeleter>();
   }
}
