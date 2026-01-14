//! # RingiFlow API サーバー
//!
//! BFF（Backend for Frontend）と Core API の共通ライブラリ。
//!
//! 詳細: [BFF パターン](../../../docs/05_技術ノート/BFFパターン.md)
//!
//! ## モジュール構成
//!
//! - [`config`] - アプリケーション設定
//! - [`error`] - API エラー定義
//! - [`handler`] - HTTP リクエストハンドラ

pub mod config;
pub mod error;
pub mod handler;
