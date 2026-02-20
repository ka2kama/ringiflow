//! # ヘルスチェック共通型
//!
//! 全サービス（BFF / Core Service / Auth Service）のヘルスチェックエンドポイントで
//! 使用される共通レスポンス型を提供する。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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

/// 個別チェックの結果ステータス
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum CheckStatus {
    /// チェック成功
    Ok,
    /// チェック失敗
    Error,
}

/// Readiness 全体のステータス
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum ReadinessStatus {
    /// 全依存サービスが利用可能
    Ready,
    /// 一部の依存サービスが利用不可
    NotReady,
}

/// Readiness Check レスポンス
///
/// 依存サービスへの接続状態を含むレスポンス型。
/// `status` は全体のステータス、`checks` は個別チェック結果を示す。
///
/// ## 使用例
///
/// ```
/// use std::collections::HashMap;
///
/// use ringiflow_shared::{CheckStatus, ReadinessResponse, ReadinessStatus};
///
/// let mut checks = HashMap::new();
/// checks.insert("database".to_string(), CheckStatus::Ok);
/// let response = ReadinessResponse {
///     status: ReadinessStatus::Ready,
///     checks,
/// };
/// assert_eq!(response.status, ReadinessStatus::Ready);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ReadinessResponse {
    /// 全体のステータス
    pub status: ReadinessStatus,
    /// 個別チェック結果（キー: チェック名、値: ステータス）
    pub checks: HashMap<String, CheckStatus>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_responseのserializeで正しいjson形状にする() {
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

    #[test]
    fn test_check_status_okのserialize結果() {
        let json = serde_json::to_value(CheckStatus::Ok).unwrap();
        assert_eq!(json, serde_json::json!("ok"));
    }

    #[test]
    fn test_check_status_errorのserialize結果() {
        let json = serde_json::to_value(CheckStatus::Error).unwrap();
        assert_eq!(json, serde_json::json!("error"));
    }

    #[test]
    fn test_readiness_status_readyのserialize結果() {
        let json = serde_json::to_value(ReadinessStatus::Ready).unwrap();
        assert_eq!(json, serde_json::json!("ready"));
    }

    #[test]
    fn test_readiness_status_not_readyのserialize結果() {
        let json = serde_json::to_value(ReadinessStatus::NotReady).unwrap();
        assert_eq!(json, serde_json::json!("not_ready"));
    }

    #[test]
    fn test_readiness_response_readyのserialize結果() {
        let mut checks = HashMap::new();
        checks.insert("database".to_string(), CheckStatus::Ok);
        let response = ReadinessResponse {
            status: ReadinessStatus::Ready,
            checks,
        };
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["status"], "ready");
        assert_eq!(json["checks"]["database"], "ok");
    }

    #[test]
    fn test_readiness_response_not_readyのserialize結果() {
        let mut checks = HashMap::new();
        checks.insert("database".to_string(), CheckStatus::Error);
        let response = ReadinessResponse {
            status: ReadinessStatus::NotReady,
            checks,
        };
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["status"], "not_ready");
        assert_eq!(json["checks"]["database"], "error");
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

    #[test]
    fn test_readiness_responseにtoschemaが実装されている() {
        let schema = ReadinessResponse::schema();
        let utoipa::openapi::RefOr::T(schema) = schema else {
            panic!("expected inline schema, got ref");
        };
        let utoipa::openapi::Schema::Object(obj) = schema else {
            panic!("expected object schema");
        };
        // status と checks フィールドがスキーマに含まれていること
        assert!(obj.properties.contains_key("status"));
        assert!(obj.properties.contains_key("checks"));
    }
}
