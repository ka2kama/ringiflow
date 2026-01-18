//! # RingiFlow インフラ層
//!
//! 外部システムとの接続・通信を担当するインフラストラクチャ層。
//!
//! ## 設計方針
//!
//! このクレートはドメイン層で定義されたインターフェース（リポジトリトレイト）の
//! 具体的な実装を提供する。外部システムの詳細をカプセル化し、ドメイン層を
//! インフラの変更から保護する。
//!
//! ## 責務
//!
//! - **データベース接続**: PostgreSQL への接続プール管理
//! - **キャッシュ接続**: Redis への接続管理
//! - **リポジトリ実装**: ドメイン層のリポジトリトレイトの具体実装
//! - **外部 API クライアント**: サードパーティサービスとの通信（将来）
//!
//! ## 依存関係
//!
//! ```text
//! api → infra → domain → shared
//!          ↘      ↓
//!            shared
//! ```
//!
//! インフラ層は `domain` と `shared` に依存する。
//! ドメイン層はインフラ層に依存しない（依存性逆転の原則）。
//!
//! ## モジュール構成
//!
//! - [`db`] - PostgreSQL データベース接続管理
//! - [`redis`] - Redis キャッシュ接続管理
//! - [`error`] - インフラ層エラー定義
//! - [`repository`] - リポジトリ実装
//!
//! ## 使用例
//!
//! ```rust,ignore
//! use ringiflow_infra::{db, redis, repository::UserRepository};
//!
//! async fn setup() -> Result<(), Box<dyn std::error::Error>> {
//!     // データベース接続プールの作成
//!     let pool = db::create_pool("postgres://localhost/ringiflow").await?;
//!
//!     // Redis 接続マネージャの作成
//!     let redis = redis::create_connection_manager("redis://localhost").await?;
//!
//!     Ok(())
//! }
//! ```

pub mod db;
pub mod error;
pub mod password;
pub mod redis;
pub mod repository;
pub mod session;

pub use error::InfraError;
pub use password::{Argon2PasswordChecker, PasswordChecker};
pub use session::{RedisSessionManager, SessionData, SessionManager};
