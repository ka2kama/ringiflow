//! # RingiFlow API サーバー
//!
//! BFF（Backend for Frontend）と Core API の共通ライブラリ。
//!
//! ## アーキテクチャ
//!
//! RingiFlow は 2 種類の API サーバーを持つ:
//!
//! ```text
//! ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
//! │   Frontend  │────▶│     BFF     │────▶│  Core API   │
//! │    (Elm)    │     │  (port 3000)│     │ (port 3001) │
//! └─────────────┘     └─────────────┘     └─────────────┘
//! ```
//!
//! ### BFF (Backend for Frontend)
//!
//! - フロントエンド専用の API
//! - 認証・セッション管理を担当
//! - フロントエンドに最適化されたレスポンス形式
//! - GraphQL / REST の統合エンドポイント（将来）
//!
//! ### Core API
//!
//! - 内部サービス間通信用の API
//! - ビジネスロジックの実行
//! - データの永続化
//!
//! ## モジュール構成
//!
//! - [`config`] - アプリケーション設定（環境変数からの読み込み）
//! - [`error`] - API エラー定義と HTTP レスポンスへの変換
//! - [`handler`] - HTTP リクエストハンドラ
//!
//! ## 依存関係
//!
//! このクレートは以下のクレートに依存する:
//!
//! - `ringiflow_domain`: ドメインモデル、エラー定義
//! - `ringiflow_infra`: データベース・Redis 接続
//! - `ringiflow_shared`: 共有ユーティリティ
//!
//! ## 使用例
//!
//! ```rust,ignore
//! use ringiflow_api::config::AppConfig;
//! use ringiflow_api::handler::health_check;
//!
//! let config = AppConfig::from_env()?;
//! ```

pub mod config;
pub mod error;
pub mod handler;
