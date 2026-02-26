//! S3 / MinIO 統合テスト
//!
//! MinIO を使用した Presigned URL 生成と HEAD Object の統合テスト。
//!
//! 実行方法:
//! ```bash
//! just dev-deps
//! cd backend && cargo test -p ringiflow-infra --test s3_test
//! ```

use std::time::Duration;

use ringiflow_infra::s3::{self, S3Client};

/// テスト用の S3（MinIO）エンドポイント
///
/// 優先順位:
/// 1. `S3_ENDPOINT_URL`（CI で明示的に設定）
/// 2. `MINIO_API_PORT` から構築（justfile が root `.env` から渡す）
/// 3. フォールバック: `http://localhost:19000`
fn s3_endpoint() -> String {
    std::env::var("S3_ENDPOINT_URL").unwrap_or_else(|_| {
        let port = std::env::var("MINIO_API_PORT").unwrap_or_else(|_| "19000".to_string());
        format!("http://localhost:{port}")
    })
}

/// テスト用のバケット名
fn s3_bucket() -> String {
    std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "ringiflow-dev-documents".to_string())
}

/// テストごとに一意な S3 キーを生成する（UUID v7 で分離）
fn test_s3_key(prefix: &str) -> String {
    format!("test/{prefix}/{}", uuid::Uuid::now_v7())
}

/// テスト用の S3 クライアントを作成する
///
/// `backend/.env` から環境変数を読み込み、MinIO に接続するクライアントを作成する。
async fn create_test_client() -> impl S3Client {
    // .env ファイルから MinIO の接続情報・AWS クレデンシャルを読み込む
    dotenvy::dotenv().ok();

    let endpoint = s3_endpoint();
    let bucket = s3_bucket();
    let client = s3::create_client(Some(&endpoint)).await;
    s3::AwsS3Client::new(client, bucket)
}

#[tokio::test]
async fn test_presigned_put_urlでminioにファイルをアップロードできる() {
    let s3_client = create_test_client().await;
    let s3_key = test_s3_key("put");
    let content = b"Hello, MinIO!";
    let content_type = "text/plain";

    // Presigned PUT URL を生成
    let put_url = s3_client
        .generate_presigned_put_url(
            &s3_key,
            content_type,
            content.len() as i64,
            Duration::from_secs(300),
        )
        .await
        .expect("Presigned PUT URL の生成に失敗");

    assert!(!put_url.is_empty(), "生成された URL が空です");
    assert!(
        put_url.starts_with("http"),
        "URL が http で始まっていません: {put_url}"
    );

    // reqwest で Presigned URL に PUT（ブラウザからのアップロードを模擬）
    let http_client = reqwest::Client::new();
    let response = http_client
        .put(&put_url)
        .header("Content-Type", content_type)
        .body(content.to_vec())
        .send()
        .await
        .expect("PUT リクエストの送信に失敗");

    assert!(
        response.status().is_success(),
        "PUT が失敗しました: status={}",
        response.status()
    );
}

#[tokio::test]
async fn test_presigned_get_urlでminioからファイルをダウンロードできる() {
    let s3_client = create_test_client().await;
    let s3_key = test_s3_key("get");
    let content = b"Download test content";
    let content_type = "text/plain";

    // 事前にファイルをアップロード
    let put_url = s3_client
        .generate_presigned_put_url(
            &s3_key,
            content_type,
            content.len() as i64,
            Duration::from_secs(300),
        )
        .await
        .expect("PUT URL の生成に失敗");

    let http_client = reqwest::Client::new();
    http_client
        .put(&put_url)
        .header("Content-Type", content_type)
        .body(content.to_vec())
        .send()
        .await
        .expect("PUT に失敗");

    // Presigned GET URL を生成してダウンロード
    let get_url = s3_client
        .generate_presigned_get_url(&s3_key, Duration::from_secs(300))
        .await
        .expect("Presigned GET URL の生成に失敗");

    let response = http_client
        .get(&get_url)
        .send()
        .await
        .expect("GET リクエストの送信に失敗");

    assert!(
        response.status().is_success(),
        "GET が失敗しました: status={}",
        response.status()
    );

    let body = response
        .bytes()
        .await
        .expect("レスポンスボディの取得に失敗");
    assert_eq!(
        body.as_ref(),
        content,
        "ダウンロードした内容がアップロードした内容と一致しません"
    );
}

