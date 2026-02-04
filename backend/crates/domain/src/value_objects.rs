//! # 共通値オブジェクト
//!
//! 複数のエンティティで共有される値オブジェクトを定義する。
//!
//! ## 設計方針
//!
//! - **Newtype パターン**: プリミティブ型をラップし、型安全性を確保
//! - **バリデーション**: 生成時に検証し、不正な値の存在を型レベルで排除
//! - **不変性**: 一度作成したら変更不可
//!
//! ## 含まれる型
//!
//! | 型 | ラップ対象 | 用途 |
//! |---|-----------|------|
//! | [`Version`] | `u32` | エンティティのバージョン番号 |
//! | [`DisplayNumber`] | `i64` | 表示用連番（テナント内で一意） |
//! | [`DisplayId`] | `prefix + number` | 表示用 ID（`WF-42` 形式） |
//! | [`DisplayIdEntityType`] | enum | 表示用 ID の対象エンティティ種別 |
//! | [`UserName`] | `String` | ユーザー表示名 |
//! | [`WorkflowName`] | `String` | ワークフロー定義名 |

use serde::{Deserialize, Serialize};

use crate::DomainError;

// =========================================================================
// Version（バージョン番号）
// =========================================================================

/// バージョン番号（値オブジェクト）
///
/// ワークフロー定義などのバージョン管理に使用。
/// 1 から始まり、更新のたびにインクリメントされる。
///
/// # 不変条件
///
/// - バージョン番号は 1 以上
/// - u32 の範囲内（0 〜 4,294,967,295）
///
/// # 使用例
///
/// ```rust
/// use ringiflow_domain::value_objects::Version;
///
/// let v1 = Version::initial();
/// assert_eq!(v1.as_u32(), 1);
///
/// let v2 = v1.next();
/// assert_eq!(v2.as_u32(), 2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Version(u32);

impl Version {
   /// 初期バージョン（1）を作成する
   pub fn initial() -> Self {
      Self(1)
   }

   /// 指定した値からバージョンを作成する
   ///
   /// # バリデーション
   ///
   /// - 0 は無効（バージョンは 1 以上）
   ///
   /// # エラー
   ///
   /// バリデーションに失敗した場合は `DomainError::Validation` を返す。
   pub fn new(value: u32) -> Result<Self, DomainError> {
      if value == 0 {
         return Err(DomainError::Validation(
            "バージョン番号は 1 以上である必要があります".to_string(),
         ));
      }
      Ok(Self(value))
   }

   /// 次のバージョンを返す
   ///
   /// # パニック
   ///
   /// u32 の最大値を超える場合はパニックする。
   /// 実運用では到達しない想定。
   pub fn next(&self) -> Self {
      Self(
         self
            .0
            .checked_add(1)
            .expect("バージョン番号がオーバーフローしました"),
      )
   }

   /// 内部の u32 値を取得する
   pub fn as_u32(&self) -> u32 {
      self.0
   }

   /// i32 に変換する（DB 互換用）
   ///
   /// # パニック
   ///
   /// i32 の範囲を超える場合はパニックする。
   pub fn as_i32(&self) -> i32 {
      i32::try_from(self.0).expect("バージョン番号が i32 の範囲を超えています")
   }
}

impl TryFrom<i32> for Version {
   type Error = DomainError;

   /// i32 から Version への変換を試みる
   ///
   /// # エラー
   ///
   /// - 値が 0 以下の場合は `DomainError::Validation` を返す
   fn try_from(value: i32) -> Result<Self, Self::Error> {
      if value <= 0 {
         return Err(DomainError::Validation(
            "バージョン番号は 1 以上である必要があります".to_string(),
         ));
      }
      Ok(Self(value as u32))
   }
}

impl Default for Version {
   fn default() -> Self {
      Self::initial()
   }
}

impl std::fmt::Display for Version {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "v{}", self.0)
   }
}

// =========================================================================
// DisplayNumber（表示用連番）
// =========================================================================

