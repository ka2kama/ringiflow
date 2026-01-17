//! # BFF (Backend for Frontend) ライブラリ
//!
//! フロントエンド専用の API サーバーのコアモジュール。
//!
//! ## モジュール構成
//!
//! - `client`: 外部 API クライアント（Core API 等）
//! - `handler`: HTTP ハンドラ
//! - `middleware`: ミドルウェア（CSRF 検証等）

pub mod client;
pub mod handler;
pub mod middleware;
