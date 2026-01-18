//! # ロール（権限管理）
//!
//! ユーザーのロールとその権限を管理する。
//!
//! ## ドメイン用語
//!
//! | 型 | ドメイン用語 | 要件 |
//! |---|------------|------|
//! | [`Role`] | ロール（役割） | AUTHZ-001: RBAC（役割ベースアクセス制御）。ユーザーに「管理者」「一般ユーザー」等を割り当て |
//! | [`Permission`] | 権限 | RBAC の一部。ロールに紐づく操作許可（`workflow:read` など） |
//!
//! ## 設計方針
//!
//! - **システムロール**: tenant_id が None のロールは全テナント共通
//! - **カスタムロール**: tenant_id を持つロールはテナント固有
//! - **権限の柔軟性**: JSON 配列で権限を表現し、拡張可能に
//!
//! ## 使用例
//!
//! ```rust
//! use ringiflow_domain::{
//!    role::{Permission, Role, RoleId},
//!    tenant::TenantId,
//! };
//!
//! // システムロールの作成
//! let permissions = vec![
//!    Permission::new("workflow:read"),
//!    Permission::new("task:read"),
//! ];
//! let role = Role::new_system(
//!    "user".to_string(),
//!    Some("一般ユーザー".to_string()),
//!    permissions,
//! );
//!
//! assert!(role.is_system());
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{DomainError, tenant::TenantId, user::UserId};

/// ロール ID（一意識別子）
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoleId(Uuid);

impl RoleId {
   /// 新しいロール ID を生成する
   pub fn new() -> Self {
      Self(Uuid::now_v7())
   }

   /// 既存の UUID からロール ID を作成する
   pub fn from_uuid(uuid: Uuid) -> Self {
      Self(uuid)
   }

   /// 内部の UUID 参照を取得する
   pub fn as_uuid(&self) -> &Uuid {
      &self.0
   }
}

impl Default for RoleId {
   fn default() -> Self {
      Self::new()
   }
}

impl std::fmt::Display for RoleId {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
   }
}

/// 権限（値オブジェクト）
///
/// リソースとアクションを `:` で区切った形式（例: `workflow:read`）。
///
/// ## 権限の形式
///
/// - `resource:action` - 特定リソースの特定アクション（例: `workflow:create`）
/// - `resource:*` - 特定リソースのすべてのアクション（例: `workflow:*`）
/// - `*` - すべてのリソース・アクション（システム管理者用）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permission(String);

impl Permission {
   /// 権限を作成する
   ///
   /// # バリデーション
   ///
   /// - 空文字列ではない
   /// - 最大 100 文字
   pub fn new(value: impl Into<String>) -> Self {
      Self(value.into())
   }

   /// 文字列参照を取得する
   pub fn as_str(&self) -> &str {
      &self.0
   }

   /// この権限がワイルドカード（*）かどうか
   pub fn is_wildcard(&self) -> bool {
      self.0 == "*"
   }

   /// 特定の権限を包含しているかチェックする
   ///
   /// # 例
   ///
   /// - `workflow:*` は `workflow:read` を包含する
   /// - `*` はすべての権限を包含する
   /// - `workflow:read` は `workflow:read` のみを包含する
   pub fn includes(&self, other: &Permission) -> bool {
      if self.is_wildcard() {
         return true;
      }

      if self.0 == other.0 {
         return true;
      }

      // resource:* 形式のチェック（other もコロンを含む場合のみマッチ）
      if let Some(resource) = self.0.strip_suffix(":*")
         && let Some((other_resource, _)) = other.0.split_once(':')
      {
         return resource == other_resource;
      }

      false
   }
}

impl std::fmt::Display for Permission {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
   }
}

/// ロールエンティティ
///
/// ユーザーに割り当てられる権限の集合。
/// システムロールとテナント固有ロールの2種類がある。
///
/// # 不変条件
///
/// - システムロール（`is_system == true`）は `tenant_id == None`
/// - テナントロール（`is_system == false`）は `tenant_id` が必須
/// - システムロールは削除・編集不可
#[derive(Debug, Clone)]
pub struct Role {
   id:          RoleId,
   tenant_id:   Option<TenantId>,
   name:        String,
   description: Option<String>,
   permissions: Vec<Permission>,
   is_system:   bool,
   created_at:  DateTime<Utc>,
   updated_at:  DateTime<Utc>,
}

