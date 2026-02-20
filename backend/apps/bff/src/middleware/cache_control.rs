//! # キャッシュ制御ミドルウェア
//!
//! 動的 API レスポンスがブラウザにキャッシュされないよう、
//! `Cache-Control: no-store` を全レスポンスに設定する。

use axum::{
    extract::Request,
    http::{HeaderValue, header},
    middleware::Next,
    response::Response,
};

/// API レスポンスに `Cache-Control: no-store` を付与する
pub async fn no_cache(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
