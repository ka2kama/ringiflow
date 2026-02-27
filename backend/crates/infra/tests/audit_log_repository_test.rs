//! AuditLogRepository の統合テスト
//!
//! DynamoDB Local を使用した統合テスト。
//! テスト毎にランダムな `TenantId` を生成し、テナントベースで分離する。
//!
//! 実行方法:
//! ```bash
//! just dev-deps
//! cd backend && cargo test -p ringiflow-infra --test audit_log_repository_test
//! ```

use chrono::{Duration, Utc};
use ringiflow_domain::{
    audit_log::{AuditAction, AuditLog},
    tenant::TenantId,
    user::UserId,
};
use ringiflow_infra::{
    dynamodb,
    repository::audit_log_repository::{
        AuditLogFilter,
        AuditLogRepository,
        DynamoDbAuditLogRepository,
    },
};
use tokio::sync::OnceCell;

/// テスト用の DynamoDB エンドポイント
fn dynamodb_endpoint() -> String {
    std::env::var("DYNAMODB_ENDPOINT").unwrap_or_else(|_| {
        let port = std::env::var("DYNAMODB_PORT").unwrap_or_else(|_| "18000".to_string());
        format!("http://localhost:{port}")
    })
}

/// テスト用テーブル名（全テスト共有、テナント ID で分離）
const TEST_TABLE_NAME: &str = "test_audit_logs";

/// テーブルセットアップの一度だけ実行を保証する
///
/// DynamoDB クライアントは各テストで独立に作成する。
/// 単一クライアントの共有は内部コネクションプールがボトルネックになるため避ける。
static TABLE_INITIALIZED: OnceCell<()> = OnceCell::const_new();

/// テスト用のリポジトリをセットアップする
async fn setup() -> DynamoDbAuditLogRepository {
    let client = dynamodb::create_client(&dynamodb_endpoint()).await;
    TABLE_INITIALIZED
        .get_or_init(|| {
            let client = &client;
            async move {
                dynamodb::ensure_audit_log_table(client, TEST_TABLE_NAME)
                    .await
                    .expect("テーブルのセットアップに失敗");
            }
        })
        .await;
    DynamoDbAuditLogRepository::new(client, TEST_TABLE_NAME.to_string())
}

/// テスト用の監査ログを作成する
fn create_test_log(tenant_id: &TenantId, action: AuditAction) -> AuditLog {
    AuditLog::new_success(
        tenant_id.clone(),
        UserId::new(),
        "Test User".to_string(),
        action,
        "user",
        uuid::Uuid::now_v7().to_string(),
        None,
        None,
    )
}

#[tokio::test]
async fn test_recordが監査ログをdynamodbに書き込める() {
    let repo = setup().await;
    let tenant_id = TenantId::new();
    let log = create_test_log(&tenant_id, AuditAction::UserCreate);

    let result = repo.record(&log).await;
    assert!(result.is_ok(), "監査ログの記録に失敗: {:?}", result.err());

    // 書き込んだログが検索できることを確認
    let page = repo
        .find_by_tenant(&tenant_id, None, 10, &AuditLogFilter::default())
        .await
        .expect("検索に失敗");

    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id, log.id);
    assert_eq!(page.items[0].action, AuditAction::UserCreate);
}

#[tokio::test]
async fn test_find_by_tenantがテナントidで検索でき新しい順に返る() {
    let repo = setup().await;
    let tenant_id = TenantId::new();

    // 3件のログを順番に作成（タイムスタンプの順序を保証するため少し待つ）
    let log1 = create_test_log(&tenant_id, AuditAction::UserCreate);
    repo.record(&log1).await.unwrap();

    let log2 = create_test_log(&tenant_id, AuditAction::UserUpdate);
    repo.record(&log2).await.unwrap();

    let log3 = create_test_log(&tenant_id, AuditAction::RoleCreate);
    repo.record(&log3).await.unwrap();

    let page = repo
        .find_by_tenant(&tenant_id, None, 10, &AuditLogFilter::default())
        .await
        .expect("検索に失敗");

    assert_eq!(page.items.len(), 3);
    // 新しい順（log3 → log2 → log1）
    assert_eq!(page.items[0].id, log3.id);
    assert_eq!(page.items[1].id, log2.id);
    assert_eq!(page.items[2].id, log1.id);
}

