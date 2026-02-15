//! # RingiFlow ドメイン層
//!
//! ビジネスロジックの中核を担うドメインモデルを定義する。
//!
//! ## 設計方針
//!
//! このクレートは DDD（ドメイン駆動設計）の原則に従い、以下を提供する:
//!
//! - **エンティティ**: 一意の識別子を持つオブジェクト（例: Workflow, Task）
//! - **値オブジェクト**: 識別子を持たない不変オブジェクト（例: TenantId,
//!   Status）
//! - **ドメインサービス**: エンティティに属さないビジネスロジック
//! - **ドメインエラー**: ビジネスルール違反を表現するエラー型
//!
//! ## 依存関係の方向
//!
//! ```text
//! api → infra → domain → shared
//! ```
//!
//! ドメイン層は `shared` のみに依存し、インフラ層（DB、外部サービス）には
//! 一切依存しない。これにより、ビジネスロジックの純粋性が保たれる。
//!
//! ## モジュール構成
//!
//! - [`error`] - ドメイン層で発生するエラーの定義
//! - [`tenant`] - マルチテナント機能のための識別子
//!
//! ## 使用例
//!
//! ```rust
//! use ringiflow_domain::{DomainError, tenant::TenantId};
//!
//! // テナント ID の生成
//! let tenant_id = TenantId::new();
//!
//! // ドメインエラーの生成
//! let error = DomainError::NotFound {
//!     entity_type: "Workflow",
//!     id:          "wf-123".to_string(),
//! };
//! ```

pub mod audit_log;
pub mod clock;
pub mod error;
pub mod password;
pub mod role;
pub mod tenant;
pub mod user;
pub mod value_objects;
pub mod workflow;

pub use error::DomainError;
