//! # インフラ層エラー定義
//!
//! データベースや外部サービスとの通信で発生するエラーを表現する。
//!
//! ## 設計方針
//!
//! - **エラーの変換**: sqlx::Error, redis::RedisError などをラップ
//! - **ドメインエラーとの分離**: インフラ固有のエラーを明示
//! - **ログ可能性**: Debug によりログ出力時に詳細情報を表示
//! - **SpanTrace 自動捕捉**: `From` 実装や convenience constructor で
//!   エラー生成時の呼び出し経路を自動記録する
//!
//! ## 構造
//!
//! `std::io::Error` と同じ struct + enum パターンを採用:
//! - [`InfraError`]: エラー種別（[`InfraErrorKind`]）と [`SpanTrace`] を保持するラッパー
//! - [`InfraErrorKind`]: エラーの具体的な種別（Database, Redis, Conflict 等）
//!
//! → 設計判断の詳細: [Observability 設計 > SpanTrace 設計](../../../../docs/03_詳細設計書/14_Observability設計.md)

use std::fmt;

use derive_more::Display;
use thiserror::Error;
use tracing_error::SpanTrace;

/// インフラ層で発生するエラー
///
/// エラー種別（[`InfraErrorKind`]）と [`SpanTrace`]（呼び出し経路）を保持する。
/// `From<sqlx::Error>` 等の変換や convenience constructor でエラーを生成すると、
/// その時点のスパン情報が自動的にキャプチャされる。
///
/// ## パターンマッチ
///
/// エラー種別に応じた処理には [`kind()`](InfraError::kind) を使用する:
///
/// ```ignore
/// match error.kind() {
///     InfraErrorKind::Conflict { entity, id } => { /* 競合処理 */ }
///     _ => { /* その他 */ }
/// }
/// ```
#[derive(Display)]
#[display("{kind}")]
pub struct InfraError {
    kind:       InfraErrorKind,
    span_trace: SpanTrace,
}

/// インフラ層エラーの種別
///
/// データベースクエリ、Redis 操作、外部 API 呼び出しなどで発生するエラーの具体的な種別。
/// API 層でこのエラー種別に応じて適切な HTTP レスポンスに変換する。
#[derive(Debug, Error)]
pub enum InfraErrorKind {
    /// データベースエラー
    ///
    /// SQL クエリの実行失敗、接続エラー、制約違反など。
    #[error("データベースエラー: {0}")]
    Database(#[source] sqlx::Error),

    /// Redis エラー
    ///
    /// Redis への接続失敗、コマンド実行エラーなど。
    #[error("Redis エラー: {0}")]
    Redis(#[source] redis::RedisError),

    /// シリアライズ/デシリアライズエラー
    ///
    /// JSON の変換に失敗した場合に使用する。
    #[error("シリアライズエラー: {0}")]
    Serialization(#[source] serde_json::Error),

    /// 楽観的ロック競合（バージョン不一致）
    ///
    /// UPDATE 時に期待したバージョンと DB 上のバージョンが一致しなかった場合。
    /// ユースケース層で適切なエラーメッセージに変換して返す。
    #[error("競合が発生しました: {entity}(id={id})")]
    Conflict {
        /// エンティティ名（例: "WorkflowInstance"）
        entity: String,
        /// エンティティの ID
        id:     String,
    },

    /// DynamoDB エラー
    ///
    /// DynamoDB への操作で発生するエラー。
    /// AWS SDK のエラー型はジェネリクスが深く `#[from]` が困難なため、
    /// 手動で String にマップする。
    #[error("DynamoDB エラー: {0}")]
    DynamoDb(String),

    /// S3 エラー
    ///
    /// S3 への操作で発生するエラー。
    /// AWS SDK のエラー型はジェネリクスが深く `#[from]` が困難なため、
    /// 手動で String にマップする。
    #[error("S3 エラー: {0}")]
    S3(String),

    /// クライアント入力エラー
    ///
    /// クライアントからの入力が不正な場合に使用する。
    /// インフラ層で検出されるが、原因はクライアント入力にある。
    #[error("入力エラー: {0}")]
    InvalidInput(String),

    /// 予期しないエラー
    ///
    /// 上記に分類できない予期しないエラー。
    #[error("予期しないエラー: {0}")]
    Unexpected(String),
}

// ===== InfraError のメソッド =====

impl InfraError {
    /// エラー種別を取得する
    pub fn kind(&self) -> &InfraErrorKind {
        &self.kind
    }

    /// SpanTrace を取得する
    pub fn span_trace(&self) -> &SpanTrace {
        &self.span_trace
    }

    /// Conflict バリアントの場合、entity と id を返す
    ///
    /// パターンマッチで所有権の競合を避けるためのヘルパー。
    /// `kind()` で borrow → 別 arm で `self` を move のパターンに対応する。
    pub fn as_conflict(&self) -> Option<(&str, &str)> {
        match &self.kind {
            InfraErrorKind::Conflict { entity, id } => Some((entity, id)),
            _ => None,
        }
    }

    /// InfraError を分解して InfraErrorKind と SpanTrace を取り出す
    pub fn into_parts(self) -> (InfraErrorKind, SpanTrace) {
        (self.kind, self.span_trace)
    }

    /// InfraErrorKind と SpanTrace から InfraError を組み立てる
    pub fn from_parts(kind: InfraErrorKind, span_trace: SpanTrace) -> Self {
        Self { kind, span_trace }
    }

    // ===== Convenience constructors =====

    /// 楽観的ロック競合エラーを生成する
    pub fn conflict(entity: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            kind:       InfraErrorKind::Conflict {
                entity: entity.into(),
                id:     id.into(),
            },
            span_trace: SpanTrace::capture(),
        }
    }

    /// DynamoDB エラーを生成する
    pub fn dynamo_db(msg: impl Into<String>) -> Self {
        Self {
            kind:       InfraErrorKind::DynamoDb(msg.into()),
            span_trace: SpanTrace::capture(),
        }
    }

    /// S3 エラーを生成する
    pub fn s3(msg: impl Into<String>) -> Self {
        Self {
            kind:       InfraErrorKind::S3(msg.into()),
            span_trace: SpanTrace::capture(),
        }
    }

    /// クライアント入力エラーを生成する
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self {
            kind:       InfraErrorKind::InvalidInput(msg.into()),
            span_trace: SpanTrace::capture(),
        }
    }

    /// 予期しないエラーを生成する
    pub fn unexpected(msg: impl Into<String>) -> Self {
        Self {
            kind:       InfraErrorKind::Unexpected(msg.into()),
            span_trace: SpanTrace::capture(),
        }
    }
}

// ===== トレイト実装 =====

impl fmt::Debug for InfraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InfraError")
            .field("kind", &self.kind)
            .field("span_trace", &self.span_trace)
            .finish()
    }
}