#[tokio::test]
async fn test_head_objectがアップロード済みオブジェクトにtrueを返す() {
    let s3_client = create_test_client().await;
    let s3_key = test_s3_key("head-exists");
    let content = b"Head object test";
    let content_type = "application/octet-stream";

    // ファイルをアップロード
    let put_url = s3_client
        .generate_presigned_put_url(
            &s3_key,
            content_type,
            content.len() as i64,
            Duration::from_secs(300),
        )
        .await
        .expect("PUT URL の生成に失敗");

    let http_client = reqwest::Client::new();
    http_client
        .put(&put_url)
        .header("Content-Type", content_type)
        .body(content.to_vec())
        .send()
        .await
        .expect("PUT に失敗");

    // HEAD Object で存在確認
    let exists = s3_client
        .head_object(&s3_key)
        .await
        .expect("head_object の実行に失敗");

    assert!(
        exists,
        "アップロード済みオブジェクトに対して false が返されました"
    );
}

#[tokio::test]
async fn test_head_objectが存在しないオブジェクトにfalseを返す() {
    let s3_client = create_test_client().await;
    let s3_key = test_s3_key("head-not-found");

    // アップロードせずに HEAD Object
    let exists = s3_client
        .head_object(&s3_key)
        .await
        .expect("head_object の実行に失敗");

    assert!(
        !exists,
        "存在しないオブジェクトに対して true が返されました"
    );
}

#[tokio::test]
async fn test_put_head_getの完全フロー() {
    let s3_client = create_test_client().await;
    let s3_key = test_s3_key("full-flow");
    let content = b"Full flow test: upload, check, download";
    let content_type = "text/plain";
    let http_client = reqwest::Client::new();

    // Step 1: アップロード前は存在しない
    let exists_before = s3_client
        .head_object(&s3_key)
        .await
        .expect("head_object（アップロード前）に失敗");
    assert!(
        !exists_before,
        "アップロード前にオブジェクトが存在しています"
    );

    // Step 2: Presigned PUT URL でアップロード
    let put_url = s3_client
        .generate_presigned_put_url(
            &s3_key,
            content_type,
            content.len() as i64,
            Duration::from_secs(300),
        )
        .await
        .expect("PUT URL の生成に失敗");

    let put_response = http_client
        .put(&put_url)
        .header("Content-Type", content_type)
        .body(content.to_vec())
        .send()
        .await
        .expect("PUT リクエストの送信に失敗");
    assert!(put_response.status().is_success(), "PUT が失敗しました");

    // Step 3: HEAD Object で存在確認
    let exists_after = s3_client
        .head_object(&s3_key)
        .await
        .expect("head_object（アップロード後）に失敗");
    assert!(exists_after, "アップロード後にオブジェクトが存在しません");

    // Step 4: Presigned GET URL でダウンロード＆内容検証
    let get_url = s3_client
        .generate_presigned_get_url(&s3_key, Duration::from_secs(300))
        .await
        .expect("GET URL の生成に失敗");

    let get_response = http_client
        .get(&get_url)
        .send()
        .await
        .expect("GET リクエストの送信に失敗");
    assert!(get_response.status().is_success(), "GET が失敗しました");

    let body = get_response
        .bytes()
        .await
        .expect("レスポンスボディの取得に失敗");
    assert_eq!(body.as_ref(), content, "ダウンロードした内容が一致しません");
}
