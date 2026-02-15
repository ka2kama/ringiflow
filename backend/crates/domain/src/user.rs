//! # ユーザー
//!
//! ユーザーエンティティとそれに関連する値オブジェクトを定義する。
//!
//! ## ドメイン用語
//!
//! | 型 | ドメイン用語 | 要件 |
//! |---|------------|------|
//! | [`User`] | ユーザー | CORE-03: ユーザー区分（システム管理者、テナント管理者、一般ユーザー、API 利用者） |
//! | [`UserStatus`] | ユーザー状態 | AUTH-009: アカウント無効化（退職/異動時の即時アクセス停止） |
//!
//! ## 設計方針
//!
//! - **Newtype パターン**: UserId は UUID をラップし、型安全性を確保
//! - **不変性**: エンティティフィールドは基本的に不変、変更はメソッド経由
//! - **バリデーション**: 値オブジェクトの生成時に検証ロジックを実行
//!
//! ## 使用例
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use ringiflow_domain::{
//!     tenant::TenantId,
//!     user::{Email, User, UserId, UserStatus},
//!     value_objects::{DisplayNumber, UserName},
//! };
//!
//! // 新規ユーザー作成
//! let user = User::new(
//!     UserId::new(),
//!     TenantId::new(),
//!     DisplayNumber::new(1)?,
//!     Email::new("user@example.com")?,
//!     UserName::new("山田太郎")?,
//!     chrono::Utc::now(),
//! );
//!
//! // ステータス確認
//! assert!(user.is_active());
//! # Ok(())
//! # }
//! ```

use chrono::{DateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;
use uuid::Uuid;

use crate::{
    DomainError,
    tenant::TenantId,
    value_objects::{DisplayNumber, UserName},
};

/// ユーザー ID（一意識別子）
///
/// UUID v7 を使用し、生成順にソート可能。
/// Newtype パターンで型安全性を確保。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display)]
#[display("{_0}")]
pub struct UserId(Uuid);

impl UserId {
    /// 新しいユーザー ID を生成する
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// 既存の UUID からユーザー ID を作成する
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// 内部の UUID 参照を取得する
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

/// メールアドレス（値オブジェクト）
///
/// RFC 5322 に準拠した形式を要求する。
/// 生成時にバリデーションを実行し、不正な値の作成を防ぐ。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Email(String);

impl Email {
    /// メールアドレスを作成する
    ///
    /// # バリデーション
    ///
    /// - 空文字列ではない
    /// - `@` を含む
    /// - 最大 255 文字
    ///
    /// # エラー
    ///
    /// バリデーションに失敗した場合は `DomainError::Validation` を返す。
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();

        if value.is_empty() {
            return Err(DomainError::Validation(
                "メールアドレスは必須です".to_string(),
            ));
        }

        // 基本的な構造検証: local@domain の形式であること
        let Some((local, domain)) = value.split_once('@') else {
            return Err(DomainError::Validation(
                "メールアドレスの形式が不正です".to_string(),
            ));
        };

        if local.is_empty() || domain.is_empty() {
            return Err(DomainError::Validation(
                "メールアドレスの形式が不正です".to_string(),
            ));
        }

        if value.len() > 255 {
            return Err(DomainError::Validation(
                "メールアドレスは255文字以内である必要があります".to_string(),
            ));
        }

        Ok(Self(value))
    }

    /// 文字列参照を取得する
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 所有権を持つ文字列に変換する
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// ユーザーステータス
///
/// ユーザーの状態を表現する列挙型。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, IntoStaticStr, strum::Display,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum UserStatus {
    /// アクティブ（ログイン可能）
    Active,
    /// 非アクティブ（一時停止）
    Inactive,
    /// 削除済み（論理削除）
    Deleted,
}

impl std::str::FromStr for UserStatus {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            "deleted" => Ok(Self::Deleted),
            _ => Err(DomainError::Validation(format!(
                "不正なユーザーステータス: {}",
                s
            ))),
        }
    }
}

