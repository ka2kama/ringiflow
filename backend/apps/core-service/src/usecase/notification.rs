//! # 通知ユースケース
//!
//! ワークフロー操作に伴うメール通知の生成・送信・ログ記録を統合する。
//!
//! ## モジュール構成
//!
//! - [`template_renderer`] - tera テンプレートエンジンによるメール生成
//! - [`service`] - テンプレートレンダリング + 送信 + ログ記録の統合サービス

pub mod service;
pub mod template_renderer;

pub use service::NotificationService;
pub use template_renderer::TemplateRenderer;
