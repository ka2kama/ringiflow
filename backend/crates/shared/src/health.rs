//! # ヘルスチェック共通型
//!
//! 全サービス（BFF / Core Service / Auth Service）のヘルスチェックエンドポイントで
//! 使用される共通レスポンス型を提供する。

use serde::Serialize;

/// ヘルスチェックレスポンス
///
/// 各サービスのヘルスチェックエンドポイントが返すレスポンス型。
/// `status` はサービスの稼働状態、`version` は Cargo.toml のバージョンを示す。
///
/// ## 使用例
///
/// ```
/// use ringiflow_shared::HealthResponse;
///
/// let response = HealthResponse {
///     status:  "healthy".to_string(),
///     version: "0.1.0".to_string(),
/// };
/// assert_eq!(response.status, "healthy");
/// ```
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct HealthResponse {
    /// 稼働状態（`"healthy"` または `"unhealthy"`）
    pub status:  String,
    /// アプリケーションバージョン（Cargo.toml から取得）
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serializeで正しいjson形状にする() {
        let response = HealthResponse {
            status:  "healthy".to_string(),
            version: "0.1.0".to_string(),
        };
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(
            json,
            serde_json::json!({
                "status": "healthy",
                "version": "0.1.0"
            })
        );
    }
}

#[cfg(all(test, feature = "openapi"))]
mod openapi_tests {
    use utoipa::PartialSchema;

    use super::*;

    #[test]
    fn test_health_responseにtoschemaが実装されている() {
        let schema = HealthResponse::schema();
        let utoipa::openapi::RefOr::T(schema) = schema else {
            panic!("expected inline schema, got ref");
        };
        let utoipa::openapi::Schema::Object(obj) = schema else {
            panic!("expected object schema");
        };
        // status と version フィールドがスキーマに含まれていること
        assert!(obj.properties.contains_key("status"));
        assert!(obj.properties.contains_key("version"));
    }
}