/// ユーザーエンティティ
///
/// システムのユーザーを表現する。テナントに所属し、
/// メール/パスワード認証または SSO 認証でログインする。
/// 認証情報（パスワードハッシュ等）は Auth Service の `auth.credentials`
/// テーブルで管理。
///
/// # 不変条件
///
/// - `email` はテナント内で一意
/// - `display_number` はテナント内で一意
/// - `status` が `Deleted` の場合、ログイン不可
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    id: UserId,
    tenant_id: TenantId,
    display_number: DisplayNumber,
    email: Email,
    name: UserName,
    status: UserStatus,
    last_login_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl User {
    /// 新しいユーザーを作成する
    ///
    /// # 引数
    ///
    /// - `id`: ユーザー ID
    /// - `tenant_id`: テナント ID
    /// - `display_number`: 表示用連番（採番済み）
    /// - `email`: メールアドレス
    /// - `name`: 表示名
    /// - `now`: 現在日時（呼び出し元から注入）
    ///
    /// # 不変条件
    ///
    /// - 作成時のステータスは `Active`
    /// - `last_login_at` は None
    pub fn new(
        id: UserId,
        tenant_id: TenantId,
        display_number: DisplayNumber,
        email: Email,
        name: UserName,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            tenant_id,
            display_number,
            email,
            name,
            status: UserStatus::Active,
            last_login_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// 既存のデータからユーザーを復元する（データベースから取得時）
    #[allow(clippy::too_many_arguments)]
    pub fn from_db(
        id: UserId,
        tenant_id: TenantId,
        display_number: DisplayNumber,
        email: Email,
        name: UserName,
        status: UserStatus,
        last_login_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            tenant_id,
            display_number,
            email,
            name,
            status,
            last_login_at,
            created_at,
            updated_at,
        }
    }

    // Getter メソッド

    pub fn id(&self) -> &UserId {
        &self.id
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn display_number(&self) -> DisplayNumber {
        self.display_number
    }

    pub fn email(&self) -> &Email {
        &self.email
    }

    pub fn name(&self) -> &UserName {
        &self.name
    }

    pub fn status(&self) -> UserStatus {
        self.status
    }

    pub fn last_login_at(&self) -> Option<DateTime<Utc>> {
        self.last_login_at
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    // ビジネスロジックメソッド

    /// ユーザーがアクティブか判定する
    pub fn is_active(&self) -> bool {
        self.status == UserStatus::Active
    }

    /// ユーザーがログイン可能か判定する
    ///
    /// アクティブステータスの場合に true を返す。
    pub fn can_login(&self) -> bool {
        self.is_active()
    }

    /// 最終ログイン日時を更新した新しいインスタンスを返す
    pub fn with_last_login_updated(self, now: DateTime<Utc>) -> Self {
        Self {
            last_login_at: Some(now),
            updated_at: now,
            ..self
        }
    }

    /// ユーザー名を変更した新しいインスタンスを返す
    pub fn with_name(self, name: UserName, now: DateTime<Utc>) -> Self {
        Self {
            name,
            updated_at: now,
            ..self
        }
    }

    /// ユーザーステータスを変更した新しいインスタンスを返す
    pub fn with_status(self, status: UserStatus, now: DateTime<Utc>) -> Self {
        Self {
            status,
            updated_at: now,
            ..self
        }
    }

    /// 論理削除した新しいインスタンスを返す
    pub fn deleted(self, now: DateTime<Utc>) -> Self {
        Self {
            status: UserStatus::Deleted,
            updated_at: now,
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::{fixture, rstest};

    use super::*;
    use crate::value_objects::DisplayNumber;

    // フィクスチャ

    /// テスト用の固定タイムスタンプ
    #[fixture]
    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    #[fixture]
    fn active_user(now: DateTime<Utc>) -> User {
        User::new(
            UserId::new(),
            TenantId::new(),
            DisplayNumber::new(42).unwrap(),
            Email::new("user@example.com").unwrap(),
            UserName::new("Test User").unwrap(),
            now,
        )
    }

    // Email のテスト

    #[test]
    fn test_メールアドレスは正常な形式を受け入れる() {
        assert!(Email::new("user@example.com").is_ok());
    }

    #[rstest]
    #[case("", "空文字列")]
    #[case("no-at-sign", "@記号なし")]
    #[case("@", "@のみ")]
    #[case("@example.com", "ローカル部分が空")]
    #[case("user@", "ドメイン部分が空")]
    #[case(&format!("{}@example.com", "a".repeat(256)), "255文字超過")]
    fn test_メールアドレスは不正な形式を拒否する(
        #[case] input: &str,
        #[case] _reason: &str,
    ) {
        assert!(Email::new(input).is_err());
    }

    // User のテスト

    #[rstest]
    fn test_新規ユーザーはアクティブ状態(active_user: User) {
        assert!(active_user.is_active());
    }

    #[rstest]
    fn test_新規ユーザーはログイン可能(active_user: User) {
        assert!(active_user.can_login());
    }

    #[rstest]
    fn test_新規ユーザーは最終ログイン日時なし(active_user: User) {
        assert_eq!(active_user.last_login_at(), None);
    }

    #[rstest]
    fn test_新規ユーザーのcreated_atとupdated_atは注入された値と一致する(
        now: DateTime<Utc>,
        active_user: User,
    ) {
        assert_eq!(active_user.created_at(), now);
        assert_eq!(active_user.updated_at(), now);
    }

    #[rstest]
    fn test_ステータス変更後の状態(active_user: User) {
        let transition_time = DateTime::from_timestamp(1_700_001_000, 0).unwrap();
        let original = active_user.clone();
        let sut = active_user.with_status(UserStatus::Inactive, transition_time);

        let expected = User::from_db(
            original.id().clone(),
            original.tenant_id().clone(),
            original.display_number(),
            original.email().clone(),
            original.name().clone(),
            UserStatus::Inactive,
            original.last_login_at(),
            original.created_at(),
            transition_time,
        );
        assert_eq!(sut, expected);
    }

    #[rstest]
    fn test_非アクティブユーザーはアクティブでない(active_user: User) {
        let transition_time = DateTime::from_timestamp(1_700_001_000, 0).unwrap();
        let updated = active_user.with_status(UserStatus::Inactive, transition_time);

        assert!(!updated.is_active());
    }

    #[rstest]
    fn test_削除後の状態(active_user: User) {
        let transition_time = DateTime::from_timestamp(1_700_001_000, 0).unwrap();
        let original = active_user.clone();
        let sut = active_user.deleted(transition_time);

        let expected = User::from_db(
            original.id().clone(),
            original.tenant_id().clone(),
            original.display_number(),
            original.email().clone(),
            original.name().clone(),
            UserStatus::Deleted,
            original.last_login_at(),
            original.created_at(),
            transition_time,
        );
        assert_eq!(sut, expected);
    }

    #[rstest]
    fn test_削除されたユーザーはログインできない(active_user: User) {
        let transition_time = DateTime::from_timestamp(1_700_001_000, 0).unwrap();
        let deleted = active_user.deleted(transition_time);

        assert!(!deleted.can_login());
    }

    #[rstest]
    fn test_最終ログイン日時更新後の状態(active_user: User) {
        let login_time = DateTime::from_timestamp(1_700_001_000, 0).unwrap();
        let original = active_user.clone();
        let sut = active_user.with_last_login_updated(login_time);

        let expected = User::from_db(
            original.id().clone(),
            original.tenant_id().clone(),
            original.display_number(),
            original.email().clone(),
            original.name().clone(),
            original.status(),
            Some(login_time),
            original.created_at(),
            login_time,
        );
        assert_eq!(sut, expected);
    }

    #[rstest]
    fn test_名前変更後の状態(active_user: User) {
        let transition_time = DateTime::from_timestamp(1_700_001_000, 0).unwrap();
        let original = active_user.clone();
        let new_name = UserName::new("新しい名前").unwrap();
        let sut = active_user.with_name(new_name.clone(), transition_time);

        let expected = User::from_db(
            original.id().clone(),
            original.tenant_id().clone(),
            original.display_number(),
            original.email().clone(),
            new_name,
            original.status(),
            original.last_login_at(),
            original.created_at(),
            transition_time,
        );
        assert_eq!(sut, expected);
    }

    #[rstest]
    fn test_ユーザーから表示用連番を取得できる(active_user: User) {
        assert_eq!(active_user.display_number().as_i64(), 42);
    }
}