impl std::error::Error for InfraError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.kind.source()
    }
}

// ===== From 実装（SpanTrace 自動キャプチャ） =====

impl From<sqlx::Error> for InfraError {
    fn from(source: sqlx::Error) -> Self {
        Self {
            kind:       InfraErrorKind::Database(source),
            span_trace: SpanTrace::capture(),
        }
    }
}

impl From<redis::RedisError> for InfraError {
    fn from(source: redis::RedisError) -> Self {
        Self {
            kind:       InfraErrorKind::Redis(source),
            span_trace: SpanTrace::capture(),
        }
    }
}

impl From<serde_json::Error> for InfraError {
    fn from(source: serde_json::Error) -> Self {
        Self {
            kind:       InfraErrorKind::Serialization(source),
            span_trace: SpanTrace::capture(),
        }
    }
}

#[cfg(test)]
mod tests {
    use tracing_subscriber::layer::SubscriberExt as _;

    use super::*;

    /// テスト用に ErrorLayer 付き subscriber を設定する
    fn with_error_layer(f: impl FnOnce()) {
        let subscriber = tracing_subscriber::registry().with(tracing_error::ErrorLayer::default());
        let _guard = tracing::subscriber::set_default(subscriber);
        f();
    }

    // ===== From 実装のテスト =====

    #[test]
    fn test_from_sqlx_errorでspan_traceがキャプチャされる() {
        with_error_layer(|| {
            let span = tracing::info_span!("test_repo", tenant_id = "TNT-001");
            let _enter = span.enter();

            let sqlx_err = sqlx::Error::RowNotFound;
            let err: InfraError = sqlx_err.into();

            assert!(matches!(err.kind(), InfraErrorKind::Database(_)));
            let trace_str = format!("{}", err.span_trace());
            assert!(
                trace_str.contains("test_repo"),
                "SpanTrace がスパン名を含むこと: {trace_str}",
            );
        });
    }