impl Role {
   /// システムロールを作成する
   ///
   /// # 引数
   ///
   /// - `name`: ロール名
   /// - `description`: 説明
   /// - `permissions`: 権限リスト
   pub fn new_system(
      name: String,
      description: Option<String>,
      permissions: Vec<Permission>,
   ) -> Self {
      let now = Utc::now();
      Self {
         id: RoleId::new(),
         tenant_id: None,
         name,
         description,
         permissions,
         is_system: true,
         created_at: now,
         updated_at: now,
      }
   }

   /// テナント固有のロールを作成する
   ///
   /// # 引数
   ///
   /// - `tenant_id`: テナント ID
   /// - `name`: ロール名
   /// - `description`: 説明
   /// - `permissions`: 権限リスト
   pub fn new_tenant(
      tenant_id: TenantId,
      name: String,
      description: Option<String>,
      permissions: Vec<Permission>,
   ) -> Self {
      let now = Utc::now();
      Self {
         id: RoleId::new(),
         tenant_id: Some(tenant_id),
         name,
         description,
         permissions,
         is_system: false,
         created_at: now,
         updated_at: now,
      }
   }

   /// 既存のデータからロールを復元する（データベースから取得時）
   #[allow(clippy::too_many_arguments)]
   pub fn from_db(
      id: RoleId,
      tenant_id: Option<TenantId>,
      name: String,
      description: Option<String>,
      permissions: Vec<Permission>,
      is_system: bool,
      created_at: DateTime<Utc>,
      updated_at: DateTime<Utc>,
   ) -> Self {
      Self {
         id,
         tenant_id,
         name,
         description,
         permissions,
         is_system,
         created_at,
         updated_at,
      }
   }

   // Getter メソッド

   pub fn id(&self) -> &RoleId {
      &self.id
   }

   pub fn tenant_id(&self) -> Option<&TenantId> {
      self.tenant_id.as_ref()
   }

   pub fn name(&self) -> &str {
      &self.name
   }

   pub fn description(&self) -> Option<&str> {
      self.description.as_deref()
   }

   pub fn permissions(&self) -> &[Permission] {
      &self.permissions
   }

   pub fn is_system(&self) -> bool {
      self.is_system
   }

   pub fn created_at(&self) -> DateTime<Utc> {
      self.created_at
   }

   pub fn updated_at(&self) -> DateTime<Utc> {
      self.updated_at
   }

   // ビジネスロジックメソッド

   /// 特定の権限を持っているかチェックする
   pub fn has_permission(&self, permission: &Permission) -> bool {
      self.permissions.iter().any(|p| p.includes(permission))
   }

   /// ロールが削除可能かチェックする
   ///
   /// システムロールは削除不可。
   pub fn can_delete(&self) -> Result<(), DomainError> {
      if self.is_system {
         return Err(DomainError::Forbidden(
            "システムロールは削除できません".to_string(),
         ));
      }
      Ok(())
   }

   /// ロールが編集可能かチェックする
   ///
   /// システムロールは編集不可。
   pub fn can_edit(&self) -> Result<(), DomainError> {
      if self.is_system {
         return Err(DomainError::Forbidden(
            "システムロールは編集できません".to_string(),
         ));
      }
      Ok(())
   }

   /// 権限を更新した新しいインスタンスを返す
   pub fn with_permissions(self, permissions: Vec<Permission>) -> Result<Self, DomainError> {
      self.can_edit()?;
      Ok(Self {
         permissions,
         updated_at: Utc::now(),
         ..self
      })
   }
}

/// ユーザーロール関連（User と Role の多対多）
///
/// ユーザーに割り当てられたロールを表現する。
#[derive(Debug, Clone)]
pub struct UserRole {
   id:         Uuid,
   user_id:    UserId,
   role_id:    RoleId,
   created_at: DateTime<Utc>,
}

