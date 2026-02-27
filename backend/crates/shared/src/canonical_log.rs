//! # Canonical Log Line ミドルウェア
//!
//! HTTP リクエスト完了時に、そのリクエストの重要情報を1行に集約した
//! サマリログ（Canonical Log Line）を出力する tower Layer。
//!
//! Stripe が提唱した [Canonical Log Lines パターン](https://brandur.org/canonical-log-lines)
//! に基づき、ログの検索性・集計性を向上させる。
//!
//! ## 既存 TraceLayer との責務分離
//!
//! - TraceLayer: スパン作成（method, uri, request_id 等）。リクエストスコープのコンテキスト管理
//! - CanonicalLogLineLayer: リクエスト完了サマリ（status, latency）。1行で全体像を提供
//!
//! TraceLayer のスパン内に配置することで、スパンフィールド（request_id, tenant_id, user_id）が
//! JSON ログに自動的に含まれる。
//!
//! → 設計: [Observability 設計書](../../../../docs/03_詳細設計書/14_Observability設計.md)
//! → スキーマ: [ログスキーマ](../../../../docs/06_ナレッジベース/backend/log-schema.md)

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

use http::{Request, Response};
use tower::{Layer, Service};

/// ヘルスチェックパスかどうかを判定する
///
/// `/health`（liveness）と `/health/ready`（readiness）を除外対象とする。
fn is_health_check_path(path: &str) -> bool {
    path.starts_with("/health")
}

/// Canonical Log Line を出力する Layer
///
/// リクエスト完了時に INFO レベルで `log.type = "canonical"` マーカー付きの
/// サマリログを出力する。ヘルスチェックパスは出力対象外。
///
/// ## レイヤー配置
///
/// TraceLayer の内側に配置し、スパンフィールドを活用する:
///
/// ```text
/// TraceLayer → CanonicalLogLineLayer → [他のミドルウェア] → handler
/// ```
#[derive(Clone, Debug)]
pub struct CanonicalLogLineLayer;

impl<S> Layer<S> for CanonicalLogLineLayer {
    type Service = CanonicalLogLineService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CanonicalLogLineService { inner }
    }
}

/// Canonical Log Line を出力する Service
///
/// [`CanonicalLogLineLayer`] が生成する Service 実装。
#[derive(Clone, Debug)]
pub struct CanonicalLogLineService<S> {
    inner: S,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for CanonicalLogLineService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: std::fmt::Display + 'static,
    ReqBody: Send + 'static,
    ResBody: Send + 'static,
{
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    type Response = S::Response;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        // clone-swap パターン: poll_ready で得た readiness を保持する inner を使う
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        let path = req.uri().path().to_owned();

        // ヘルスチェックはスキップ
        if is_health_check_path(&path) {
            return Box::pin(async move { inner.call(req).await });
        }

        let start = Instant::now();

        Box::pin(async move {
            let result = inner.call(req).await;
            let latency_ms = start.elapsed().as_millis() as u64;

            match &result {
                Ok(response) => {
                    let status = response.status().as_u16();
                    tracing::info!(
                        log.r#type = "canonical",
                        http.status_code = status,
                        http.latency_ms = latency_ms,
                        "リクエスト完了"
                    );
                }
                Err(err) => {
                    tracing::error!(
                        log.r#type = "canonical",
                        http.latency_ms = latency_ms,
                        error.message = %err,
                        "リクエスト処理エラー"
                    );
                }
            }

            result
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        convert::Infallible,
        sync::{Arc, Mutex},
    };

    use tracing_subscriber::layer::SubscriberExt;

    use super::*;

    // テスト用のダミー Service
    #[derive(Clone)]
    struct DummyService {
        status: http::StatusCode,
    }

    impl Service<Request<()>> for DummyService {
        type Error = Infallible;
        type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
        type Response = Response<()>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _req: Request<()>) -> Self::Future {
            let status = self.status;
            Box::pin(async move { Ok(Response::builder().status(status).body(()).unwrap()) })
        }
    }

    // テスト用のエラーを返す Service
    #[derive(Clone)]
    struct ErrorService;

    impl Service<Request<()>> for ErrorService {
        type Error = String;
        type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
        type Response = Response<()>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _req: Request<()>) -> Self::Future {
            Box::pin(async { Err("internal service error".to_string()) })
        }
    }

    /// テスト用にログイベントをキャプチャする Layer
    #[derive(Clone)]
    struct CaptureLayer {
        events: Arc<Mutex<Vec<CapturedEvent>>>,
    }

