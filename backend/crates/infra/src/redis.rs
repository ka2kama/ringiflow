//! # Redis 接続管理
//!
//! Redis キャッシュサーバーへの接続管理を行う。
//!
//! ## 設計方針
//!
//! - **ConnectionManager**: 自動再接続機能を持つ接続マネージャを使用
//! - **非同期対応**: tokio ランタイムとの統合
//! - **ElastiCache 互換**: AWS ElastiCache Redis との互換性を確保
//!
//! ## Redis の用途
//!
//! RingiFlow では Redis を以下の目的で使用する:
//!
//! - **セッション管理**: ユーザーセッションの保存
//! - **キャッシュ**: 頻繁にアクセスされるデータのキャッシュ
//! - **レート制限**: API レート制限のカウンター
//! - **分散ロック**: 排他制御（将来）
//!
//! ## ConnectionManager vs Connection
//!
//! | 方式 | 特徴 | 用途 |
//! |------|------|------|
//! | `Connection` | 単一接続、手動管理 | 短期間の処理 |
//! | `ConnectionManager` | 自動再接続、スレッドセーフ | 長期稼働アプリ |
//!
//! `ConnectionManager` は接続が切断された場合に自動で再接続を試みる。
//! これにより、ネットワーク障害からの復旧が容易になる。
//!
//! ## 使用例
//!
//! ```rust,ignore
//! use ringiflow_infra::redis;
//! use redis::AsyncCommands;
//!
//! async fn example() -> Result<(), redis::RedisError> {
//!     let mut conn = redis::create_connection_manager("redis://localhost").await?;
//!
//!     // キー・バリューの設定（有効期限付き）
//!     conn.set_ex("session:abc123", "user_data", 3600).await?;
//!
//!     // 値の取得
//!     let value: Option<String> = conn.get("session:abc123").await?;
//!
//!     Ok(())
//! }
//! ```

use redis::{Client, aio::ConnectionManager};

/// Redis 接続マネージャを作成する
///
/// アプリケーション起動時に一度だけ呼び出し、作成したマネージャを
/// アプリケーション全体で共有する。
///
/// # 引数
///
/// * `redis_url` - Redis 接続 URL
///   - 形式: `redis://[[username:]password@]host[:port][/database]`
///   - TLS: `rediss://` スキームで TLS 接続
///
/// # 戻り値
///
/// 成功時は `ConnectionManager` を返す。
/// 失敗時は `redis::RedisError` を返す。
///
/// # ConnectionManager の特徴
///
/// - **自動再接続**: 接続が切断されても自動的に再接続を試みる
/// - **Clone 可能**: 複数のタスクで安全に共有できる
/// - **非同期**: tokio と統合された非同期 API
///
/// # 例
///
/// ```rust,ignore
/// use ringiflow_infra::redis;
///
/// let conn = redis::create_connection_manager("redis://localhost").await?;
/// ```
///
/// # エラー
///
/// - URL パースエラー: 不正な URL 形式
/// - 接続エラー: Redis サーバーに接続できない
/// - 認証エラー: パスワードが不正
pub async fn create_connection_manager(
   redis_url: &str,
) -> Result<ConnectionManager, redis::RedisError> {
   let client = Client::open(redis_url)?;
   ConnectionManager::new(client).await
}