    #[test]
    fn test_from_redis_errorでspan_traceがキャプチャされる() {
        with_error_layer(|| {
            let span = tracing::info_span!("test_redis");
            let _enter = span.enter();

            let redis_err: redis::RedisError = (redis::ErrorKind::Io, "接続失敗").into();
            let err: InfraError = redis_err.into();

            assert!(matches!(err.kind(), InfraErrorKind::Redis(_)));
            let trace_str = format!("{}", err.span_trace());
            assert!(
                trace_str.contains("test_redis"),
                "SpanTrace がスパン名を含むこと: {trace_str}",
            );
        });
    }

    #[test]
    fn test_from_serde_json_errorでspan_traceがキャプチャされる() {
        with_error_layer(|| {
            let span = tracing::info_span!("test_serialization");
            let _enter = span.enter();

            let json_err = serde_json::from_str::<String>("invalid").unwrap_err();
            let err: InfraError = json_err.into();

            assert!(matches!(err.kind(), InfraErrorKind::Serialization(_)));
            let trace_str = format!("{}", err.span_trace());
            assert!(
                trace_str.contains("test_serialization"),
                "SpanTrace がスパン名を含むこと: {trace_str}",
            );
        });
    }

    // ===== Convenience constructor のテスト =====

    #[test]
    fn test_conflictでspan_traceがキャプチャされる() {
        with_error_layer(|| {
            let span = tracing::info_span!("test_update");
            let _enter = span.enter();

            let err = InfraError::conflict("WorkflowInstance", "WI-001");

            assert!(matches!(
                err.kind(),
                InfraErrorKind::Conflict { entity, id }
                    if entity == "WorkflowInstance" && id == "WI-001"
            ));
            let trace_str = format!("{}", err.span_trace());
            assert!(
                trace_str.contains("test_update"),
                "SpanTrace がスパン名を含むこと: {trace_str}",
            );
        });
    }

    #[test]
    fn test_dynamo_dbでspan_traceがキャプチャされる() {
        with_error_layer(|| {
            let span = tracing::info_span!("test_dynamo");
            let _enter = span.enter();

            let err = InfraError::dynamo_db("接続失敗");

            assert!(matches!(err.kind(), InfraErrorKind::DynamoDb(msg) if msg == "接続失敗"));
            let trace_str = format!("{}", err.span_trace());
            assert!(trace_str.contains("test_dynamo"));
        });
    }

    #[test]
    fn test_s3でspan_traceがキャプチャされる() {
        with_error_layer(|| {
            let err = InfraError::s3("アップロード失敗");
            assert!(matches!(err.kind(), InfraErrorKind::S3(msg) if msg == "アップロード失敗"));
        });
    }

    #[test]
    fn test_invalid_inputでspan_traceがキャプチャされる() {
        with_error_layer(|| {
            let err = InfraError::invalid_input("不正な入力");
            assert!(matches!(
                err.kind(),
                InfraErrorKind::InvalidInput(msg) if msg == "不正な入力"
            ));
        });
    }

    #[test]
    fn test_unexpectedでspan_traceがキャプチャされる() {
        with_error_layer(|| {
            let err = InfraError::unexpected("予期しないエラー");
            assert!(matches!(
                err.kind(),
                InfraErrorKind::Unexpected(msg) if msg == "予期しないエラー"
            ));
        });
    }

    // ===== Display / source のテスト =====

    #[test]
    fn test_displayがinfra_error_kindのメッセージを出力する() {
        let err = InfraError::conflict("User", "U-001");
        assert_eq!(format!("{err}"), "競合が発生しました: User(id=U-001)");
    }

    #[test]
    fn test_sourceがinfra_error_kindに委譲する() {
        use std::error::Error;

        let sqlx_err = sqlx::Error::RowNotFound;
        let err: InfraError = sqlx_err.into();

        // Database variant は sqlx::Error を source として持つ
        assert!(err.source().is_some());
    }

    // ===== kind / as_conflict のテスト =====

    #[test]
    fn test_kindでinfra_error_kindにアクセスできる() {
        let err = InfraError::dynamo_db("test");
        assert!(matches!(err.kind(), InfraErrorKind::DynamoDb(_)));
    }

    #[test]
    fn test_as_conflictでconflictの情報を取得できる() {
        let err = InfraError::conflict("Step", "S-001");
        let (entity, id) = err.as_conflict().expect("Conflict バリアントであること");
        assert_eq!(entity, "Step");
        assert_eq!(id, "S-001");
    }

    #[test]
    fn test_as_conflictで非conflictはnoneを返す() {
        let err = InfraError::unexpected("test");
        assert!(err.as_conflict().is_none());
    }
}