#[tokio::test]
async fn test_find_by_tenantがカーソルベースページネーションで正しく動作する() {
    let repo = setup().await;
    let tenant_id = TenantId::new();

    // 5件のログを作成
    let mut logs = Vec::new();
    for _ in 0..5 {
        let log = create_test_log(&tenant_id, AuditAction::UserCreate);
        repo.record(&log).await.unwrap();
        logs.push(log);
    }

    // 1ページ目: 2件取得
    let page1 = repo
        .find_by_tenant(&tenant_id, None, 2, &AuditLogFilter::default())
        .await
        .expect("1ページ目の検索に失敗");

    assert_eq!(page1.items.len(), 2);
    assert!(
        page1.next_cursor.is_some(),
        "次ページのカーソルが存在するべき"
    );

    // 2ページ目: カーソルを使って次の2件取得
    let page2 = repo
        .find_by_tenant(
            &tenant_id,
            page1.next_cursor.as_deref(),
            2,
            &AuditLogFilter::default(),
        )
        .await
        .expect("2ページ目の検索に失敗");

    assert_eq!(page2.items.len(), 2);
    assert!(
        page2.next_cursor.is_some(),
        "次ページのカーソルが存在するべき"
    );

    // 3ページ目: 残り1件
    let page3 = repo
        .find_by_tenant(
            &tenant_id,
            page2.next_cursor.as_deref(),
            2,
            &AuditLogFilter::default(),
        )
        .await
        .expect("3ページ目の検索に失敗");

    assert_eq!(page3.items.len(), 1);
    assert!(
        page3.next_cursor.is_none(),
        "最後のページではカーソルが null であるべき"
    );

    // 全ページの ID が重複しないことを確認
    let all_ids: Vec<_> = page1
        .items
        .iter()
        .chain(page2.items.iter())
        .chain(page3.items.iter())
        .map(|l| l.id)
        .collect();
    assert_eq!(all_ids.len(), 5);
    let unique_ids: std::collections::HashSet<_> = all_ids.iter().collect();
    assert_eq!(unique_ids.len(), 5, "全ページの ID が一意であるべき");
}

#[tokio::test]
async fn test_find_by_tenantが日付範囲フィルタで絞り込める() {
    let repo = setup().await;
    let tenant_id = TenantId::new();

    // 現在時刻を基準にログを作成
    let log = create_test_log(&tenant_id, AuditAction::UserCreate);
    repo.record(&log).await.unwrap();

    // 過去のフィルタ → ヒットする
    let filter = AuditLogFilter {
        from: Some(Utc::now() - Duration::hours(1)),
        to: Some(Utc::now() + Duration::hours(1)),
        ..Default::default()
    };
    let page = repo
        .find_by_tenant(&tenant_id, None, 10, &filter)
        .await
        .expect("検索に失敗");
    assert_eq!(page.items.len(), 1, "日付範囲内のログがヒットするべき");

    // 未来のフィルタ → ヒットしない
    let filter_future = AuditLogFilter {
        from: Some(Utc::now() + Duration::hours(1)),
        to: Some(Utc::now() + Duration::hours(2)),
        ..Default::default()
    };
    let page_empty = repo
        .find_by_tenant(&tenant_id, None, 10, &filter_future)
        .await
        .expect("検索に失敗");
    assert_eq!(
        page_empty.items.len(),
        0,
        "日付範囲外のログはヒットしないべき"
    );
}

#[tokio::test]
async fn test_find_by_tenantがactor_idフィルタで絞り込める() {
    let repo = setup().await;
    let tenant_id = TenantId::new();
    let target_actor_id = UserId::new();

    // ターゲットの actor_id でログを作成
    let log1 = AuditLog::new_success(
        tenant_id.clone(),
        target_actor_id.clone(),
        "Target User".to_string(),
        AuditAction::UserCreate,
        "user",
        uuid::Uuid::now_v7().to_string(),
        None,
        None,
    );
    repo.record(&log1).await.unwrap();

    // 別の actor_id でログを作成
    let log2 = create_test_log(&tenant_id, AuditAction::UserUpdate);
    repo.record(&log2).await.unwrap();

    // actor_id フィルタ
    let filter = AuditLogFilter {
        actor_id: Some(target_actor_id),
        ..Default::default()
    };
    let page = repo
        .find_by_tenant(&tenant_id, None, 10, &filter)
        .await
        .expect("検索に失敗");

    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id, log1.id);
}

#[tokio::test]
async fn test_find_by_tenantがactionsフィルタで絞り込める() {
    let repo = setup().await;
    let tenant_id = TenantId::new();

    let log1 = create_test_log(&tenant_id, AuditAction::UserCreate);
    repo.record(&log1).await.unwrap();

    let log2 = create_test_log(&tenant_id, AuditAction::UserUpdate);
    repo.record(&log2).await.unwrap();

    let log3 = create_test_log(&tenant_id, AuditAction::RoleCreate);
    repo.record(&log3).await.unwrap();

    // UserCreate と RoleCreate でフィルタ
    let filter = AuditLogFilter {
        actions: Some(vec![AuditAction::UserCreate, AuditAction::RoleCreate]),
        ..Default::default()
    };
    let page = repo
        .find_by_tenant(&tenant_id, None, 10, &filter)
        .await
        .expect("検索に失敗");

    assert_eq!(page.items.len(), 2);
    // 新しい順なので log3(RoleCreate), log1(UserCreate)
    assert_eq!(page.items[0].action, AuditAction::RoleCreate);
    assert_eq!(page.items[1].action, AuditAction::UserCreate);
}

