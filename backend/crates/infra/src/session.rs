//! # セッション管理
//!
//! Redis を使用したセッション管理を提供する。
//!
//! 詳細: [07_認証機能設計.md](../../../docs/03_詳細設計書/07_認証機能設計.md)
//!
//! ## Redis キー設計
//!
//! | キー | 値 | TTL |
//! |-----|-----|-----|
//! | `session:{tenant_id}:{session_id}` | SessionData (JSON) | 28800秒（8時間） |
//!
//! ## テナント退会時の削除
//!
//! `delete_all_for_tenant` で `session:{tenant_id}:*` パターンを SCAN して削除する。

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use redis::{AsyncCommands, aio::ConnectionManager};
use ringiflow_domain::{tenant::TenantId, user::UserId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::InfraError;

/// セッションの有効期限（秒）
/// 8時間 = 28800秒
const SESSION_TTL_SECONDS: u64 = 28800;

/// セッションデータ
///
/// Redis に JSON 形式で保存されるセッション情報。
/// ログイン成功時に作成され、ログアウトまたは TTL 経過で削除される。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
   user_id: UserId,
   tenant_id: TenantId,
   email: String,
   name: String,
   roles: Vec<String>,
   created_at: DateTime<Utc>,
   last_accessed_at: DateTime<Utc>,
}

impl SessionData {
   /// 新しいセッションデータを作成する
   ///
   /// `created_at` と `last_accessed_at` は現在時刻で初期化される。
   pub fn new(
      user_id: UserId,
      tenant_id: TenantId,
      email: String,
      name: String,
      roles: Vec<String>,
   ) -> Self {
      let now = Utc::now();
      Self {
         user_id,
         tenant_id,
         email,
         name,
         roles,
         created_at: now,
         last_accessed_at: now,
      }
   }

   pub fn user_id(&self) -> &UserId {
      &self.user_id
   }

   pub fn tenant_id(&self) -> &TenantId {
      &self.tenant_id
   }

   pub fn email(&self) -> &str {
      &self.email
   }

   pub fn name(&self) -> &str {
      &self.name
   }

   pub fn roles(&self) -> &[String] {
      &self.roles
   }

   pub fn created_at(&self) -> DateTime<Utc> {
      self.created_at
   }

   pub fn last_accessed_at(&self) -> DateTime<Utc> {
      self.last_accessed_at
   }
}

/// セッション管理トレイト
///
/// セッションの作成・取得・削除を行う。
/// 実装は Redis を使用する `RedisSessionManager` を参照。
#[async_trait]
pub trait SessionManager: Send + Sync {
   /// セッションを作成し、セッション ID を返す
   ///
   /// # 戻り値
   ///
   /// 生成されたセッション ID（UUID v4）
   async fn create(&self, data: &SessionData) -> Result<String, InfraError>;

   /// セッションを取得する
   ///
   /// # 引数
   ///
   /// - `tenant_id`: テナント ID
   /// - `session_id`: セッション ID
   ///
   /// # 戻り値
   ///
   /// セッションが存在すれば `Some(SessionData)`、なければ `None`
   async fn get(
      &self,
      tenant_id: &TenantId,
      session_id: &str,
   ) -> Result<Option<SessionData>, InfraError>;

   /// セッションを削除する
   ///
   /// 存在しないセッションを削除しても成功とする。
   async fn delete(&self, tenant_id: &TenantId, session_id: &str) -> Result<(), InfraError>;

   /// テナントの全セッションを削除する（テナント退会時）
   ///
   /// SCAN コマンドでパターンマッチし、該当するキーを全て削除する。
   async fn delete_all_for_tenant(&self, tenant_id: &TenantId) -> Result<(), InfraError>;

   /// セッションの TTL（残り秒数）を取得する（テスト用）
   async fn get_ttl(
      &self,
      tenant_id: &TenantId,
      session_id: &str,
   ) -> Result<Option<i64>, InfraError>;
}

/// Redis を使用したセッションマネージャ
pub struct RedisSessionManager {
   conn: ConnectionManager,
}

impl RedisSessionManager {
   /// 新しい RedisSessionManager を作成する
   ///
   /// # 引数
   ///
   /// - `redis_url`: Redis 接続 URL（例: `redis://localhost:6379`）
   pub async fn new(redis_url: &str) -> Result<Self, InfraError> {
      let client = redis::Client::open(redis_url)?;
      let conn = ConnectionManager::new(client).await?;
      Ok(Self { conn })
   }

   /// セッションキーを生成する
   fn session_key(tenant_id: &TenantId, session_id: &str) -> String {
      format!("session:{}:{}", tenant_id.as_uuid(), session_id)
   }

   /// テナントのセッションキーパターンを生成する
   fn tenant_session_pattern(tenant_id: &TenantId) -> String {
      format!("session:{}:*", tenant_id.as_uuid())
   }
}

#[async_trait]
impl SessionManager for RedisSessionManager {
   async fn create(&self, data: &SessionData) -> Result<String, InfraError> {
      // UUID v4 でセッション ID を生成（暗号論的に安全なランダム値）
      let session_id = Uuid::new_v4().to_string();
      let key = Self::session_key(data.tenant_id(), &session_id);
      let json = serde_json::to_string(data)?;

      let mut conn = self.conn.clone();
      let _: () = conn.set_ex(&key, json, SESSION_TTL_SECONDS).await?;

      Ok(session_id)
   }

   async fn get(
      &self,
      tenant_id: &TenantId,
      session_id: &str,
   ) -> Result<Option<SessionData>, InfraError> {
      let key = Self::session_key(tenant_id, session_id);
      let mut conn = self.conn.clone();

      let result: Option<String> = conn.get(&key).await?;

      match result {
         Some(json) => {
            let data: SessionData = serde_json::from_str(&json)?;
            Ok(Some(data))
         }
         None => Ok(None),
      }
   }

   async fn delete(&self, tenant_id: &TenantId, session_id: &str) -> Result<(), InfraError> {
      let key = Self::session_key(tenant_id, session_id);
      let mut conn = self.conn.clone();
      let _: () = conn.del(&key).await?;
      Ok(())
   }

   async fn delete_all_for_tenant(&self, tenant_id: &TenantId) -> Result<(), InfraError> {
      let pattern = Self::tenant_session_pattern(tenant_id);
      let mut conn = self.conn.clone();

      // SCAN でパターンにマッチするキーを取得して削除
      // KEYS コマンドは本番環境では非推奨だが、SCAN は安全
      let mut cursor = 0u64;
      loop {
         let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(&pattern)
            .arg("COUNT")
            .arg(100)
            .query_async(&mut conn)
            .await?;

         if !keys.is_empty() {
            let _: () = conn.del(&keys).await?;
         }

         cursor = next_cursor;
         if cursor == 0 {
            break;
         }
      }

      Ok(())
   }

   async fn get_ttl(
      &self,
      tenant_id: &TenantId,
      session_id: &str,
   ) -> Result<Option<i64>, InfraError> {
      let key = Self::session_key(tenant_id, session_id);
      let mut conn = self.conn.clone();

      let ttl: i64 = conn.ttl(&key).await?;

      // TTL が -2 の場合はキーが存在しない、-1 の場合は TTL が設定されていない
      if ttl < 0 { Ok(None) } else { Ok(Some(ttl)) }
   }
}
