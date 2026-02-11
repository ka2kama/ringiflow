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
//! - **system_role**: tenant_id が None のロールは全テナント共通
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
//! // system_roleの作成
//! let permissions = vec![
//!    Permission::new("workflow:read"),
//!    Permission::new("task:read"),
//! ];
//! let role = Role::new_system(
//!    RoleId::new(),
//!    "user".to_string(),
//!    Some("一般ユーザー".to_string()),
//!    permissions,
//!    chrono::Utc::now(),
//! );
//!
//! assert!(role.is_system());
//! ```

use chrono::{DateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{tenant::TenantId, user::UserId};

/// ロール ID（一意識別子）
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display)]
#[display("{_0}")]
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

/// 権限（値オブジェクト）
///
/// リソースとアクションを `:` で区切った形式（例: `workflow:read`）。
///
/// ## 権限の形式
///
/// - `resource:action` - 特定リソースの特定アクション（例: `workflow:create`）
/// - `resource:*` - 特定リソースのすべてのアクション（例: `workflow:*`）
/// - `*` - すべてのリソース・アクション（システム管理者用）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display)]
#[display("{_0}")]
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

   /// この権限が、要求された権限を満たすか判定する
   ///
   /// ## マッチングルール
   ///
   /// | 保持権限 | 要求権限 | 結果 |
   /// |---------|---------|------|
   /// | `*` | 任意 | true（全権限） |
   /// | `user:*` | `user:read` | true（リソース内の全アクション） |
   /// | `user:read` | `user:read` | true（完全一致） |
   /// | `user:read` | `user:write` | false |
   /// | `user:*` | `task:read` | false（リソース不一致） |
   pub fn satisfies(&self, required: &Permission) -> bool {
      let held = self.as_str();
      let req = required.as_str();

      if held == "*" {
         return true;
      }

      if let Some(resource) = held.strip_suffix(":*") {
         return req.starts_with(&format!("{resource}:"));
      }

      held == req
   }
}

