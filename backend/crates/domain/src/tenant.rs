//! # テナント
//!
//! マルチテナント SaaS アーキテクチャにおけるテナント（顧客企業）のモデル。
//!
//! ## マルチテナントとは
//!
//! 単一のアプリケーションインスタンスで複数の顧客（テナント）にサービスを提供する
//! アーキテクチャ。各テナントのデータは論理的に分離され、他のテナントからは
//! アクセスできない。
//!
//! ## 設計判断
//!
//! ### Newtype パターンの採用
//!
//! `TenantId` は `Uuid` をラップした Newtype である。これにより:
//!
//! - 型安全性: `TenantId` と `UserId` など、同じ UUID でも異なる型として扱える
//! - コンパイル時検証: 引数の取り違えをコンパイラが検出
//! - ゼロコスト: 実行時のオーバーヘッドなし
//!
//! ### UUID v7 の採用
//!
//! UUID v7 はタイムスタンプベースの UUID であり、以下の利点がある:
//!
//! - 時系列ソート: 生成順にソート可能（インデックス効率が良い）
//! - 一意性: 衝突の可能性が極めて低い
//! - 分散生成: 中央のシーケンス発番機が不要
//!
//! ## 使用例
//!
//! ```rust
//! use ringiflow_domain::tenant::TenantId;
//! use uuid::Uuid;
//!
//! // 新規テナント登録時
//! let tenant_id = TenantId::new();
//!
//! // データベースから取得した UUID から復元
//! let uuid = Uuid::parse_str("01234567-89ab-cdef-0123-456789abcdef").unwrap();
//! let tenant_id = TenantId::from_uuid(uuid);
//!
//! // ログ出力
//! println!("テナント: {}", tenant_id);
//! ```

use derive_more::Display;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DomainError;

/// テナント（顧客企業）の一意識別子
///
/// マルチテナント環境において、データの所属先を識別するために使用する。
/// すべてのビジネスエンティティ（Workflow, Task, Document など）は
/// この `TenantId` を持ち、テナント間のデータ分離を保証する。
///
/// # データベース設計
///
/// テナント分離は以下の方式で実現する（Row-Level Security）:
///
/// - すべてのテーブルに `tenant_id` カラムを追加
/// - クエリ実行時に自動的にテナント ID でフィルタリング
/// - PostgreSQL の RLS（Row Level Security）機能を活用
///
/// # セキュリティ考慮事項
///
/// テナント ID は認証トークン（JWT）から取得し、クライアントからの
/// 直接指定は受け付けない。これにより、テナント境界の突破を防ぐ。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display)]
#[display("{_0}")]
pub struct TenantId(Uuid);

impl TenantId {
   /// 新しいテナント ID を生成する
   ///
   /// UUID v7 を使用するため、生成順にソート可能。
   /// 新規テナント登録時に使用する。
   ///
   /// # 例
   ///
   /// ```rust
   /// use ringiflow_domain::tenant::TenantId;
   ///
   /// let tenant_id = TenantId::new();
   /// // UUID v7 形式の文字列が生成される
   /// println!("{}", tenant_id);
   /// ```
   pub fn new() -> Self {
      Self(Uuid::now_v7())
   }

   /// 既存の UUID からテナント ID を作成する
   ///
   /// データベースから取得した値や、外部システムから受け取った値を
   /// 型安全な `TenantId` に変換する際に使用する。
   ///
   /// # 引数
   ///
   /// * `uuid` - 変換元の UUID
   ///
   /// # 例
   ///
   /// ```rust
   /// use ringiflow_domain::tenant::TenantId;
   /// use uuid::Uuid;
   ///
   /// // データベースから取得した UUID を TenantId に変換
   /// let uuid = Uuid::nil(); // 実際にはDBから取得
   /// let tenant_id = TenantId::from_uuid(uuid);
   /// ```
   pub fn from_uuid(uuid: Uuid) -> Self {
      Self(uuid)
   }

   /// 内部の UUID 参照を取得する
   ///
   /// データベースへの保存や、外部 API との連携時に使用する。
   ///
   /// # 例
   ///
   /// ```rust
   /// use ringiflow_domain::tenant::TenantId;
   ///
   /// let tenant_id = TenantId::new();
   /// let uuid = tenant_id.as_uuid();
   /// // sqlx などでパラメータとして使用
   /// ```
   pub fn as_uuid(&self) -> &Uuid {
      &self.0
   }
}

impl Default for TenantId {
   /// デフォルトで新しいテナント ID を生成する
   fn default() -> Self {
      Self::new()
   }
}

// =========================================================================
// TenantName（テナント名）
// =========================================================================

/// テナント名（値オブジェクト）
///
/// テナント（顧客企業）の表示名。
///
/// # 不変条件
///
/// - 空文字列ではない
/// - 最大 255 文字（DB: `VARCHAR(255)`）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display)]
#[display("{_0}")]
pub struct TenantName(String);

impl TenantName {
   /// テナント名を作成する
   ///
   /// # バリデーション
   ///
   /// - 空文字列ではない
   /// - 前後の空白はトリミング
   /// - 最大 255 文字
   pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
      let value = value.into().trim().to_string();

      if value.is_empty() {
         return Err(DomainError::Validation("テナント名は必須です".to_string()));
      }

      if value.chars().count() > 255 {
         return Err(DomainError::Validation(
            "テナント名は 255 文字以内である必要があります".to_string(),
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

// =========================================================================
// Tenant（テナントエンティティ）
// =========================================================================

/// テナント（顧客企業）エンティティ
///
/// マルチテナント環境における顧客企業を表現する。
/// 現時点では ID と名前のみの最小構成。
///
/// # 不変条件
///
/// - `id` はシステム内で一意
#[derive(Debug, Clone)]
pub struct Tenant {
   id:   TenantId,
   name: TenantName,
}

impl Tenant {
   /// データベースからテナントを復元する
   pub fn from_db(id: TenantId, name: TenantName) -> Self {
      Self { id, name }
   }

   /// テナント ID を取得する
   pub fn id(&self) -> &TenantId {
      &self.id
   }

   /// テナント名を取得する
   pub fn name(&self) -> &TenantName {
      &self.name
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   // TenantName のテスト

   #[test]
   fn test_テナント名は正常な名前を受け入れる() {
      let name = TenantName::new("Development Tenant");
      assert!(name.is_ok());
      assert_eq!(name.unwrap().as_str(), "Development Tenant");
   }

   #[test]
   fn test_テナント名は空文字列を拒否する() {
      assert!(TenantName::new("").is_err());
   }

   #[test]
   fn test_テナント名は空白のみの文字列を拒否する() {
      assert!(TenantName::new("   ").is_err());
   }

   #[test]
   fn test_テナント名は前後の空白をトリミングする() {
      let name = TenantName::new("  Test Tenant  ").unwrap();
      assert_eq!(name.as_str(), "Test Tenant");
   }

   #[test]
   fn test_テナント名は255文字を超えると拒否する() {
      let long_name = "a".repeat(256);
      assert!(TenantName::new(long_name).is_err());
   }

   #[test]
   fn test_テナント名は255文字以内を受け入れる() {
      let name = "a".repeat(255);
      assert!(TenantName::new(name).is_ok());
   }

   // Tenant のテスト

   #[test]
   fn test_from_dbでテナントを復元できる() {
      let id = TenantId::new();
      let name = TenantName::new("Test Tenant").unwrap();
      let tenant = Tenant::from_db(id.clone(), name);

      assert_eq!(tenant.id(), &id);
      assert_eq!(tenant.name().as_str(), "Test Tenant");
   }
}
