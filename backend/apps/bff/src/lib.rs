//! # BFF (Backend for Frontend) ライブラリ
//!
//! フロントエンド専用の API サーバーのコアモジュール。
//!
//! ## モジュール構成
//!
//! - `client`: 外部 API クライアント（Core API 等）
//! - `handler`: HTTP ハンドラ

pub mod client;
pub mod handler;
