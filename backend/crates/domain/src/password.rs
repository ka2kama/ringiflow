//! # パスワード
//!
//! パスワード関連の値オブジェクトを定義する。
//!
//! ## ドメイン用語
//!
//! | 型 | ドメイン用語 | 用途 |
//! |---|------------|------|
//! | [`PlainPassword`] | 平文パスワード | ログイン時の入力値 |
//! | [`PasswordHash`] | パスワードハッシュ | 永続化用のハッシュ値 |
//! | [`PasswordVerifyResult`] | 検証結果 | パスワード検証の成否 |

/// 平文パスワード（ログイン時の入力値）
///
/// ユーザーが入力したパスワードをラップする。
/// ログイン時の検証に使用する。
///
/// # セキュリティ
///
/// Debug 出力ではパスワードの値をマスクする。
#[derive(Clone)]
pub struct PlainPassword(String);

impl std::fmt::Debug for PlainPassword {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      f.debug_tuple("PlainPassword").field(&"[REDACTED]").finish()
   }
}

impl PlainPassword {
   /// パスワードを作成する
   pub fn new(value: impl Into<String>) -> Self {
      Self(value.into())
   }

   /// 文字列参照を取得する
   pub fn as_str(&self) -> &str {
      &self.0
   }
}

/// パスワードハッシュ（永続化用）
///
/// Argon2id でハッシュ化されたパスワード文字列をラップする。
/// データベースに保存される形式。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordHash(String);

impl PasswordHash {
   /// ハッシュ文字列からインスタンスを作成する
   ///
   /// 主にデータベースからの復元時に使用する。
   pub fn new(hash: impl Into<String>) -> Self {
      Self(hash.into())
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

/// パスワード検証結果
///
/// パスワード検証の成否を表す列挙型。
/// bool ではなく専用の型を使うことで、意図が明確になる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordVerifyResult {
   /// パスワードが一致した
   Match,
   /// パスワードが一致しなかった
   Mismatch,
}

impl PasswordVerifyResult {
   /// 一致したかどうかを返す
   pub fn is_match(&self) -> bool {
      matches!(self, Self::Match)
   }

   /// 一致しなかったかどうかを返す
   pub fn is_mismatch(&self) -> bool {
      matches!(self, Self::Mismatch)
   }
}

impl From<bool> for PasswordVerifyResult {
   fn from(matched: bool) -> Self {
      if matched { Self::Match } else { Self::Mismatch }
   }
}

#[cfg(test)]
mod tests {
   use rstest::rstest;

   use super::*;

   #[rstest]
   fn test_平文パスワードを作成できる() {
      let password = PlainPassword::new("password123");
      assert_eq!(password.as_str(), "password123");
   }

   #[rstest]
   fn test_平文パスワードのdebug出力はマスクされる() {
      let password = PlainPassword::new("secret");
      let debug = format!("{:?}", password);
      assert!(debug.contains("[REDACTED]"));
      assert!(!debug.contains("secret"));
   }

   #[rstest]
   fn test_パスワードハッシュを作成できる() {
      let hash = PasswordHash::new("$argon2id$v=19$...");
      assert_eq!(hash.as_str(), "$argon2id$v=19$...");
   }

   #[rstest]
   fn test_検証結果_一致() {
      let result = PasswordVerifyResult::Match;
      assert!(result.is_match());
      assert!(!result.is_mismatch());
   }

   #[rstest]
   fn test_検証結果_不一致() {
      let result = PasswordVerifyResult::Mismatch;
      assert!(!result.is_match());
      assert!(result.is_mismatch());
   }

   #[rstest]
   fn test_boolからの変換() {
      assert_eq!(
         PasswordVerifyResult::from(true),
         PasswordVerifyResult::Match
      );
      assert_eq!(
         PasswordVerifyResult::from(false),
         PasswordVerifyResult::Mismatch
      );
   }
}
