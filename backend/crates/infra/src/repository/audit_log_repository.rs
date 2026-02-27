//! # AuditLogRepository
//!
//! 監査ログの永続化を担当するリポジトリ。
//!
//! ## 設計方針
//!
//! - **DynamoDB**: 監査ログは DynamoDB に格納（PostgreSQL ではない）
//! - **テナント分離**: PK = tenant_id で論理分離
//! - **時系列ソート**: SK = `{timestamp}#{uuid}` でレキシカル順ソート
//! - **カーソルページネーション**: DynamoDB の `LastEvaluatedKey` を base64
//!   でエンコード

use std::collections::HashMap;

use async_trait::async_trait;
use aws_sdk_dynamodb::{Client, types::AttributeValue};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use chrono::{DateTime, Utc};
use ringiflow_domain::{
    audit_log::{AuditAction, AuditLog, AuditResult},
    tenant::TenantId,
    user::UserId,
};

use crate::InfraError;

/// 監査ログのフィルタ条件
#[derive(Debug, Default)]
pub struct AuditLogFilter {
    pub from:     Option<DateTime<Utc>>,
    pub to:       Option<DateTime<Utc>>,
    pub actor_id: Option<UserId>,
    pub actions:  Option<Vec<AuditAction>>,
    pub result:   Option<AuditResult>,
}

/// 監査ログのページ
#[derive(Debug)]
pub struct AuditLogPage {
    pub items:       Vec<AuditLog>,
    pub next_cursor: Option<String>,
}

/// 監査ログリポジトリトレイト
#[async_trait]
pub trait AuditLogRepository: Send + Sync {
    /// 監査ログを記録する
    async fn record(&self, log: &AuditLog) -> Result<(), InfraError>;

    /// テナントの監査ログを検索する（新しい順）
    async fn find_by_tenant(
        &self,
        tenant_id: &TenantId,
        cursor: Option<&str>,
        limit: i32,
        filter: &AuditLogFilter,
    ) -> Result<AuditLogPage, InfraError>;
}

/// DynamoDB 実装の AuditLogRepository
pub struct DynamoDbAuditLogRepository {
    client:     Client,
    table_name: String,
}

impl DynamoDbAuditLogRepository {
    pub fn new(client: Client, table_name: String) -> Self {
        Self { client, table_name }
    }
}

#[async_trait]
impl AuditLogRepository for DynamoDbAuditLogRepository {
    #[tracing::instrument(skip_all, level = "debug")]
    async fn record(&self, log: &AuditLog) -> Result<(), InfraError> {
        let sk = log.sort_key();

        let mut item = HashMap::new();
        item.insert(
            "tenant_id".to_string(),
            AttributeValue::S(log.tenant_id.as_uuid().to_string()),
        );
        item.insert("sk".to_string(), AttributeValue::S(sk));
        item.insert(
            "actor_id".to_string(),
            AttributeValue::S(log.actor_id.as_uuid().to_string()),
        );
        item.insert(
            "actor_name".to_string(),
            AttributeValue::S(log.actor_name.clone()),
        );
        item.insert(
            "action".to_string(),
            AttributeValue::S(log.action.to_string()),
        );
        item.insert(
            "result".to_string(),
            AttributeValue::S(log.result.to_string()),
        );
        item.insert(
            "resource_type".to_string(),
            AttributeValue::S(log.resource_type.clone()),
        );
        item.insert(
            "resource_id".to_string(),
            AttributeValue::S(log.resource_id.clone()),
        );

        if let Some(detail) = &log.detail {
            item.insert(
                "detail".to_string(),
                AttributeValue::S(
                    serde_json::to_string(detail).map_err(InfraError::Serialization)?,
                ),
            );
        }

        if let Some(source_ip) = &log.source_ip {
            item.insert(
                "source_ip".to_string(),
                AttributeValue::S(source_ip.clone()),
            );
        }

        item.insert("ttl".to_string(), AttributeValue::N(log.ttl.to_string()));

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await
            .map_err(|e| InfraError::DynamoDb(format!("監査ログの記録に失敗: {e}")))?;

        Ok(())
    }

