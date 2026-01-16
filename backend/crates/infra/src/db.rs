//! # PostgreSQL データベース接続管理
//!
//! データベース接続プールの作成と管理を行う。
//!
//! ## 設計方針
//!
//! - **接続プール**: 毎回接続を張り直すオーバーヘッドを避け、接続を再利用
//! - **sqlx 採用**: コンパイル時 SQL 検証、非同期サポート、型安全なクエリ
//! - **PostgreSQL 専用**: Aurora PostgreSQL との互換性を考慮
//!
//! ## 接続プールとは
//!
//! データベース接続は TCP 接続の確立、認証、SSL ハンドシェイクなど
//! コストの高い処理を伴う。接続プールは以下のように動作する:
//!
//! 1. 起動時に複数の接続を事前に確立
//! 2. クエリ実行時にプールから接続を借りる
//! 3. クエリ完了後、接続をプールに返却（切断しない）
//! 4. 次のクエリで同じ接続を再利用
//!
//! ## 本番環境での推奨設定
//!
//! ```rust,ignore
//! PgPoolOptions::new()
//!     .max_connections(20)           // 最大接続数（CPU コア数 × 2〜4）
//!     .min_connections(5)            // 最小接続数（起動時に確保）
//!     .acquire_timeout(Duration::from_secs(5))  // 接続取得タイムアウト
//!     .idle_timeout(Duration::from_secs(600))   // アイドル接続のタイムアウト
//!     .max_lifetime(Duration::from_secs(1800))  // 接続の最大寿命
//! ```
//!
//! ## 使用例
//!
//! ```rust,ignore
//! use ringiflow_infra::db;
//!
//! async fn example() -> Result<(), sqlx::Error> {
//!     let pool = db::create_pool("postgres://user:pass@localhost/ringiflow").await?;
//!
//!     // クエリ実行（接続はプールから自動取得・返却）
//!     let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
//!         .fetch_one(&pool)
//!         .await?;
//!
//!     Ok(())
//! }
//! ```

use std::time::Duration;

use sqlx::{PgPool, postgres::PgPoolOptions};

/// PostgreSQL 接続プールを作成する
///
/// アプリケーション起動時に一度だけ呼び出し、作成したプールを
/// アプリケーション全体で共有する。
///
/// # 引数
///
/// * `database_url` - PostgreSQL 接続 URL
///   - 形式: `postgres://user:password@host:port/database`
///   - SSL: `?sslmode=require` を付与して SSL を強制可能
///
/// # 戻り値
///
/// 成功時は `PgPool`（接続プール）を返す。
/// 失敗時は `sqlx::Error` を返す（接続失敗、認証エラーなど）。
///
/// # 設定値
///
/// - `max_connections(10)`: 最大接続数。本番環境では負荷に応じて調整
/// - `acquire_timeout(5秒)`: 接続取得のタイムアウト。超過時はエラー
///
/// # 例
///
/// ```rust,ignore
/// use ringiflow_infra::db;
///
/// let pool = db::create_pool("postgres://localhost/ringiflow").await?;
/// ```
///
/// # パニック
///
/// この関数はパニックしない。すべてのエラーは `Result` で返される。
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
   PgPoolOptions::new()
      .max_connections(10)
      .acquire_timeout(Duration::from_secs(5))
      .connect(database_url)
      .await
}
