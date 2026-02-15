//! # Core Service ライブラリ
//!
//! Core Service のユースケースとハンドラを公開する。
//! テスト用に内部モジュールへのアクセスを提供する。

pub mod error;
pub mod handler;
pub mod usecase;

// テストユーティリティ（内部実装、ドキュメントからは隠す）
#[doc(hidden)]
pub mod test_utils;