    #[derive(Debug, Clone)]
    struct CapturedEvent {
        level:   tracing::Level,
        message: String,
        fields:  Vec<(String, String)>,
    }

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for CaptureLayer {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            let mut visitor = FieldVisitor::default();
            event.record(&mut visitor);

            let captured = CapturedEvent {
                level:   *event.metadata().level(),
                message: visitor.message.unwrap_or_default(),
                fields:  visitor.fields,
            };

            self.events.lock().unwrap().push(captured);
        }
    }

    #[derive(Default)]
    struct FieldVisitor {
        message: Option<String>,
        fields:  Vec<(String, String)>,
    }

    impl tracing::field::Visit for FieldVisitor {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                self.message = Some(format!("{:?}", value));
            } else {
                self.fields
                    .push((field.name().to_string(), format!("{:?}", value)));
            }
        }

        fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
            self.fields
                .push((field.name().to_string(), value.to_string()));
        }

        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            if field.name() == "message" {
                self.message = Some(value.to_string());
            } else {
                self.fields
                    .push((field.name().to_string(), value.to_string()));
            }
        }
    }

    /// テスト用にキャプチャ subscriber をセットアップする
    ///
    /// 返り値の `DefaultGuard` はスコープに保持すること（ドロップでリセット）。
    fn setup_capture() -> (
        tracing::subscriber::DefaultGuard,
        Arc<Mutex<Vec<CapturedEvent>>>,
    ) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let capture = CaptureLayer {
            events: events.clone(),
        };
        let subscriber = tracing_subscriber::registry().with(capture);
        let guard = tracing::subscriber::set_default(subscriber);
        (guard, events)
    }

    fn build_request(path: &str) -> Request<()> {
        Request::builder().uri(path).body(()).unwrap()
    }

    // ===== is_health_check_path テスト =====

    #[test]
    fn test_is_health_check_path_healthでtrueを返す() {
        assert!(is_health_check_path("/health"));
    }

    #[test]
    fn test_is_health_check_path_health_readyでtrueを返す() {
        assert!(is_health_check_path("/health/ready"));
    }

    #[test]
    fn test_is_health_check_path_apiパスでfalseを返す() {
        assert!(!is_health_check_path("/api/v1/workflows"));
    }

    // ===== CanonicalLogLineService テスト =====

    #[tokio::test]
    async fn test_正常リクエストでcanonical_log_lineがinfoレベルで出力される() {
        let (_guard, events) = setup_capture();

        let mut sut = CanonicalLogLineLayer.layer(DummyService {
            status: http::StatusCode::OK,
        });

        let response = sut.call(build_request("/api/v1/workflows")).await.unwrap();
        assert_eq!(response.status(), http::StatusCode::OK);

        let captured = events.lock().unwrap();
        assert_eq!(captured.len(), 1, "1つのログイベントが出力されること");
        assert_eq!(captured[0].level, tracing::Level::INFO);
        assert_eq!(captured[0].message, "リクエスト完了");
    }

    #[tokio::test]
    async fn test_canonical_log_lineにlog_type_canonicalが含まれる() {
        let (_guard, events) = setup_capture();

        let mut sut = CanonicalLogLineLayer.layer(DummyService {
            status: http::StatusCode::OK,
        });

        sut.call(build_request("/api/v1/test")).await.unwrap();

        let captured = events.lock().unwrap();
        let log_type = captured[0]
            .fields
            .iter()
            .find(|(k, _)| k == "log.type")
            .map(|(_, v)| v.as_str());
        assert_eq!(log_type, Some("canonical"));
    }

    #[tokio::test]
    async fn test_canonical_log_lineにhttp_status_codeが含まれる() {
        let (_guard, events) = setup_capture();

        let mut sut = CanonicalLogLineLayer.layer(DummyService {
            status: http::StatusCode::CREATED,
        });

        sut.call(build_request("/api/v1/test")).await.unwrap();

        let captured = events.lock().unwrap();
        let status = captured[0]
            .fields
            .iter()
            .find(|(k, _)| k == "http.status_code")
            .map(|(_, v)| v.as_str());
        assert_eq!(status, Some("201"));
    }

    #[tokio::test]
    async fn test_canonical_log_lineにhttp_latency_msが含まれる() {
        let (_guard, events) = setup_capture();

        let mut sut = CanonicalLogLineLayer.layer(DummyService {
            status: http::StatusCode::OK,
        });

        sut.call(build_request("/api/v1/test")).await.unwrap();

        let captured = events.lock().unwrap();
        let latency = captured[0]
            .fields
            .iter()
            .find(|(k, _)| k == "http.latency_ms");
        assert!(
            latency.is_some(),
            "http.latency_ms フィールドが存在すること"
        );
        let latency_value: u64 = latency.unwrap().1.parse().unwrap();
        assert!(latency_value < 1000, "レイテンシが妥当な範囲であること");
    }

    #[tokio::test]
    async fn test_healthパスではcanonical_log_lineが出力されない() {
        let (_guard, events) = setup_capture();

        let mut sut = CanonicalLogLineLayer.layer(DummyService {
            status: http::StatusCode::OK,
        });

        sut.call(build_request("/health")).await.unwrap();

        let captured = events.lock().unwrap();
        assert!(
            captured.is_empty(),
            "ヘルスチェックではログが出力されないこと"
        );
    }

    #[tokio::test]
    async fn test_health_readyパスではcanonical_log_lineが出力されない() {
        let (_guard, events) = setup_capture();

        let mut sut = CanonicalLogLineLayer.layer(DummyService {
            status: http::StatusCode::OK,
        });

        sut.call(build_request("/health/ready")).await.unwrap();

        let captured = events.lock().unwrap();
        assert!(
            captured.is_empty(),
            "/health/ready ではログが出力されないこと"
        );
    }

    #[tokio::test]
    async fn test_serviceエラー時にerrorレベルで出力される() {
        let (_guard, events) = setup_capture();

        let mut sut = CanonicalLogLineLayer.layer(ErrorService);

        let result = sut.call(build_request("/api/v1/test")).await;
        assert!(result.is_err());

        let captured = events.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].level, tracing::Level::ERROR);
        assert_eq!(captured[0].message, "リクエスト処理エラー");
    }

    #[tokio::test]
    async fn test_レスポンスが透過的に返される() {
        let (_guard, _events) = setup_capture();

        let mut sut = CanonicalLogLineLayer.layer(DummyService {
            status: http::StatusCode::NOT_FOUND,
        });

        let response = sut.call(build_request("/api/v1/test")).await.unwrap();
        assert_eq!(
            response.status(),
            http::StatusCode::NOT_FOUND,
            "元のステータスコードが保持されること"
        );
    }
}