    #[tracing::instrument(skip_all, level = "debug", fields(%tenant_id))]
    async fn find_by_tenant(
        &self,
        tenant_id: &TenantId,
        cursor: Option<&str>,
        limit: i32,
        filter: &AuditLogFilter,
    ) -> Result<AuditLogPage, InfraError> {
        let mut query = self
            .client
            .query()
            .table_name(&self.table_name)
            .scan_index_forward(false) // 新しい順
            .limit(limit);

        // Key Condition: tenant_id = :tid
        // 日付範囲がある場合は SK も条件に含める
        let mut key_condition = "tenant_id = :tid".to_string();
        let mut expr_attr_values: HashMap<String, AttributeValue> = HashMap::new();
        expr_attr_values.insert(
            ":tid".to_string(),
            AttributeValue::S(tenant_id.as_uuid().to_string()),
        );

        // 日付範囲フィルタ（SK の KeyConditionExpression で実現）
        match (&filter.from, &filter.to) {
            (Some(from), Some(to)) => {
                key_condition.push_str(" AND sk BETWEEN :sk_from AND :sk_to");
                expr_attr_values
                    .insert(":sk_from".to_string(), AttributeValue::S(from.to_rfc3339()));
                // to の末尾に ~ を付与して、同一タイムスタンプの全エントリを含める
                expr_attr_values.insert(
                    ":sk_to".to_string(),
                    AttributeValue::S(format!("{}~", to.to_rfc3339())),
                );
            }
            (Some(from), None) => {
                key_condition.push_str(" AND sk >= :sk_from");
                expr_attr_values
                    .insert(":sk_from".to_string(), AttributeValue::S(from.to_rfc3339()));
            }
            (None, Some(to)) => {
                key_condition.push_str(" AND sk <= :sk_to");
                expr_attr_values.insert(
                    ":sk_to".to_string(),
                    AttributeValue::S(format!("{}~", to.to_rfc3339())),
                );
            }
            (None, None) => {}
        }

        query = query.key_condition_expression(&key_condition);

        // FilterExpression（SK 以外のフィルタ）
        let mut filter_parts: Vec<String> = Vec::new();

        if let Some(actor_id) = &filter.actor_id {
            filter_parts.push("actor_id = :actor_id".to_string());
            expr_attr_values.insert(
                ":actor_id".to_string(),
                AttributeValue::S(actor_id.as_uuid().to_string()),
            );
        }

        if let Some(actions) = &filter.actions
            && !actions.is_empty()
        {
            let placeholders: Vec<String> = actions
                .iter()
                .enumerate()
                .map(|(i, action)| {
                    let key = format!(":action_{i}");
                    expr_attr_values.insert(key.clone(), AttributeValue::S(action.to_string()));
                    key
                })
                .collect();
            // "action" は DynamoDB の予約語のため、ExpressionAttributeNames で回避
            filter_parts.push(format!("#action_attr IN ({})", placeholders.join(", ")));
            query = query.expression_attribute_names("#action_attr", "action");
        }

        if let Some(result) = &filter.result {
            filter_parts.push("#result_attr = :result_val".to_string());
            expr_attr_values.insert(
                ":result_val".to_string(),
                AttributeValue::S(result.to_string()),
            );
            // "result" は DynamoDB の予約語のため、ExpressionAttributeNames で回避
            query = query.expression_attribute_names("#result_attr", "result");
        }

        if !filter_parts.is_empty() {
            query = query.filter_expression(filter_parts.join(" AND "));
        }

        // Expression attribute values を設定
        for (k, v) in &expr_attr_values {
            query = query.expression_attribute_values(k, v.clone());
        }

        // カーソル（前ページの LastEvaluatedKey を base64 デコード）
        // AttributeValue は Serialize/Deserialize 非対応のため、
        // HashMap<String, String> に変換してシリアライズする
        if let Some(cursor_str) = cursor {
            let decoded = BASE64
                .decode(cursor_str)
                .map_err(|e| InfraError::InvalidInput(format!("カーソルのデコードに失敗: {e}")))?;
            let key_strings: HashMap<String, String> =
                serde_json::from_slice(&decoded).map_err(|e| {
                    InfraError::InvalidInput(format!("カーソルのデシリアライズに失敗: {e}"))
                })?;
            let last_key: HashMap<String, AttributeValue> = key_strings
                .into_iter()
                .map(|(k, v)| (k, AttributeValue::S(v)))
                .collect();
            query = query.set_exclusive_start_key(Some(last_key));
        }

        let output = query
            .send()
            .await
            .map_err(|e| InfraError::DynamoDb(format!("監査ログの検索に失敗: {e}")))?;

        // レスポンスをドメインモデルに変換
        let items = output
            .items()
            .iter()
            .filter_map(|item| convert_item_to_audit_log(item).ok())
            .collect();

        // 次ページのカーソル
        // AttributeValue → HashMap<String, String> に変換してからシリアライズ
        let next_cursor = output.last_evaluated_key().map(|key| {
            let key_strings: HashMap<String, String> = key
                .iter()
                .filter_map(|(k, v)| v.as_s().ok().map(|s| (k.clone(), s.clone())))
                .collect();
            let json = serde_json::to_vec(&key_strings).unwrap_or_default();
            BASE64.encode(json)
        });

        Ok(AuditLogPage { items, next_cursor })
    }
}

