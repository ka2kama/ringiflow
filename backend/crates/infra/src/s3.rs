//! # S3 接続管理
//!
//! Amazon S3 / MinIO への接続管理と Presigned URL 生成を行う。
//!
//! ## 設計方針
//!
//! - **ローカル開発**: MinIO を使用（`S3_ENDPOINT_URL` で接続先を指定）
//! - **本番環境**: IAM ロールによる認証で Amazon S3 に接続（`S3_ENDPOINT_URL` 未設定）
//! - **Presigned URL**: ブラウザが S3 に直接 PUT/GET する方式（サーバーはURL発行のみ）
//!
//! ## 使用例
//!
//! ```rust,ignore
//! use ringiflow_infra::s3;
//! use std::time::Duration;
//!
//! async fn setup() -> Result<(), Box<dyn std::error::Error>> {
//!     // ローカル（MinIO）
//!     let client = s3::create_client(Some("http://localhost:19000")).await;
//!     let s3 = s3::AwsS3Client::new(client, "ringiflow-dev-documents".to_string());
//!
//!     // 本番（AWS S3）
//!     let client = s3::create_client(None).await;
//!     let s3 = s3::AwsS3Client::new(client, "ringiflow-prod-documents".to_string());
//!
//!     Ok(())
//! }
//! ```

use std::time::Duration;

use async_trait::async_trait;
use aws_sdk_s3::{Client, presigning::PresigningConfig};

use crate::InfraError;

/// S3 クライアントのインターフェース
///
/// Presigned URL の生成とオブジェクトの存在確認を提供する。
/// テスト時はモックに差し替え可能。
#[async_trait]
pub trait S3Client: Send + Sync {
    /// Presigned PUT URL を生成する（アップロード用）
    ///
    /// ブラウザがこの URL に対して HTTP PUT でファイルを直接アップロードする。
    ///
    /// # 引数
    ///
    /// * `s3_key` - S3 オブジェクトキー（例: `tenant-abc/workflows/019.../file.pdf`）
    /// * `content_type` - MIME タイプ（例: `application/pdf`）
    /// * `content_length` - ファイルサイズ（バイト）
    /// * `expires_in` - URL の有効期限
    async fn generate_presigned_put_url(
        &self,
        s3_key: &str,
        content_type: &str,
        content_length: i64,
        expires_in: Duration,
    ) -> Result<String, InfraError>;

    /// Presigned GET URL を生成する（ダウンロード用）
    ///
    /// ブラウザがこの URL に対して HTTP GET でファイルを直接ダウンロードする。
    ///
    /// # 引数
    ///
    /// * `s3_key` - S3 オブジェクトキー
    /// * `expires_in` - URL の有効期限
    async fn generate_presigned_get_url(
        &self,
        s3_key: &str,
        expires_in: Duration,
    ) -> Result<String, InfraError>;

    /// オブジェクトの存在を確認する（HEAD Object）
    ///
    /// アップロード完了通知時に、S3 にファイルが実際に存在するかを確認する。
    ///
    /// # 引数
    ///
    /// * `s3_key` - S3 オブジェクトキー
    ///
    /// # 戻り値
    ///
    /// オブジェクトが存在すれば `true`、存在しなければ `false`
    async fn head_object(&self, s3_key: &str) -> Result<bool, InfraError>;
}

/// AWS S3 クライアント
///
/// `aws-sdk-s3` を使用した [`S3Client`] の実装。
/// MinIO とも互換動作する。
pub struct AwsS3Client {
    client:      Client,
    bucket_name: String,
}

impl AwsS3Client {
    /// 新しい S3 クライアントを作成する
    pub fn new(client: Client, bucket_name: String) -> Self {
        Self {
            client,
            bucket_name,
        }
    }
}

#[async_trait]
impl S3Client for AwsS3Client {
    async fn generate_presigned_put_url(
        &self,
        s3_key: &str,
        content_type: &str,
        content_length: i64,
        expires_in: Duration,
    ) -> Result<String, InfraError> {
        let presign_config = PresigningConfig::expires_in(expires_in)
            .map_err(|e| InfraError::S3(format!("Presigned 設定の構築に失敗: {e}")))?;

        let presigned = self
            .client
            .put_object()
            .bucket(&self.bucket_name)
            .key(s3_key)
            .content_type(content_type)
            .content_length(content_length)
            .presigned(presign_config)
            .await
            .map_err(|e| InfraError::S3(format!("Presigned PUT URL の生成に失敗: {e}")))?;

        Ok(presigned.uri().to_string())
    }

    async fn generate_presigned_get_url(
        &self,
        s3_key: &str,
        expires_in: Duration,
    ) -> Result<String, InfraError> {
        let presign_config = PresigningConfig::expires_in(expires_in)
            .map_err(|e| InfraError::S3(format!("Presigned 設定の構築に失敗: {e}")))?;

        let presigned = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(s3_key)
            .presigned(presign_config)
            .await
            .map_err(|e| InfraError::S3(format!("Presigned GET URL の生成に失敗: {e}")))?;

        Ok(presigned.uri().to_string())
    }

    async fn head_object(&self, s3_key: &str) -> Result<bool, InfraError> {
        let result = self
            .client
            .head_object()
            .bucket(&self.bucket_name)
            .key(s3_key)
            .send()
            .await;

        match result {
            Ok(_) => Ok(true),
            Err(err) => {
                // NotFound（404）の場合は false を返す
                let is_not_found = err
                    .as_service_error()
                    .map(|e| e.is_not_found())
                    .unwrap_or(false);
                if is_not_found {
                    Ok(false)
                } else {
                    Err(InfraError::S3(format!("HEAD Object の実行に失敗: {err}")))
                }
            }
        }
    }
}

/// S3 クライアントを作成する
///
/// `endpoint` が `Some` の場合は MinIO 等のカスタムエンドポイントに接続する。
/// `None` の場合は AWS S3 のデフォルトエンドポイントを使用する。
///
/// 認証情報は SDK のデフォルト認証チェーンで解決する:
/// - ローカル: 環境変数 `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY`（`.env` で設定）
/// - 本番: IAM ロール
///
/// # 引数
///
/// * `endpoint` - カスタムエンドポイント URL（例: `http://localhost:19000`）。
///   `None` の場合は AWS S3 のデフォルトエンドポイントを使用する。
///
/// # 戻り値
///
/// S3 クライアント
pub async fn create_client(endpoint: Option<&str>) -> Client {
    let mut config_builder = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("ap-northeast-1"));

    if let Some(endpoint_url) = endpoint {
        config_builder = config_builder.endpoint_url(endpoint_url);
    }

    let config = config_builder.load().await;

    // MinIO はパススタイルが必要（バーチャルホスト型 URL を使わない）
    // エンドポイント指定時のみ force_path_style を有効化
    let s3_config_builder = aws_sdk_s3::config::Builder::from(&config);
    let s3_config = if endpoint.is_some() {
        s3_config_builder.force_path_style(true).build()
    } else {
        s3_config_builder.build()
    };

    Client::from_conf(s3_config)
}
