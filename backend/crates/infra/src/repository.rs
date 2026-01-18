//! # リポジトリ実装
//!
//! ドメイン層で定義されたリポジトリトレイトの具体的な実装を提供する。
//!
//! ## 設計方針
//!
//! - **依存性逆転**: ドメイン層のトレイトをインフラ層で実装
//! - **データベース抽象化**: sqlx を使用し、PostgreSQL 固有の処理をカプセル化
//! - **テスタビリティ**: トレイト経由でモック可能な設計

pub mod user_repository;

pub use user_repository::{PostgresUserRepository, UserRepository};
