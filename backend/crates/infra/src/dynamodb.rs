//! # DynamoDB 接続管理
//!
//! Amazon DynamoDB への接続管理を行う。
//!
//! ## 設計方針
//!
//! - **ローカル開発**: DynamoDB Local を使用（`-sharedDb -inMemory`）
//! - **本番環境**: IAM ロールによる認証で Amazon DynamoDB に接続
//! - **テーブル自動作成**: アプリケーション起動時にテーブルが存在しなければ作成（冪等）
//!
//! ## DynamoDB の用途
//!
//! RingiFlow では DynamoDB を以下の目的で使用する:
//!
//! - **監査ログ**: ユーザー操作・ロール操作の記録と閲覧
//!
//! ## 使用例
//!
//! ```rust,ignore
//! use ringiflow_infra::dynamodb;
//!
//! async fn setup() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = dynamodb::create_client("http://localhost:18000").await;
//!     dynamodb::ensure_audit_log_table(&client, "audit_logs").await?;
//!     Ok(())
//! }
//! ```

use aws_sdk_dynamodb::{
    Client,
    types::{
        AttributeDefinition,
        BillingMode,
        KeySchemaElement,
        KeyType,
        ScalarAttributeType,
        TimeToLiveSpecification,
    },
};

use crate::InfraError;

/// DynamoDB クライアントを作成する
///
/// DynamoDB Local 用のクライアントを作成する。認証情報はダミー値を使用する
/// （DynamoDB Local の `-sharedDb` モードでは認証情報を検証しない）。
///
/// 本番環境では IAM ロール等の認証情報プロバイダを使用するため、
/// この関数を拡張するか、別のファクトリ関数を用意する。
///
/// # 引数
///
/// * `endpoint` - DynamoDB エンドポイント URL（例: `http://localhost:18000`）
///
/// # 戻り値
///
/// DynamoDB クライアント
pub async fn create_client(endpoint: &str) -> Client {
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .endpoint_url(endpoint)
        .region(aws_config::Region::new("ap-northeast-1"))
        // DynamoDB Local はクレデンシャルを検証しないが、SDK はプロバイダが必要
        .credentials_provider(aws_sdk_dynamodb::config::Credentials::new(
            "local", "local", None, None, "local",
        ))
        .load()
        .await;

    Client::new(&config)
}

/// 監査ログテーブルが存在しなければ作成する（冪等）
///
/// テーブルスキーマ:
/// - PK: `tenant_id` (String) — テナント ID
/// - SK: `sk` (String) — `{ISO8601_timestamp}#{uuid}` 形式
/// - TTL: `ttl` 属性で自動削除（created_at + 1年）
///
/// # 引数
///
/// * `client` - DynamoDB クライアント
/// * `table_name` - テーブル名
pub async fn ensure_audit_log_table(client: &Client, table_name: &str) -> Result<(), InfraError> {
    // テーブルの存在確認
    match client.describe_table().table_name(table_name).send().await {
        Ok(_) => {
            tracing::debug!("テーブル '{}' は既に存在します", table_name);
            return Ok(());
        }
        Err(err) => {
            // ResourceNotFoundException の場合のみテーブル作成に進む
            let service_err = err.as_service_error();
            if !service_err
                .map(|e| e.is_resource_not_found_exception())
                .unwrap_or(false)
            {
                return Err(InfraError::DynamoDb(format!(
                    "テーブル '{}' の確認に失敗: {}",
                    table_name, err
                )));
            }
        }
    }

    // テーブル作成
    tracing::info!("テーブル '{}' を作成します", table_name);

    let create_result = client
        .create_table()
        .table_name(table_name)
        .key_schema(
            KeySchemaElement::builder()
                .attribute_name("tenant_id")
                .key_type(KeyType::Hash)
                .build()
                .map_err(|e| InfraError::DynamoDb(format!("KeySchema 構築エラー: {}", e)))?,
        )
        .key_schema(
            KeySchemaElement::builder()
                .attribute_name("sk")
                .key_type(KeyType::Range)
                .build()
                .map_err(|e| InfraError::DynamoDb(format!("KeySchema 構築エラー: {}", e)))?,
        )
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("tenant_id")
                .attribute_type(ScalarAttributeType::S)
                .build()
                .map_err(|e| {
                    InfraError::DynamoDb(format!("AttributeDefinition 構築エラー: {}", e))
                })?,
        )
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("sk")
                .attribute_type(ScalarAttributeType::S)
                .build()
                .map_err(|e| {
                    InfraError::DynamoDb(format!("AttributeDefinition 構築エラー: {}", e))
                })?,
        )
        .billing_mode(BillingMode::PayPerRequest)
        .send()
        .await;

    match create_result {
        Ok(_) => {}
        Err(err) => {
            // ResourceInUseException は並行呼び出し時に発生しうる（テーブルが作成中）
            // この場合は冪等として成功扱いにする
            let is_resource_in_use = err
                .as_service_error()
                .map(|e| e.is_resource_in_use_exception())
                .unwrap_or(false);
            if !is_resource_in_use {
                return Err(InfraError::DynamoDb(format!(
                    "テーブル '{}' の作成に失敗: {}",
                    table_name, err
                )));
            }
            tracing::debug!(
                "テーブル '{}' は既に作成中または存在します（ResourceInUseException）",
                table_name
            );
            return Ok(());
        }
    }

    // TTL 設定
    client
        .update_time_to_live()
        .table_name(table_name)
        .time_to_live_specification(
            TimeToLiveSpecification::builder()
                .enabled(true)
                .attribute_name("ttl")
                .build()
                .map_err(|e| InfraError::DynamoDb(format!("TTL 設定の構築に失敗: {}", e)))?,
        )
        .send()
        .await
        .map_err(|e| {
            InfraError::DynamoDb(format!(
                "テーブル '{}' の TTL 設定に失敗: {}",
                table_name, e
            ))
        })?;

    tracing::info!("テーブル '{}' を作成しました", table_name);

    Ok(())
}
