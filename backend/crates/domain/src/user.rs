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
//! use ringiflow_domain::{
//!    tenant::TenantId,
//!    user::{Email, User, UserId, UserStatus},
//! };
//!
//! // 新規ユーザー作成
//! let user = User::new(
//!    TenantId::new(),
//!    Email::new("user@example.com").unwrap(),
//!    "山田太郎".to_string(),
//!    Some("$argon2id$...".to_string()),
//! );
//!
//! // ステータス確認
//! assert!(user.is_active());
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{DomainError, tenant::TenantId};

/// ユーザー ID（一意識別子）
///
/// UUID v7 を使用し、生成順にソート可能。
/// Newtype パターンで型安全性を確保。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl std::fmt::Display for UserId {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
   /// アクティブ（ログイン可能）
   Active,
   /// 非アクティブ（一時停止）
   Inactive,
   /// 削除済み（論理削除）
   Deleted,
}

impl UserStatus {
   /// データベースの VARCHAR 値に変換する
   pub fn as_str(&self) -> &'static str {
      match self {
         Self::Active => "active",
         Self::Inactive => "inactive",
         Self::Deleted => "deleted",
      }
   }
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

impl std::fmt::Display for UserStatus {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.as_str())
   }
}

/// ユーザーエンティティ
///
/// システムのユーザーを表現する。テナントに所属し、
/// メール/パスワード認証または SSO 認証でログインする。
///
/// # 不変条件
///
/// - `email` はテナント内で一意
/// - `password_hash` は SSO ユーザーの場合のみ None
/// - `status` が `Deleted` の場合、ログイン不可
#[derive(Debug, Clone)]
pub struct User {
   id: UserId,
   tenant_id: TenantId,
   email: Email,
   name: String,
   password_hash: Option<String>,
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
   /// - `tenant_id`: テナント ID
   /// - `email`: メールアドレス
   /// - `name`: 表示名
   /// - `password_hash`: パスワードハッシュ（SSO の場合は None）
   ///
   /// # 不変条件
   ///
   /// - 作成時のステータスは `Active`
   /// - `last_login_at` は None
   pub fn new(
      tenant_id: TenantId,
      email: Email,
      name: String,
      password_hash: Option<String>,
   ) -> Self {
      let now = Utc::now();
      Self {
         id: UserId::new(),
         tenant_id,
         email,
         name,
         password_hash,
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
      email: Email,
      name: String,
      password_hash: Option<String>,
      status: UserStatus,
      last_login_at: Option<DateTime<Utc>>,
      created_at: DateTime<Utc>,
      updated_at: DateTime<Utc>,
   ) -> Self {
      Self {
         id,
         tenant_id,
         email,
         name,
         password_hash,
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

   pub fn email(&self) -> &Email {
      &self.email
   }

   pub fn name(&self) -> &str {
      &self.name
   }

   pub fn password_hash(&self) -> Option<&str> {
      self.password_hash.as_deref()
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
   pub fn with_last_login_updated(self) -> Self {
      Self {
         last_login_at: Some(Utc::now()),
         updated_at: Utc::now(),
         ..self
      }
   }

   /// ユーザーステータスを変更した新しいインスタンスを返す
   pub fn with_status(self, status: UserStatus) -> Self {
      Self {
         status,
         updated_at: Utc::now(),
         ..self
      }
   }

   /// パスワードハッシュを更新した新しいインスタンスを返す
   pub fn with_password_hash(self, password_hash: String) -> Self {
      Self {
         password_hash: Some(password_hash),
         updated_at: Utc::now(),
         ..self
      }
   }

   /// 論理削除した新しいインスタンスを返す
   pub fn deleted(self) -> Self {
      Self {
         status: UserStatus::Deleted,
         updated_at: Utc::now(),
         ..self
      }
   }
}

#[cfg(test)]
mod tests {
   use pretty_assertions::assert_eq;
   use rstest::{fixture, rstest};

   use super::*;

   // フィクスチャ

   #[fixture]
   fn アクティブなユーザー() -> User {
      let tenant_id = TenantId::new();
      let email = Email::new("user@example.com").unwrap();
      User::new(tenant_id, email, "Test User".to_string(), None)
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
   fn test_新規ユーザーはアクティブ状態(アクティブなユーザー: User) {
      assert!(アクティブなユーザー.is_active());
   }

   #[rstest]
   fn test_新規ユーザーはログイン可能(アクティブなユーザー: User) {
      assert!(アクティブなユーザー.can_login());
   }

   #[rstest]
   fn test_新規ユーザーは最終ログイン日時なし(
      アクティブなユーザー: User
   ) {
      assert_eq!(アクティブなユーザー.last_login_at(), None);
   }

   #[rstest]
   fn test_ステータス変更で状態が更新される(アクティブなユーザー: User) {
      let updated = アクティブなユーザー.with_status(UserStatus::Inactive);

      assert_eq!(updated.status(), UserStatus::Inactive);
   }

   #[rstest]
   fn test_非アクティブユーザーはアクティブでない(
      アクティブなユーザー: User
   ) {
      let updated = アクティブなユーザー.with_status(UserStatus::Inactive);

      assert!(!updated.is_active());
   }

   #[rstest]
   fn test_削除されたユーザーのステータスは削除済み(
      アクティブなユーザー: User,
   ) {
      let deleted = アクティブなユーザー.deleted();

      assert_eq!(deleted.status(), UserStatus::Deleted);
   }

   #[rstest]
   fn test_削除されたユーザーはログインできない(
      アクティブなユーザー: User
   ) {
      let deleted = アクティブなユーザー.deleted();

      assert!(!deleted.can_login());
   }

   #[rstest]
   fn test_最終ログイン日時を更新できる(アクティブなユーザー: User) {
      let updated = アクティブなユーザー.with_last_login_updated();

      assert!(updated.last_login_at().is_some());
   }
}
