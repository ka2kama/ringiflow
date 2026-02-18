//! # Observability 基盤
//!
//! トレーシング初期化とログ出力形式の設定を提供する。
//! 3サービス（BFF / Core Service / Auth Service）で共通のログ初期化ロジックを集約し、
//! 環境変数 `LOG_FORMAT` による JSON / Pretty 出力の切り替えに対応する。

/// ログ出力形式
///
/// 環境変数 `LOG_FORMAT` で切り替える。
/// 値が未設定または不正な場合は [`Pretty`](LogFormat::Pretty) にフォールバックする。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    /// JSON 形式（本番環境向け）
    Json,
    /// 人間が読みやすい形式（開発環境向け）
    #[default]
    Pretty,
}

impl LogFormat {
    /// 文字列からログ形式をパースする
    ///
    /// 不正な値の場合は [`Pretty`](LogFormat::Pretty) にフォールバックし、
    /// stderr に警告を出力する。
    pub fn parse(s: &str) -> Self {
        match s {
            "json" => Self::Json,
            "pretty" => Self::Pretty,
            other => {
                eprintln!("WARNING: unknown LOG_FORMAT={other:?}, falling back to pretty");
                Self::Pretty
            }
        }
    }

    /// 環境変数 `LOG_FORMAT` から読み取る
    ///
    /// 未設定の場合は [`Pretty`](LogFormat::Pretty) をデフォルトとする。
    pub fn from_env() -> Self {
        match std::env::var("LOG_FORMAT") {
            Ok(val) => Self::parse(&val),
            Err(_) => Self::default(),
        }
    }
}

/// トレーシング初期化設定
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// サービス名（JSON ログの `span.service` フィールドに出力）
    pub service_name: String,
    /// ログ出力形式
    pub log_format:   LogFormat,
}

impl TracingConfig {
    /// 新しい設定を作成する
    pub fn new(service_name: impl Into<String>, log_format: LogFormat) -> Self {
        Self {
            service_name: service_name.into(),
            log_format,
        }
    }

    /// 環境変数から設定を読み取る
    ///
    /// `LOG_FORMAT` 環境変数で出力形式を決定する。
    pub fn from_env(service_name: impl Into<String>) -> Self {
        Self::new(service_name, LogFormat::from_env())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== LogFormat::parse テスト =====

    #[test]
    fn test_parse_jsonでjsonを返す() {
        assert_eq!(LogFormat::parse("json"), LogFormat::Json);
    }

    #[test]
    fn test_parse_prettyでprettyを返す() {
        assert_eq!(LogFormat::parse("pretty"), LogFormat::Pretty);
    }

    #[test]
    fn test_parse_不正な値でprettyにフォールバックする() {
        assert_eq!(LogFormat::parse("unknown"), LogFormat::Pretty);
        assert_eq!(LogFormat::parse(""), LogFormat::Pretty);
        assert_eq!(LogFormat::parse("JSON"), LogFormat::Pretty);
    }

    // ===== LogFormat::default テスト =====

    #[test]
    fn test_defaultでprettyを返す() {
        assert_eq!(LogFormat::default(), LogFormat::Pretty);
    }

    // ===== TracingConfig::new テスト =====

    #[test]
    fn test_newでフィールドが正しく設定される() {
        let config = TracingConfig::new("bff", LogFormat::Json);

        assert_eq!(config.service_name, "bff");
        assert_eq!(config.log_format, LogFormat::Json);
    }
}
