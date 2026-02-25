//! # ビジネスイベントログとエラーコンテキストの構造化ヘルパー
//!
//! AI エージェントが `jq` で効率的に調査できるよう、ログフィールドの命名規約と
//! ヘルパーマクロを提供する。
//!
//! ## ビジネスイベント
//!
//! [`log_business_event!`] マクロで出力する。`event.kind = "business_event"` マーカーが
//! 自動付与され、`jq 'select(.["event.kind"] == "business_event")'` でフィルタできる。
//!
//! ## エラーコンテキスト
//!
//! 既存の `tracing::error!` に `error.category` + `error.kind` フィールドを直接追加する。
//! 定数は [`error`] モジュールで提供。
//!
//! ## フィールド命名規約
//!
//! ドット記法（`event.category`、`error.kind`）を使用。tracing の
//! `$($field:ident).+` パターンでサポートされ、JSON 出力でフラットなキーになる。
//!
//! → 詳細: [ログスキーマ](../../../docs/06_ナレッジベース/backend/log-schema.md)

/// ビジネスイベントを構造化ログとして出力する。
///
/// `event.kind = "business_event"` マーカーを自動付与し、
/// `tracing::info!` レベルで出力する。
///
/// ## 必須フィールド（慣例）
///
/// - `event.category`: イベントカテゴリ（[`event::category`] の定数を使用）
/// - `event.action`: アクション名（[`event::action`] の定数を使用）
/// - `event.tenant_id`: テナント ID
/// - `event.result`: 結果（[`event::result`] の定数を使用）
///
/// ## 推奨フィールド
///
/// - `event.entity_type`: エンティティ種別（[`event::entity_type`] の定数を使用）
/// - `event.entity_id`: エンティティ ID
/// - `event.actor_id`: 操作者 ID
#[macro_export]
macro_rules! log_business_event {
    ($($args:tt)*) => {
        ::tracing::info!(
            event.kind = "business_event",
            $($args)*
        )
    };
}

/// イベントフィールドの定数
pub mod event {
    /// イベントカテゴリ
    pub mod category {
        pub const WORKFLOW: &str = "workflow";
        pub const AUTH: &str = "auth";
        pub const NOTIFICATION: &str = "notification";
    }

    /// イベントアクション
    pub mod action {
        // ワークフロー
        pub const WORKFLOW_CREATED: &str = "workflow.created";
        pub const WORKFLOW_SUBMITTED: &str = "workflow.submitted";
        pub const STEP_APPROVED: &str = "step.approved";
        pub const STEP_REJECTED: &str = "step.rejected";
        pub const STEP_CHANGES_REQUESTED: &str = "step.changes_requested";
        pub const WORKFLOW_RESUBMITTED: &str = "workflow.resubmitted";

        // 認証
        pub const LOGIN_SUCCESS: &str = "auth.login_success";
        pub const LOGIN_FAILURE: &str = "auth.login_failure";
        pub const LOGOUT: &str = "auth.logout";

        // 通知
        pub const NOTIFICATION_SENT: &str = "notification.sent";
        pub const NOTIFICATION_FAILED: &str = "notification.failed";
    }

    /// エンティティ種別
    pub mod entity_type {
        pub const WORKFLOW_INSTANCE: &str = "workflow_instance";
        pub const WORKFLOW_STEP: &str = "workflow_step";
        pub const USER: &str = "user";
        pub const SESSION: &str = "session";
        pub const NOTIFICATION_LOG: &str = "notification_log";
    }

    /// イベント結果
    pub mod result {
        pub const SUCCESS: &str = "success";
        pub const FAILURE: &str = "failure";
    }
}

/// エラーコンテキストフィールドの定数
pub mod error {
    /// エラーカテゴリ
    pub mod category {
        /// インフラストラクチャ（DB、Redis、セッションストア）
        pub const INFRASTRUCTURE: &str = "infrastructure";
        /// 外部サービス呼び出し（Core Service、Auth Service）
        pub const EXTERNAL_SERVICE: &str = "external_service";
    }

    /// エラー種別
    pub mod kind {
        pub const DATABASE: &str = "database";
        pub const SESSION: &str = "session";
        pub const INTERNAL: &str = "internal";
        pub const USER_LOOKUP: &str = "user_lookup";
        pub const PASSWORD_VERIFICATION: &str = "password_verification";
        pub const CSRF_TOKEN: &str = "csrf_token";
        pub const SERVICE_COMMUNICATION: &str = "service_communication";
    }
}
