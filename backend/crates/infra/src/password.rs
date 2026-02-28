//! # パスワード検証
//!
//! Argon2id によるパスワード検証を提供する。
//!
//! 詳細: [パスワードハッシュ](../../../docs/06_ナレッジベース/security/パスワードハッシュ.md)

use argon2::{
    Argon2,
    Params,
    PasswordVerifier as _,
    password_hash::PasswordHash as Argon2PasswordHash,
};
use ringiflow_domain::password::{PasswordHash, PasswordVerifyResult, PlainPassword};

use crate::InfraError;

/// パスワード検証を担当するトレイト
pub trait PasswordChecker: Send + Sync {
    /// パスワードを検証する
    ///
    /// # Errors
    ///
    /// - 不正なハッシュ形式の場合
    fn verify(
        &self,
        password: &PlainPassword,
        hash: &PasswordHash,
    ) -> Result<PasswordVerifyResult, InfraError>;
}

/// Argon2id によるパスワード検証の実装
///
/// OWASP 推奨パラメータ（RFC 9106）を使用:
/// - Memory: 64 MB
/// - Iterations: 1
/// - Parallelism: 1
pub struct Argon2PasswordChecker {
    argon2: Argon2<'static>,
}

impl Argon2PasswordChecker {
    pub fn new() -> Self {
        let params = Params::new(
            65536, // memory (KB) = 64 MB
            1,     // iterations
            1,     // parallelism
            None,  // output length (default: 32)
        )
        .expect("Argon2 パラメータが不正です");

        Self {
            argon2: Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params),
        }
    }
}

impl Default for Argon2PasswordChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl PasswordChecker for Argon2PasswordChecker {
    fn verify(
        &self,
        password: &PlainPassword,
        hash: &PasswordHash,
    ) -> Result<PasswordVerifyResult, InfraError> {
        let parsed = Argon2PasswordHash::new(hash.as_str())
            .map_err(|e| InfraError::unexpected(format!("不正なハッシュ形式: {e}")))?;

        let matched = self
            .argon2
            .verify_password(password.as_str().as_bytes(), &parsed)
            .is_ok();

        Ok(PasswordVerifyResult::from(matched))
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    // シードデータと同じハッシュ（password123）
    const TEST_HASH: &str = "$argon2id$v=19$m=65536,t=1,p=1$olntqw+EoVpwH4B1vUAI0A$5yCA1izLODgz8nQOInDGwbuQB/AS0sIQDwpmIilve5M";

    #[rstest]
    fn test_正しいパスワードを検証できる() {
        let checker = Argon2PasswordChecker::new();
        let password = PlainPassword::new("password123");
        let hash = PasswordHash::new(TEST_HASH);

        let result = checker.verify(&password, &hash).unwrap();

        assert!(result.is_match());
    }

    #[rstest]
    fn test_不正なパスワードを検証できる() {
        let checker = Argon2PasswordChecker::new();
        let password = PlainPassword::new("wrongpassword");
        let hash = PasswordHash::new(TEST_HASH);

        let result = checker.verify(&password, &hash).unwrap();

        assert!(result.is_mismatch());
    }

    #[rstest]
    fn test_不正なハッシュ形式はエラー() {
        let checker = Argon2PasswordChecker::new();
        let password = PlainPassword::new("password123");
        let invalid_hash = PasswordHash::new("not-a-valid-hash");

        let result = checker.verify(&password, &invalid_hash);

        assert!(result.is_err());
    }
}
