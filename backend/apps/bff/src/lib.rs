//! # BFF (Backend for Frontend) ライブラリ
//!
//! フロントエンド専用の API サーバーのコアモジュール。
//!
//! ## モジュール構成
//!
//! - `client`: 外部 API クライアント（Core API 等）
//! - `dev_auth`: 開発用認証バイパス（`dev-auth` feature 有効時のみ）
//! - `handler`: HTTP ハンドラ
//! - `middleware`: ミドルウェア（CSRF 検証等）

pub mod client;
#[cfg(feature = "dev-auth")]
pub mod dev_auth;
pub mod error;
pub mod handler;
pub mod middleware;