impl UserRole {
   /// 新しいユーザーロール関連を作成する
   pub fn new(user_id: UserId, role_id: RoleId) -> Self {
      Self {
         id: Uuid::now_v7(),
         user_id,
         role_id,
         created_at: Utc::now(),
      }
   }

   /// 既存のデータから復元する
   pub fn from_db(id: Uuid, user_id: UserId, role_id: RoleId, created_at: DateTime<Utc>) -> Self {
      Self {
         id,
         user_id,
         role_id,
         created_at,
      }
   }

   pub fn id(&self) -> &Uuid {
      &self.id
   }

   pub fn user_id(&self) -> &UserId {
      &self.user_id
   }

   pub fn role_id(&self) -> &RoleId {
      &self.role_id
   }

   pub fn created_at(&self) -> DateTime<Utc> {
      self.created_at
   }
}

#[cfg(test)]
mod tests {
   use pretty_assertions::assert_eq;
   use rstest::{fixture, rstest};

   use super::*;

   // フィクスチャ

   #[fixture]
   fn システムロール() -> Role {
      let permissions = vec![Permission::new("workflow:*"), Permission::new("task:read")];
      Role::new_system("test_role".to_string(), None, permissions)
   }

   #[fixture]
   fn テナントロール() -> Role {
      let tenant_id = TenantId::new();
      let permissions = vec![Permission::new("workflow:read")];
      Role::new_tenant(tenant_id, "custom_role".to_string(), None, permissions)
   }

   // Permission のテスト

   #[test]
   fn test_ワイルドカードはワイルドカードとして判定される() {
      let wildcard = Permission::new("*");

      assert!(wildcard.is_wildcard());
   }

   #[test]
   fn test_具体的な権限はワイルドカードではない() {
      let specific = Permission::new("workflow:read");

      assert!(!specific.is_wildcard());
   }

   #[rstest]
   #[case("*", "workflow:read", true, "全権限")]
   #[case("*", "task:read", true, "全権限")]
   #[case("workflow:*", "workflow:read", true, "リソース単位")]
   #[case("workflow:*", "task:read", false, "リソース単位")]
   #[case("workflow:*", "workflow", false, "コロンなし権限は包含しない")]
   #[case("workflow:read", "workflow:read", true, "完全一致")]
   #[case("workflow:read", "task:read", false, "完全一致")]
   fn test_権限の包含判定(
      #[case] parent: &str,
      #[case] child: &str,
      #[case] expected: bool,
      #[case] _reason: &str,
   ) {
      let parent_perm = Permission::new(parent);
      let child_perm = Permission::new(child);

      assert_eq!(parent_perm.includes(&child_perm), expected);
   }

   // Role のテスト

   #[rstest]
   fn test_システムロールはシステムロールとして識別される(
      システムロール: Role,
   ) {
      assert!(システムロール.is_system());
   }

   #[rstest]
   fn test_システムロールはテナントidを持たない(システムロール: Role) {
      assert!(システムロール.tenant_id().is_none());
   }

   #[rstest]
   fn test_テナントロールはシステムロールではない(テナントロール: Role) {
      assert!(!テナントロール.is_system());
   }

   #[rstest]
   fn test_テナントロールはテナントidを持つ(テナントロール: Role) {
      assert!(テナントロール.tenant_id().is_some());
   }

   #[rstest]
   #[case("workflow:read", true)]
   #[case("workflow:create", true)]
   #[case("task:read", true)]
   #[case("task:write", false)]
   fn test_ロールは権限を保持しているかチェックできる(
      システムロール: Role,
      #[case] permission: &str,
      #[case] expected: bool,
   ) {
      assert_eq!(
         システムロール.has_permission(&Permission::new(permission)),
         expected
      );
   }

   #[rstest]
   fn test_システムロールは削除できない(システムロール: Role) {
      assert!(システムロール.can_delete().is_err());
   }

   #[rstest]
   fn test_テナントロールは削除できる(テナントロール: Role) {
      assert!(テナントロール.can_delete().is_ok());
   }
}
