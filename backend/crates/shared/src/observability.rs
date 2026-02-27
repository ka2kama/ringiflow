//! # Observability 基盤
//!
//! トレーシング初期化、ログ出力形式の設定、Request ID 管理を提供する。
//! 3サービス（BFF / Core Service / Auth Service）で共通の Observability ロジックを集約し、
//! 環境変数 `LOG_FORMAT` による JSON / Pretty 出力の切り替えに対応する。
//!
//! Request ID 関連:
//! - [`MakeRequestUuidV7`]: UUID v7 ベースの Request ID 生成器
//! - [`make_request_span`]: `X-Request-Id` ヘッダーをトレーシングスパンに注入するカスタムスパン作成関数
//! - [`REQUEST_ID_HEADER`]: `x-request-id` ヘッダー名定数

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

/// トレーシングを初期化する
///
/// `RUST_LOG` 環境変数でログレベルを制御可能。
/// 未設定の場合は `"info,ringiflow=debug"` をデフォルトとする。
///
/// JSON モードでは以下のフィールドがトップレベルに出力される:
/// - `timestamp`, `level`, `target`, `message`
///
/// サービス名は呼び出し元で `tracing::info_span!("app", service = "...")` を設定することで
/// `span.service` として JSON に含まれる。
#[cfg(feature = "observability")]
pub fn init_tracing(config: TracingConfig) {
    use tracing_subscriber::{Layer as _, layer::SubscriberExt, util::SubscriberInitExt};

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,ringiflow=debug".into());

    let fmt_layer = match config.log_format {
        LogFormat::Json => tracing_subscriber::fmt::layer()
            .json()
            .flatten_event(true)
            .with_target(true)
            .with_current_span(true)
            .with_span_list(false)
            .boxed(),
        LogFormat::Pretty => tracing_subscriber::fmt::layer().boxed(),
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}

/// `X-Request-Id` ヘッダー名
///
/// 業界標準の HTTP ヘッダー名。リクエスト追跡に使用する。
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// UUID v7 ベースの Request ID 生成器
///
/// tower-http の [`MakeRequestUuid`](tower_http::request_id::MakeRequestUuid) は UUID v4 を使用するが、
/// UUID v7 は時系列ソート可能で運用時のログ分析に有利なため、独自実装を使用する。
///
/// → ナレッジベース: [Observability > Request ID](../../docs/06_ナレッジベース/backend/observability.md)
#[cfg(feature = "observability")]
#[derive(Clone, Copy, Default)]
pub struct MakeRequestUuidV7;

#[cfg(feature = "observability")]
impl tower_http::request_id::MakeRequestId for MakeRequestUuidV7 {
    fn make_request_id<B>(
        &mut self,
        _request: &http::Request<B>,
    ) -> Option<tower_http::request_id::RequestId> {
        let id = uuid::Uuid::now_v7()
            .to_string()
            .parse()
            .expect("UUID 文字列は有効な HeaderValue");
        Some(tower_http::request_id::RequestId::new(id))
    }
}

/// TraceLayer 用のカスタムスパン作成関数
///
/// `X-Request-Id` ヘッダーから Request ID を、`X-Tenant-Id` ヘッダーから
/// Tenant ID を取得し、トレーシングスパンに記録する。
/// `user_id` は [`tracing::field::Empty`] で宣言し、認証成功後に
/// [`record_user_id`] で記録する。
///
/// JSON ログ形式（`with_current_span(true)`）では、スパンのフィールドが
/// 自動的にログ出力に含まれるため、すべてのログに `request_id` と
/// `tenant_id` が記録される。
#[cfg(feature = "observability")]
pub fn make_request_span<B>(request: &http::Request<B>) -> tracing::Span {
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    let tenant_id = request
        .headers()
        .get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    tracing::info_span!(
        "request",
        method = %request.method(),
        uri = %request.uri(),
        version = ?request.version(),
        request_id = %request_id,
        tenant_id = %tenant_id,
        user_id = tracing::field::Empty,
    )
}

/// 現在のスパンに user_id を記録する
///
/// BFF の認証成功後に呼び出す。[`make_request_span`] で `user_id = Empty` として
/// 宣言されたフィールドに値を設定する。
/// Canonical Log Line を含む、スパン内の後続全ログに `user_id` が含まれる。
#[cfg(feature = "observability")]
pub fn record_user_id(user_id: &impl std::fmt::Display) {
    tracing::Span::current().record("user_id", tracing::field::display(user_id));
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

    // ===== MakeRequestUuidV7 テスト =====

    #[test]
    fn test_make_request_id_uuid_v7形式のrequest_idを返す() {
        use tower_http::request_id::MakeRequestId;

        let mut maker = MakeRequestUuidV7;
        let request = http::Request::builder().body(()).unwrap();

        let id = maker.make_request_id(&request).expect("Some を返す");
        let id_str = id.header_value().to_str().unwrap();

        // UUID v7 形式: xxxxxxxx-xxxx-7xxx-[89ab]xxx-xxxxxxxxxxxx
        let uuid = uuid::Uuid::parse_str(id_str).expect("有効な UUID");
        assert_eq!(uuid.get_version(), Some(uuid::Version::SortRand));
    }

    #[test]
    fn test_make_request_id_連続呼び出しで異なるidを生成する() {
        use tower_http::request_id::MakeRequestId;

        let mut maker = MakeRequestUuidV7;
        let request = http::Request::builder().body(()).unwrap();

        let id1 = maker.make_request_id(&request).unwrap();
        let id2 = maker.make_request_id(&request).unwrap();

        assert_ne!(
            id1.header_value().to_str().unwrap(),
            id2.header_value().to_str().unwrap(),
        );
    }

    // ===== make_request_span テスト =====

    /// テスト用にトレーシング subscriber を設定する
    ///
    /// subscriber がないとスパンが無効化され metadata() が None になるため、
    /// テスト時に最低限の subscriber を登録する。
    fn with_test_subscriber(f: impl FnOnce()) {
        let subscriber = tracing_subscriber::registry();
        let _guard = tracing::subscriber::set_default(subscriber);
        f();
    }

    #[test]
    fn test_make_request_span_ヘッダーの値をスパンに含める() {
        with_test_subscriber(|| {
            let request = http::Request::builder()
                .header(REQUEST_ID_HEADER, "test-request-id-123")
                .body(())
                .unwrap();

            let span = make_request_span(&request);

            // スパンが作成されること（フィールドの値はトレーシング内部のため直接検証困難）
            // スパン名が "request" であることを確認
            assert_eq!(span.metadata().unwrap().name(), "request");
        });
    }

    #[test]
    fn test_make_request_span_ヘッダー未設定時もスパンが作成される() {
        with_test_subscriber(|| {
            let request = http::Request::builder().body(()).unwrap();

            let span = make_request_span(&request);

            assert_eq!(span.metadata().unwrap().name(), "request");
        });
    }

    #[test]
    fn test_make_request_span_tenant_idヘッダーありでスパンが作成される() {
        with_test_subscriber(|| {
            let request = http::Request::builder()
                .header(REQUEST_ID_HEADER, "test-request-id")
                .header("x-tenant-id", "test-tenant-id")
                .body(())
                .unwrap();

            let span = make_request_span(&request);

            // スパンが正常に作成されること（tenant_id フィールドを含む）
            assert_eq!(span.metadata().unwrap().name(), "request");
        });
    }

    #[test]
    fn test_make_request_span_tenant_idヘッダーなしでスパンが作成される() {
        with_test_subscriber(|| {
            let request = http::Request::builder().body(()).unwrap();

            let span = make_request_span(&request);

            // X-Tenant-ID なしでもスパンが作成されること（tenant_id = "-"）
            assert_eq!(span.metadata().unwrap().name(), "request");
        });
    }

    // ===== record_user_id テスト =====

    #[test]
    fn test_record_user_id_スパン内でuser_idを記録できる() {
        with_test_subscriber(|| {
            let request = http::Request::builder()
                .header(REQUEST_ID_HEADER, "test-request-id")
                .body(())
                .unwrap();

            let span = make_request_span(&request);
            let _guard = span.enter();

            // record_user_id がパニックせず、スパンに値を記録できること
            record_user_id(&"test-user-id");
        });
    }
}
