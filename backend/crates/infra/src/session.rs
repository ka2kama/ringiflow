//! # セッション管理
//!
//! Redis を使用したセッション管理と CSRF トークン管理を提供する。
//!
//! - 設計: [07_認証機能設計.md](../../../docs/03_詳細設計書/07_認証機能設計.md)
//! - Redis: [Redis.md](../../../docs/06_ナレッジベース/infra/Redis.
//!   md)（Pipeline、SCAN など）
//!
//! ## Redis キー設計
//!
//! | キー | 値 | TTL |
//! |-----|-----|-----|
//! | `session:{tenant_id}:{session_id}` | SessionData (JSON) | 28800秒（8時間） |
//! | `csrf:{tenant_id}:{session_id}` | CSRF トークン（64文字 hex） | 28800秒（8時間） |
//!
//! ## テナント退会時の削除
//!
//! - `delete_all_for_tenant` で以下を削除:
//!   - `session:{tenant_id}:*` パターンを SCAN して削除
//!   - `csrf:{tenant_id}:*` パターンを SCAN
//!     して削除（セッションに紐づくCSRFトークン）

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
    #[serde(default)]
    permissions: Vec<String>,
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
        permissions: Vec<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            user_id,
            tenant_id,
            email,
            name,
            roles,
            permissions,
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

    pub fn permissions(&self) -> &[String] {
        &self.permissions
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

    /// 指定したセッション ID でセッションを作成する
    ///
    /// DevAuth など、固定のセッション ID を使用したい場合に使用する。
    ///
    /// # 引数
    ///
    /// - `session_id`: セッション ID
    /// - `data`: セッションデータ
    async fn create_with_id(&self, session_id: &str, data: &SessionData) -> Result<(), InfraError>;

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

    /// セッションとCSRFトークンを削除する
    ///
    /// 存在しないセッションを削除しても成功とする。
    /// セッションに紐づくCSRFトークンも自動的に削除される。
    async fn delete(&self, tenant_id: &TenantId, session_id: &str) -> Result<(), InfraError>;

    /// テナントの全セッションとCSRFトークンを削除する（テナント退会時）
    ///
    /// SCAN コマンドでパターンマッチし、該当するキーを全て削除する。
    /// セッションに紐づくCSRFトークンも自動的に削除される。
    async fn delete_all_for_tenant(&self, tenant_id: &TenantId) -> Result<(), InfraError>;

    /// セッションの TTL（残り秒数）を取得する（テスト用）
    async fn get_ttl(
        &self,
        tenant_id: &TenantId,
        session_id: &str,
    ) -> Result<Option<i64>, InfraError>;

    // --- CSRF トークン管理 ---

    /// CSRF トークンを作成する
    ///
    /// 64文字の暗号論的ランダム文字列（hex エンコード）を生成して Redis
    /// に保存する。
    ///
    /// # 引数
    ///
    /// - `tenant_id`: テナント ID
    /// - `session_id`: セッション ID
    ///
    /// # 戻り値
    ///
    /// 生成された CSRF トークン
    async fn create_csrf_token(
        &self,
        tenant_id: &TenantId,
        session_id: &str,
    ) -> Result<String, InfraError>;

    /// CSRF トークンを取得する
    ///
    /// # 引数
    ///
    /// - `tenant_id`: テナント ID
    /// - `session_id`: セッション ID
    ///
    /// # 戻り値
    ///
    /// トークンが存在すれば `Some(token)`、なければ `None`
    async fn get_csrf_token(
        &self,
        tenant_id: &TenantId,
        session_id: &str,
    ) -> Result<Option<String>, InfraError>;

    /// CSRF トークンを削除する
    ///
    /// 存在しないトークンを削除しても成功とする。
    async fn delete_csrf_token(
        &self,
        tenant_id: &TenantId,
        session_id: &str,
    ) -> Result<(), InfraError>;

    /// テナントの全 CSRF トークンを削除する（テナント退会時）
    ///
    /// SCAN コマンドでパターンマッチし、該当するキーを全て削除する。
    async fn delete_all_csrf_for_tenant(&self, tenant_id: &TenantId) -> Result<(), InfraError>;
}

/// Redis を使用したセッションマネージャ
#[derive(Clone)]
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

    /// CSRF トークンキーを生成する
    fn csrf_key(tenant_id: &TenantId, session_id: &str) -> String {
        format!("csrf:{}:{}", tenant_id.as_uuid(), session_id)
    }

    /// テナントの CSRF トークンキーパターンを生成する
    fn tenant_csrf_pattern(tenant_id: &TenantId) -> String {
        format!("csrf:{}:*", tenant_id.as_uuid())
    }

    /// 64文字の暗号論的ランダム文字列（hex）を生成する
    ///
    /// UUID v4 を2つ生成して連結することで64文字の hex 文字列を作成する。
    /// UUID v4 は暗号論的に安全なランダム値を使用する。
    fn generate_csrf_token() -> String {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        format!("{}{}", uuid1.simple(), uuid2.simple())
    }
}