/// 表示用連番（値オブジェクト）
///
/// テナント内で一意な連番。ワークフローインスタンスなどの表示用 ID に使用する。
/// 表示形式（例: `WF-42`）のプレフィックスはこの型の責務外。
///
/// # 不変条件
///
/// - 1 以上の正整数
///
/// # 使用例
///
/// ```rust
/// use ringiflow_domain::value_objects::DisplayNumber;
///
/// let num = DisplayNumber::new(42).unwrap();
/// assert_eq!(num.as_i64(), 42);
/// assert_eq!(num.to_string(), "42");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DisplayNumber(i64);

impl DisplayNumber {
   /// 指定した値から表示用連番を作成する
   ///
   /// # バリデーション
   ///
   /// - 0 以下は無効（表示用連番は 1 以上）
   ///
   /// # エラー
   ///
   /// バリデーションに失敗した場合は `DomainError::Validation` を返す。
   pub fn new(value: i64) -> Result<Self, DomainError> {
      if value <= 0 {
         return Err(DomainError::Validation(
            "表示用連番は 1 以上である必要があります".to_string(),
         ));
      }
      Ok(Self(value))
   }

   /// 内部の i64 値を取得する
   pub fn as_i64(&self) -> i64 {
      self.0
   }
}

impl TryFrom<i64> for DisplayNumber {
   type Error = DomainError;

   /// i64 から DisplayNumber への変換を試みる
   ///
   /// # エラー
   ///
   /// - 値が 0 以下の場合は `DomainError::Validation` を返す
   fn try_from(value: i64) -> Result<Self, Self::Error> {
      Self::new(value)
   }
}

impl std::fmt::Display for DisplayNumber {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
   }
}

// =========================================================================
// display_prefix（表示用 ID プレフィックス定数）
// =========================================================================

/// 表示用 ID のプレフィックス定数
///
/// エンティティ種別ごとに固定の文字列を定義する。
/// API レスポンスでは `{prefix}-{number}` 形式で使用する。
pub mod display_prefix {
   /// ワークフローインスタンスのプレフィックス
   pub const WORKFLOW_INSTANCE: &str = "WF";
   /// ワークフローステップのプレフィックス
   pub const WORKFLOW_STEP: &str = "STEP";
}

// =========================================================================
// DisplayIdEntityType（表示用 ID 対象エンティティ種別）
// =========================================================================

/// 表示用 ID のカウンター対象エンティティ種別
///
/// `display_id_counters` テーブルの `entity_type` カラムに対応する。
/// テナント × エンティティ種別ごとに独立した連番を管理する。
///
/// # 使用例
///
/// ```rust
/// use ringiflow_domain::value_objects::DisplayIdEntityType;
///
/// let entity_type = DisplayIdEntityType::WorkflowInstance;
/// assert_eq!(entity_type.as_str(), "workflow_instance");
/// assert_eq!(entity_type.prefix(), "WF");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DisplayIdEntityType {
   /// ワークフローインスタンス
   WorkflowInstance,
   /// ワークフローステップ
   WorkflowStep,
}

impl DisplayIdEntityType {
   /// DB に保存する文字列を返す
   pub fn as_str(&self) -> &'static str {
      match self {
         Self::WorkflowInstance => "workflow_instance",
         Self::WorkflowStep => "workflow_step",
      }
   }

   /// 表示用プレフィックスを返す
   pub fn prefix(&self) -> &'static str {
      match self {
         Self::WorkflowInstance => display_prefix::WORKFLOW_INSTANCE,
         Self::WorkflowStep => display_prefix::WORKFLOW_STEP,
      }
   }
}

// =========================================================================
// DisplayId（表示用 ID）
// =========================================================================

/// 表示用 ID（プレフィックス + 連番）
///
/// API レスポンスで使用する人間可読な識別子。
/// DB には `display_number`（連番）のみ保存し、プレフィックスはアプリ層で結合する。
///
/// # 不変条件
///
/// - `prefix` はコンパイル時に決まる定数
/// - `number` は 1 以上の正整数（`DisplayNumber` で保証）
///
/// # 使用例
///
/// ```rust
/// use ringiflow_domain::value_objects::{DisplayId, DisplayNumber};
///
/// let id = DisplayId::new("WF", DisplayNumber::new(42).unwrap());
/// assert_eq!(id.to_string(), "WF-42");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DisplayId {
   prefix: &'static str,
   number: DisplayNumber,
}