/// ロールエンティティ
///
/// ユーザーに割り当てられる権限の集合。
/// system_roleとテナント固有ロールの2種類がある。
///
/// # 不変条件
///
/// - system_role（`is_system == true`）は `tenant_id == None`
/// - テナントロール（`is_system == false`）は `tenant_id` が必須
/// - system_role（`is_system == true`）は DB シードデータで管理
#[derive(Debug, Clone, PartialEq, Eq)]
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
   /// system_roleを作成する
   ///
   /// # 引数
   ///
   /// - `id`: ロール ID
   /// - `name`: ロール名
   /// - `description`: 説明
   /// - `permissions`: 権限リスト
   /// - `now`: 現在日時（呼び出し元から注入）
   pub fn new_system(
      id: RoleId,
      name: String,
      description: Option<String>,
      permissions: Vec<Permission>,
      now: DateTime<Utc>,
   ) -> Self {
      Self {
         id,
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
   /// - `id`: ロール ID
   /// - `tenant_id`: テナント ID
   /// - `name`: ロール名
   /// - `description`: 説明
   /// - `permissions`: 権限リスト
   /// - `now`: 現在日時（呼び出し元から注入）
   pub fn new_tenant(
      id: RoleId,
      tenant_id: TenantId,
      name: String,
      description: Option<String>,
      permissions: Vec<Permission>,
      now: DateTime<Utc>,
   ) -> Self {
      Self {
         id,
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

   // 不変更新メソッド

   /// ロール名を更新する
   pub fn with_name(self, name: String, now: DateTime<Utc>) -> Self {
      Self {
         name,
         updated_at: now,
         ..self
      }
   }

   /// ロールの説明を更新する
   pub fn with_description(self, description: Option<String>, now: DateTime<Utc>) -> Self {
      Self {
         description,
         updated_at: now,
         ..self
      }
   }

   /// ロールの権限を更新する
   pub fn with_permissions(self, permissions: Vec<Permission>, now: DateTime<Utc>) -> Self {
      Self {
         permissions,
         updated_at: now,
         ..self
      }
   }
}

/// ユーザーロール関連（User と Role の多対多）
///
/// ユーザーに割り当てられたロールを表現する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserRole {
   id:         Uuid,
   user_id:    UserId,
   role_id:    RoleId,
   created_at: DateTime<Utc>,
}

impl UserRole {
   /// 新しいユーザーロール関連を作成する
   pub fn new(id: Uuid, user_id: UserId, role_id: RoleId, now: DateTime<Utc>) -> Self {
      Self {
         id,
         user_id,
         role_id,
         created_at: now,
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

   /// テスト用の固定タイムスタンプ
   #[fixture]
   fn now() -> DateTime<Utc> {
      DateTime::from_timestamp(1_700_000_000, 0).unwrap()
   }

   #[fixture]
   fn system_role(now: DateTime<Utc>) -> Role {
      let permissions = vec![Permission::new("workflow:*"), Permission::new("task:read")];
      Role::new_system(
         RoleId::new(),
         "test_role".to_string(),
         None,
         permissions,
         now,
      )
   }

   #[fixture]
   fn tenant_role(now: DateTime<Utc>) -> Role {
      let tenant_id = TenantId::new();
      let permissions = vec![Permission::new("workflow:read")];
      Role::new_tenant(
         RoleId::new(),
         tenant_id,
         "custom_role".to_string(),
         None,
         permissions,
         now,
      )
   }

   // Role のテスト

   #[rstest]
   fn test_システムロールはシステムロールとして識別される(
      system_role: Role,
   ) {
      assert!(system_role.is_system());
   }

   #[rstest]
   fn test_システムロールはテナントidを持たない(system_role: Role) {
      assert!(system_role.tenant_id().is_none());
   }

   #[rstest]
   fn test_テナントロールはシステムロールではない(tenant_role: Role) {
      assert!(!tenant_role.is_system());
   }

   #[rstest]
   fn test_テナントロールはテナントidを持つ(tenant_role: Role) {
      assert!(tenant_role.tenant_id().is_some());
   }

   #[rstest]
   fn test_ロールの初期状態(now: DateTime<Utc>, system_role: Role) {
      let expected = Role::from_db(
         system_role.id().clone(),
         system_role.tenant_id().cloned(),
         system_role.name().to_string(),
         system_role.description().map(|s| s.to_string()),
         system_role.permissions().to_vec(),
         system_role.is_system(),
         now,
         now,
      );
      assert_eq!(system_role, expected);
   }

   // Permission::satisfies のテスト

   #[rstest]
   #[case("*", "user:read")]
   #[case("*", "workflow:create")]
   #[case("*", "task:delete")]
   fn test_全権限ワイルドカードは任意の権限を満たす(
      #[case] held: &str,
      #[case] required: &str,
   ) {
      let held = Permission::new(held);
      let required = Permission::new(required);
      assert!(held.satisfies(&required));
   }

   #[rstest]
   #[case("user:*", "user:read")]
   #[case("user:*", "user:create")]
   #[case("workflow:*", "workflow:approve")]
   fn test_リソースワイルドカードは同一リソースの任意のアクションを満たす(
      #[case] held: &str,
      #[case] required: &str,
   ) {
      let held = Permission::new(held);
      let required = Permission::new(required);
      assert!(held.satisfies(&required));
   }

   #[rstest]
   #[case("user:read", "user:read")]
   #[case("workflow:create", "workflow:create")]
   fn test_完全一致は権限を満たす(#[case] held: &str, #[case] required: &str) {
      let held = Permission::new(held);
      let required = Permission::new(required);
      assert!(held.satisfies(&required));
   }

   #[rstest]
   #[case("user:*", "task:read")]
   #[case("workflow:*", "user:create")]
   fn test_リソースワイルドカードは異なるリソースを満たさない(
      #[case] held: &str,
      #[case] required: &str,
   ) {
      let held = Permission::new(held);
      let required = Permission::new(required);
      assert!(!held.satisfies(&required));
   }

   #[rstest]
   #[case("user:read", "user:write")]
   #[case("workflow:read", "workflow:create")]
   fn test_異なるアクションは権限を満たさない(
      #[case] held: &str,
      #[case] required: &str,
   ) {
      let held = Permission::new(held);
      let required = Permission::new(required);
      assert!(!held.satisfies(&required));
   }

   #[rstest]
   fn test_具体的な権限は全権限ワイルドカードを満たさない() {
      let held = Permission::new("user:read");
      let required = Permission::new("*");
      assert!(!held.satisfies(&required));
   }

   // Role::with_* メソッドのテスト

   /// 更新日時として now() とは異なるタイムスタンプを使用する
   #[fixture]
   fn later() -> DateTime<Utc> {
      DateTime::from_timestamp(1_700_100_000, 0).unwrap()
   }

   #[rstest]
   fn test_with_nameで名前とupdated_atが更新される(
      tenant_role: Role,
      later: DateTime<Utc>,
   ) {
      let original_name = tenant_role.name().to_string();
      let updated = tenant_role
         .clone()
         .with_name("new_role_name".to_string(), later);

      assert_ne!(original_name, updated.name());
      assert_eq!("new_role_name", updated.name());
      assert_eq!(later, updated.updated_at());
      // 他のフィールドは変更されない
      assert_eq!(tenant_role.id(), updated.id());
      assert_eq!(tenant_role.permissions(), updated.permissions());
      assert_eq!(tenant_role.description(), updated.description());
   }

   #[rstest]
   fn test_with_descriptionで説明とupdated_atが更新される(
      tenant_role: Role,
      later: DateTime<Utc>,
   ) {
      let updated = tenant_role
         .clone()
         .with_description(Some("新しい説明".to_string()), later);

      assert_eq!(Some("新しい説明"), updated.description());
      assert_eq!(later, updated.updated_at());
      // 他のフィールドは変更されない
      assert_eq!(tenant_role.id(), updated.id());
      assert_eq!(tenant_role.name(), updated.name());
      assert_eq!(tenant_role.permissions(), updated.permissions());
   }

   #[rstest]
   fn test_with_permissionsで権限とupdated_atが更新される(
      tenant_role: Role,
      later: DateTime<Utc>,
   ) {
      let new_permissions = vec![Permission::new("user:read"), Permission::new("user:create")];
      let updated = tenant_role
         .clone()
         .with_permissions(new_permissions.clone(), later);

      assert_eq!(new_permissions, updated.permissions());
      assert_eq!(later, updated.updated_at());
      // 他のフィールドは変更されない
      assert_eq!(tenant_role.id(), updated.id());
      assert_eq!(tenant_role.name(), updated.name());
      assert_eq!(tenant_role.description(), updated.description());
   }
}