/// DynamoDB アイテムを AuditLog に変換する
fn convert_item_to_audit_log(
    item: &HashMap<String, AttributeValue>,
) -> Result<AuditLog, InfraError> {
    let tenant_id_str = get_s(item, "tenant_id")?;
    let sk = get_s(item, "sk")?;
    let actor_id_str = get_s(item, "actor_id")?;
    let actor_name = get_s(item, "actor_name")?;
    let action_str = get_s(item, "action")?;
    let result_str = get_s(item, "result")?;
    let resource_type = get_s(item, "resource_type")?;
    let resource_id = get_s(item, "resource_id")?;
    let ttl_str = get_n(item, "ttl")?;

    let detail = item
        .get("detail")
        .and_then(|v| v.as_s().ok())
        .map(|s| serde_json::from_str(s))
        .transpose()
        .map_err(InfraError::Serialization)?;

    let source_ip = item.get("source_ip").and_then(|v| v.as_s().ok()).cloned();

    let tenant_id = TenantId::from_uuid(
        uuid::Uuid::parse_str(&tenant_id_str)
            .map_err(|e| InfraError::DynamoDb(format!("tenant_id のパースに失敗: {e}")))?,
    );
    let actor_id = UserId::from_uuid(
        uuid::Uuid::parse_str(&actor_id_str)
            .map_err(|e| InfraError::DynamoDb(format!("actor_id のパースに失敗: {e}")))?,
    );
    let action: AuditAction = action_str
        .parse()
        .map_err(|e: String| InfraError::DynamoDb(e))?;
    let result: AuditResult = result_str
        .parse()
        .map_err(|e: String| InfraError::DynamoDb(e))?;
    let ttl: i64 = ttl_str
        .parse()
        .map_err(|e| InfraError::DynamoDb(format!("ttl のパースに失敗: {e}")))?;

    AuditLog::from_stored(
        tenant_id,
        &sk,
        actor_id,
        actor_name,
        action,
        result,
        resource_type,
        resource_id,
        detail,
        source_ip,
        ttl,
    )
    .map_err(InfraError::DynamoDb)
}

/// DynamoDB アイテムから文字列属性を取得する
fn get_s(item: &HashMap<String, AttributeValue>, key: &str) -> Result<String, InfraError> {
    item.get(key)
        .and_then(|v| v.as_s().ok())
        .cloned()
        .ok_or_else(|| InfraError::DynamoDb(format!("属性 '{key}' が見つかりません")))
}

/// DynamoDB アイテムから数値属性を取得する
fn get_n(item: &HashMap<String, AttributeValue>, key: &str) -> Result<String, InfraError> {
    item.get(key)
        .and_then(|v| v.as_n().ok())
        .cloned()
        .ok_or_else(|| InfraError::DynamoDb(format!("数値属性 '{key}' が見つかりません")))
}