impl DisplayId {
   /// 表示用 ID を作成する
   pub fn new(prefix: &'static str, number: DisplayNumber) -> Self {
      Self { prefix, number }
   }
}

impl std::fmt::Display for DisplayId {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}-{}", self.prefix, self.number)
   }
}

impl Serialize for DisplayId {
   fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
      serializer.serialize_str(&self.to_string())
   }
}

// =========================================================================
// UserName（ユーザー表示名）
// =========================================================================

/// ユーザー表示名（値オブジェクト）
///
/// ユーザーの表示名を表現する。
///
/// # バリデーション
///
/// - 空文字列ではない
/// - 最大 100 文字
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserName(String);

impl UserName {
   /// ユーザー名を作成する
   ///
   /// # バリデーション
   ///
   /// - 空文字列ではない
   /// - 前後の空白はトリミング
   /// - 最大 100 文字
   pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
      let value = value.into().trim().to_string();

      if value.is_empty() {
         return Err(DomainError::Validation("ユーザー名は必須です".to_string()));
      }

      if value.chars().count() > 100 {
         return Err(DomainError::Validation(
            "ユーザー名は 100 文字以内である必要があります".to_string(),
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

impl std::fmt::Display for UserName {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
   }
}

// =========================================================================
// WorkflowName（ワークフロー名）
// =========================================================================

/// ワークフロー名（値オブジェクト）
///
/// ワークフロー定義の名前を表現する。
///
/// # バリデーション
///
/// - 空文字列ではない
/// - 最大 200 文字
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowName(String);

impl WorkflowName {
   /// ワークフロー名を作成する
   ///
   /// # バリデーション
   ///
   /// - 空文字列ではない
   /// - 前後の空白はトリミング
   /// - 最大 200 文字
   pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
      let value = value.into().trim().to_string();

      if value.is_empty() {
         return Err(DomainError::Validation(
            "ワークフロー名は必須です".to_string(),
         ));
      }

      if value.chars().count() > 200 {
         return Err(DomainError::Validation(
            "ワークフロー名は 200 文字以内である必要があります".to_string(),
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

impl std::fmt::Display for WorkflowName {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
   }
}

// =========================================================================
// テスト
// =========================================================================

#[cfg(test)]
mod tests {
   use pretty_assertions::assert_eq;
   use rstest::rstest;

   use super::*;

   // Version のテスト

   #[test]
   fn test_バージョンの初期値は1() {
      let v = Version::initial();
      assert_eq!(v.as_u32(), 1);
   }

   #[test]
   fn test_バージョンのnextはインクリメントする() {
      let v1 = Version::initial();
      let v2 = v1.next();
      assert_eq!(v2.as_u32(), 2);
   }

   #[test]
   fn test_バージョン1は有効() {
      assert!(Version::new(1).is_ok());
   }

   #[test]
   fn test_バージョン0は無効() {
      assert!(Version::new(0).is_err());
   }

   #[test]
   fn test_バージョンのi32変換() {
      let v = Version::new(42).unwrap();
      assert_eq!(v.as_i32(), 42);
   }

   #[test]
   fn test_バージョンのi32からの変換() {
      let v = Version::try_from(42).unwrap();
      assert_eq!(v.as_u32(), 42);
   }

   #[test]
   fn test_バージョンのi32からの変換_0は無効() {
      assert!(Version::try_from(0).is_err());
   }

   #[test]
   fn test_バージョンのi32からの変換_負数は無効() {
      assert!(Version::try_from(-1).is_err());
   }

   // DisplayNumber のテスト

   #[test]
   fn test_表示用連番0は無効() {
      assert!(DisplayNumber::new(0).is_err());
   }

   #[test]
   fn test_表示用連番1は有効() {
      let num = DisplayNumber::new(1).unwrap();
      assert_eq!(num.as_i64(), 1);
   }

   #[test]
   fn test_表示用連番の負数は無効() {
      assert!(DisplayNumber::new(-1).is_err());
   }

   #[test]
   fn test_表示用連番の最大値は有効() {
      assert!(DisplayNumber::new(i64::MAX).is_ok());
   }

   #[test]
   fn test_表示用連番のi64からの変換_0は無効() {
      assert!(DisplayNumber::try_from(0_i64).is_err());
   }

   #[test]
   fn test_表示用連番のi64からの変換_正数は有効() {
      let num = DisplayNumber::try_from(42_i64).unwrap();
      assert_eq!(num.as_i64(), 42);
   }

   #[test]
   fn test_表示用連番の表示形式は数値のみ() {
      let num = DisplayNumber::new(42).unwrap();
      assert_eq!(num.to_string(), "42");
   }

   // DisplayIdEntityType のテスト

   #[test]
   fn test_エンティティ種別の_db文字列_ワークフローインスタンス() {
      assert_eq!(
         DisplayIdEntityType::WorkflowInstance.as_str(),
         "workflow_instance"
      );
   }

   #[test]
   fn test_エンティティ種別の_db文字列_ワークフローステップ() {
      assert_eq!(DisplayIdEntityType::WorkflowStep.as_str(), "workflow_step");
   }

   #[test]
   fn test_エンティティ種別のプレフィックス_ワークフローインスタンス() {
      assert_eq!(DisplayIdEntityType::WorkflowInstance.prefix(), "WF");
   }

   #[test]
   fn test_エンティティ種別のプレフィックス_ワークフローステップ() {
      assert_eq!(DisplayIdEntityType::WorkflowStep.prefix(), "STEP");
   }

   // DisplayId のテスト

   #[test]
   fn test_表示用_idの表示形式_ワークフロー() {
      let id = DisplayId::new("WF", DisplayNumber::new(42).unwrap());
      assert_eq!(id.to_string(), "WF-42");
   }

   #[test]
   fn test_表示用_idの表示形式_ステップ() {
      let id = DisplayId::new("STEP", DisplayNumber::new(7).unwrap());
      assert_eq!(id.to_string(), "STEP-7");
   }

   #[test]
   fn test_表示用_idの_jsonシリアライズは文字列() {
      let id = DisplayId::new("WF", DisplayNumber::new(42).unwrap());
      let json = serde_json::to_string(&id).unwrap();
      assert_eq!(json, "\"WF-42\"");
   }

   // UserName のテスト

   #[test]
   fn test_ユーザー名は正常な値を受け入れる() {
      assert!(UserName::new("山田太郎").is_ok());
   }

   #[rstest]
   #[case("", "空文字列")]
   #[case("   ", "空白のみ")]
   fn test_ユーザー名は空を拒否する(#[case] input: &str, #[case] _reason: &str) {
      assert!(UserName::new(input).is_err());
   }

   #[test]
   fn test_ユーザー名は前後の空白をトリムする() {
      let name = UserName::new("  山田太郎  ").unwrap();
      assert_eq!(name.as_str(), "山田太郎");
   }

   #[test]
   fn test_ユーザー名は100文字まで許容する() {
      let long_name = "あ".repeat(100);
      assert!(UserName::new(&long_name).is_ok());
   }

   #[test]
   fn test_ユーザー名は101文字以上を拒否する() {
      let long_name = "あ".repeat(101);
      assert!(UserName::new(&long_name).is_err());
   }

   // WorkflowName のテスト

   #[test]
   fn test_ワークフロー名は正常な値を受け入れる() {
      assert!(WorkflowName::new("汎用申請").is_ok());
   }

   #[rstest]
   #[case("", "空文字列")]
   #[case("   ", "空白のみ")]
   fn test_ワークフロー名は空を拒否する(#[case] input: &str, #[case] _reason: &str) {
      assert!(WorkflowName::new(input).is_err());
   }

   #[test]
   fn test_ワークフロー名は前後の空白をトリムする() {
      let name = WorkflowName::new("  汎用申請  ").unwrap();
      assert_eq!(name.as_str(), "汎用申請");
   }

   #[test]
   fn test_ワークフロー名は200文字まで許容する() {
      let long_name = "あ".repeat(200);
      assert!(WorkflowName::new(&long_name).is_ok());
   }

   #[test]
   fn test_ワークフロー名は201文字以上を拒否する() {
      let long_name = "あ".repeat(201);
      assert!(WorkflowName::new(&long_name).is_err());
   }
}