#[tokio::test]
async fn test_find_by_tenantがresultフィルタで絞り込める() {
    let repo = setup().await;
    let tenant_id = TenantId::new();

    // new_success で作成されるログは全て Success
    let log = create_test_log(&tenant_id, AuditAction::UserCreate);
    repo.record(&log).await.unwrap();

    // Success フィルタ → ヒットする
    let filter_success = AuditLogFilter {
        result: Some(ringiflow_domain::audit_log::AuditResult::Success),
        ..Default::default()
    };
    let page = repo
        .find_by_tenant(&tenant_id, None, 10, &filter_success)
        .await
        .expect("検索に失敗");
    assert_eq!(page.items.len(), 1, "Success フィルタでヒットするべき");

    // Failure フィルタ → ヒットしない
    let filter_failure = AuditLogFilter {
        result: Some(ringiflow_domain::audit_log::AuditResult::Failure),
        ..Default::default()
    };
    let page_empty = repo
        .find_by_tenant(&tenant_id, None, 10, &filter_failure)
        .await
        .expect("検索に失敗");
    assert_eq!(
        page_empty.items.len(),
        0,
        "Failure フィルタではヒットしないべき"
    );
}

#[tokio::test]
async fn test_異なるテナントの監査ログが分離されている() {
    let repo = setup().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();

    // テナント A のログ
    let log_a = create_test_log(&tenant_a, AuditAction::UserCreate);
    repo.record(&log_a).await.unwrap();

    // テナント B のログ
    let log_b = create_test_log(&tenant_b, AuditAction::RoleCreate);
    repo.record(&log_b).await.unwrap();

    // テナント A の検索
    let page_a = repo
        .find_by_tenant(&tenant_a, None, 10, &AuditLogFilter::default())
        .await
        .expect("検索に失敗");
    assert_eq!(page_a.items.len(), 1);
    assert_eq!(page_a.items[0].id, log_a.id);

    // テナント B の検索
    let page_b = repo
        .find_by_tenant(&tenant_b, None, 10, &AuditLogFilter::default())
        .await
        .expect("検索に失敗");
    assert_eq!(page_b.items.len(), 1);
    assert_eq!(page_b.items[0].id, log_b.id);
}

#[tokio::test]
async fn test_recordがdetailとsource_ipを正しく保存する() {
    let repo = setup().await;
    let tenant_id = TenantId::new();

    let detail = serde_json::json!({
       "email": "test@example.com",
       "name": "Test User",
       "role": "tenant_admin"
    });

    let log = AuditLog::new_success(
        tenant_id.clone(),
        UserId::new(),
        "Admin User".to_string(),
        AuditAction::UserCreate,
        "user",
        uuid::Uuid::now_v7().to_string(),
        Some(detail.clone()),
        Some("192.168.1.1".to_string()),
    );
    repo.record(&log).await.unwrap();

    let page = repo
        .find_by_tenant(&tenant_id, None, 10, &AuditLogFilter::default())
        .await
        .expect("検索に失敗");

    assert_eq!(page.items.len(), 1);
    let item = &page.items[0];
    assert_eq!(item.detail, Some(detail));
    assert_eq!(item.source_ip, Some("192.168.1.1".to_string()));
    assert_eq!(item.actor_name, "Admin User");
    assert_eq!(item.resource_type, "user");
}

// =============================================================================
// 準正常系: 不正な cursor パラメータ
// =============================================================================

#[tokio::test]
async fn test_find_by_tenantが不正なbase64のcursorでinvalid_inputエラーを返す() {
    let repo = setup().await;
    let tenant_id = TenantId::new();

    let result = repo
        .find_by_tenant(
            &tenant_id,
            Some("not-valid-base64!!!"),
            10,
            &AuditLogFilter::default(),
        )
        .await;

    assert!(result.is_err());
    assert!(
        matches!(
            result.unwrap_err(),
            ringiflow_infra::InfraError::InvalidInput(_)
        ),
        "base64 デコード不能な cursor は InvalidInput エラーであるべき"
    );
}

#[tokio::test]
async fn test_find_by_tenantがbase64デコード可能だがjsonでないcursorでinvalid_inputエラーを返す() {
    use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

    let repo = setup().await;
    let tenant_id = TenantId::new();

    // base64 としては有効だが、JSON としてデシリアライズできない文字列
    let invalid_json_cursor = BASE64.encode(b"this is not json");

    let result = repo
        .find_by_tenant(
            &tenant_id,
            Some(&invalid_json_cursor),
            10,
            &AuditLogFilter::default(),
        )
        .await;

    assert!(result.is_err());
    assert!(
        matches!(
            result.unwrap_err(),
            ringiflow_infra::InfraError::InvalidInput(_)
        ),
        "JSON デシリアライズ不能な cursor は InvalidInput エラーであるべき"
    );
}