#[async_trait]
impl SessionManager for RedisSessionManager {
    async fn create(&self, data: &SessionData) -> Result<String, InfraError> {
        // UUID v4 でセッション ID を生成（暗号論的に安全なランダム値）
        let session_id = Uuid::new_v4().to_string();
        self.create_with_id(&session_id, data).await?;
        Ok(session_id)
    }

    async fn create_with_id(&self, session_id: &str, data: &SessionData) -> Result<(), InfraError> {
        let key = Self::session_key(data.tenant_id(), session_id);
        let json = serde_json::to_string(data)?;

        let mut conn = self.conn.clone();
        let _: () = conn.set_ex(&key, json, SESSION_TTL_SECONDS).await?;

        Ok(())
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
        let session_key = Self::session_key(tenant_id, session_id);
        let csrf_key = Self::csrf_key(tenant_id, session_id);

        // Pipeline で 1 RTT でセッションと CSRF トークンを同時削除
        let mut conn = self.conn.clone();
        redis::pipe()
            .del(&session_key)
            .del(&csrf_key)
            .query_async::<()>(&mut conn)
            .await?;

        Ok(())
    }

    async fn delete_all_for_tenant(&self, tenant_id: &TenantId) -> Result<(), InfraError> {
        let mut conn = self.conn.clone();

        // セッションを削除
        let pattern = Self::tenant_session_pattern(tenant_id);
        scan_and_delete_keys(&mut conn, &pattern).await?;

        // セッションに紐づく CSRF トークンも削除
        self.delete_all_csrf_for_tenant(tenant_id).await?;

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

    // --- CSRF トークン管理 ---

    async fn create_csrf_token(
        &self,
        tenant_id: &TenantId,
        session_id: &str,
    ) -> Result<String, InfraError> {
        let token = Self::generate_csrf_token();
        let key = Self::csrf_key(tenant_id, session_id);

        let mut conn = self.conn.clone();
        let _: () = conn.set_ex(&key, &token, SESSION_TTL_SECONDS).await?;

        Ok(token)
    }

    async fn get_csrf_token(
        &self,
        tenant_id: &TenantId,
        session_id: &str,
    ) -> Result<Option<String>, InfraError> {
        let key = Self::csrf_key(tenant_id, session_id);
        let mut conn = self.conn.clone();

        let result: Option<String> = conn.get(&key).await?;
        Ok(result)
    }

    async fn delete_csrf_token(
        &self,
        tenant_id: &TenantId,
        session_id: &str,
    ) -> Result<(), InfraError> {
        let key = Self::csrf_key(tenant_id, session_id);
        let mut conn = self.conn.clone();
        let _: () = conn.del(&key).await?;
        Ok(())
    }

    async fn delete_all_csrf_for_tenant(&self, tenant_id: &TenantId) -> Result<(), InfraError> {
        let pattern = Self::tenant_csrf_pattern(tenant_id);
        let mut conn = self.conn.clone();
        scan_and_delete_keys(&mut conn, &pattern).await
    }
}

/// SCAN でパターンにマッチするキーを全て削除する
async fn scan_and_delete_keys(
    conn: &mut ConnectionManager,
    pattern: &str,
) -> Result<(), InfraError> {
    let mut cursor = 0u64;
    loop {
        let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(pattern)
            .arg("COUNT")
            .arg(100)
            .query_async(conn)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_セッションデータにpermissionsが保存される() {
        let session = SessionData::new(
            UserId::from_uuid(uuid::Uuid::nil()),
            TenantId::from_uuid(uuid::Uuid::nil()),
            "test@example.com".to_string(),
            "Test User".to_string(),
            vec!["user".to_string()],
            vec!["workflow:read".to_string(), "task:read".to_string()],
        );

        assert_eq!(
            session.permissions(),
            &["workflow:read".to_string(), "task:read".to_string()]
        );
    }

    #[test]
    fn test_permissionsフィールドなしのjsonからデシリアライズすると空vecになる() {
        // permissions フィールドがない旧形式の JSON
        let json = r#"{
         "user_id": "00000000-0000-0000-0000-000000000000",
         "tenant_id": "00000000-0000-0000-0000-000000000000",
         "email": "test@example.com",
         "name": "Test User",
         "roles": ["user"],
         "created_at": "2024-01-01T00:00:00Z",
         "last_accessed_at": "2024-01-01T00:00:00Z"
      }"#;

        let session: SessionData = serde_json::from_str(json).unwrap();
        assert!(session.permissions().is_empty());
    }
}
