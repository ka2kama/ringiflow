//! # ドメイン層エラー定義
//!
//! ビジネスルール違反やドメイン固有の例外状態を表現するエラー型。
//!
//! ## 設計方針
//!
//! - **型による分類**: エラーの種類を列挙型で明示し、パターンマッチで処理可能に
//! - **thiserror 活用**: `#[error(...)]` マクロでエラーメッセージを自動生成
//! - **HTTP ステータスへのマッピング**: API 層でステータスコードに変換可能
//!
//! ## エラーの種類と HTTP ステータスの対応
//!
//! | エラー種別 | HTTP ステータス | 用途 |
//! |-----------|----------------|------|
//! | `Validation` | 400 Bad Request | 入力値の検証失敗 |
//! | `NotFound` | 404 Not Found | エンティティが存在しない |
//! | `Conflict` | 409 Conflict | 楽観的ロックの失敗 |
//! | `Forbidden` | 403 Forbidden | 権限不足 |
//!
//! ## 使用例
//!
//! ```rust
//! use ringiflow_domain::DomainError;
//!
//! fn validate_name(name: &str) -> Result<(), DomainError> {
//!     if name.is_empty() {
//!         return Err(DomainError::Validation("名前は必須です".to_string()));
//!     }
//!     Ok(())
//! }
//!
//! fn find_workflow(id: &str) -> Result<(), DomainError> {
//!     // データベースから検索...
//!     Err(DomainError::NotFound {
//!         entity_type: "Workflow",
//!         id:          id.to_string(),
//!     })
//! }
//! ```

use thiserror::Error;

/// ドメイン層で発生するエラー
///
/// ビジネスロジックの実行中に発生する例外状態を表現する。
/// API 層でこのエラーを受け取り、適切な HTTP レスポンスに変換する。
///
/// # 設計判断
///
/// - `thiserror` を使用し、`std::error::Error` トレイトを自動実装
/// - 各バリアントに `#[error(...)]` で人間可読なメッセージを定義
/// - `Debug` derive により、ログ出力時に詳細情報を表示可能
#[derive(Debug, Error)]
pub enum DomainError {
    /// バリデーションエラー
    ///
    /// 入力値がビジネスルールに違反している場合に使用する。
    ///
    /// # 例
    ///
    /// - 必須フィールドが未入力
    /// - 文字数制限の超過
    /// - 不正なフォーマット
    #[error("バリデーションエラー: {0}")]
    Validation(String),

    /// エンティティが見つからない
    ///
    /// 指定された ID のエンティティがデータベースに存在しない場合に使用する。
    /// `entity_type` にはエンティティの種類（"Workflow", "Task" など）を指定し、
    /// エラーメッセージを具体的にする。
    ///
    /// # フィールド
    ///
    /// - `entity_type`: エンティティの種類（コンパイル時に決定される `&'static str`）
    /// - `id`: 検索に使用した識別子
    #[error("{entity_type} が見つかりません: {id}")]
    NotFound {
        /// エンティティの種類（"Workflow", "Task", "Document" など）
        entity_type: &'static str,
        /// 検索に使用した識別子
        id:          String,
    },

    /// 競合エラー（楽観的ロック失敗など）
    ///
    /// 同時更新による競合が発生した場合に使用する。
    /// 典型的には、楽観的ロック（バージョン番号チェック）の失敗時に発生する。
    ///
    /// # リトライ戦略
    ///
    /// このエラーが発生した場合、クライアントは最新データを再取得してから
    /// 再度更新を試みる必要がある。
    #[error("競合が発生しました: {0}")]
    Conflict(String),

    /// 権限エラー
    ///
    /// ユーザーに操作の実行権限がない場合に使用する。
    /// 認証（Authentication）ではなく認可（Authorization）の失敗を表す。
    ///
    /// # 認証エラーとの違い
    ///
    /// - 認証エラー（401）: ユーザーが誰か不明
    /// - 認可エラー（403）: ユーザーは特定できたが、権限がない
    #[error("権限がありません: {0}")]
    Forbidden(String),
}
