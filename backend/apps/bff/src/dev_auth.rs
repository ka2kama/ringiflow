//! # 開発用認証バイパス（DevAuth）
//!
//! 開発環境でフロントエンド開発を先行させるため、
//! ログイン画面なしで認証済み状態を実現する仕組み。
//!
//! ## 使い方
//!
//! 1. 環境変数 `DEV_AUTH_ENABLED=true` を設定して BFF を起動
//! 2. フロントエンドで Cookie `session_id=dev-session` を設定
//! 3. `X-Tenant-ID` ヘッダーに `DEV_TENANT_ID` を設定
//!
//! 詳細: [DevAuth](../../../docs/06_ナレッジベース/security/DevAuth.md)
//!
//! ## 安全策
//!
//! - `DEV_AUTH_ENABLED` が設定されていない場合は完全に無効
//! - 起動時に警告ログを出力
//! - 本番環境では絶対に有効にしないこと

use ringiflow_domain::{tenant::TenantId, user::UserId};
use ringiflow_infra::{SessionData, SessionManager};
use uuid::Uuid;

/// 開発用テナント ID
///
/// 固定の UUID を使用することで、フロントエンドとの連携が容易になる。
pub const DEV_TENANT_ID: Uuid = Uuid::from_u128(0x00000000_0000_0000_0000_000000000001);

/// 開発用ユーザー ID
///
/// シードデータの admin ユーザーに対応。
/// 承認ステップの担当者として設定されているため、
/// 承認/却下ボタンの動作確認が可能。
pub const DEV_USER_ID: Uuid = Uuid::from_u128(0x00000000_0000_0000_0000_000000000001);

/// 開発用セッション ID
///
/// フロントエンドはこの値を Cookie `session_id` に設定する。
pub const DEV_SESSION_ID: &str = "dev-session";

/// 開発用ユーザーのメールアドレス
pub const DEV_USER_EMAIL: &str = "admin@example.com";

/// 開発用ユーザーの名前
pub const DEV_USER_NAME: &str = "管理者";

/// 開発用ユーザーのロール
pub const DEV_USER_ROLES: &[&str] = &["tenant_admin"];

/// 開発用ユーザーの権限（tenant_admin 相当）
pub const DEV_USER_PERMISSIONS: &[&str] = &["tenant:*", "user:*", "workflow:*", "task:*"];

/// 開発用セッションをセットアップする
///
/// BFF 起動時に呼び出し、Redis に開発用セッションと CSRF トークンを作成する。
///
/// # 引数
///
/// - `session_manager`: セッション管理
///
/// # 戻り値
///
/// 作成された CSRF トークン
pub async fn setup_dev_session<S: SessionManager>(session_manager: &S) -> anyhow::Result<String> {
    let tenant_id = TenantId::from_uuid(DEV_TENANT_ID);
    let user_id = UserId::from_uuid(DEV_USER_ID);

    // セッションデータを作成
    let session_data = SessionData::new(
        user_id,
        tenant_id.clone(),
        DEV_USER_EMAIL.to_string(),
        DEV_USER_NAME.to_string(),
        DEV_USER_ROLES.iter().map(|s| s.to_string()).collect(),
        DEV_USER_PERMISSIONS.iter().map(|s| s.to_string()).collect(),
    );

    // 既存のセッションを削除（冪等性のため）
    let _ = session_manager.delete(&tenant_id, DEV_SESSION_ID).await;

    // セッションを作成（固定のセッション ID を使用）
    session_manager
        .create_with_id(DEV_SESSION_ID, &session_data)
        .await?;

    // CSRF トークンを作成
    let csrf_token = session_manager
        .create_csrf_token(&tenant_id, DEV_SESSION_ID)
        .await?;

    Ok(csrf_token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_開発用テナントidが固定のuuidである() {
        assert_eq!(
            DEV_TENANT_ID,
            Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
        );
    }

    #[test]
    fn test_開発用ユーザーidが固定のuuidである() {
        assert_eq!(
            DEV_USER_ID,
            Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
        );
    }

    #[test]
    fn test_開発用セッションidがdev_sessionである() {
        assert_eq!(DEV_SESSION_ID, "dev-session");
    }
}
