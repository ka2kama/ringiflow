# #880 S3 基盤と MinIO ローカル環境を構築する

Issue: #880
PR: #917
ブランチ: `feature/880-s3-minio-setup`

## Context

Epic #406（ドキュメント管理）の最初の Story。後続 Story（#881 アップロード、#882 ダウンロード/削除 等）の基盤となる S3 クライアントと MinIO ローカル開発環境を構築する。

詳細設計: `docs/40_詳細設計書/17_ドキュメント管理設計.md`

## スコープ

対象:
- S3 クライアントモジュール（infra クレート）— Presigned PUT/GET URL 生成、HEAD Object
- MinIO を Docker Compose に追加（開発環境 + API テスト環境）
- 環境変数による S3/MinIO 切替
- MinIO を使った統合テスト

対象外:
- Document ドメインモデル（#881+）
- API エンドポイント（#881+）
- フロントエンド（#885）
- フォルダ管理（#883）
- 削除レジストリ統合（#886）

## 設計判断

### 1. S3Client トレイト配置: infra クレート

既存パターン（`SessionManager`, `PasswordChecker`, `TenantDeleter` がすべて infra）に準拠。ドメイン層にはまだ S3 関連の概念がない（Document ドメインは #881+）。

### 2. エラー型: `InfraError::S3(String)`

`DynamoDb(String)` と同じパターン。AWS SDK のエラー型はジェネリクスが深く `#[from]` が困難。

### 3. クライアント作成: `Option<&str>` エンドポイント

- `Some(url)`: MinIO に接続（`force_path_style(true)` + 環境変数の認証情報）
- `None`: AWS S3 デフォルト（IAM ロール）

DynamoDB は常にエンドポイント必須だが、S3 は本番で未設定（SDK デフォルト使用）のため `Option` にする。

### 4. 認証情報: SDK デフォルト認証チェーン

`create_client` にクレデンシャルをハードコードしない。`.env` に `AWS_ACCESS_KEY_ID=minioadmin` / `AWS_SECRET_ACCESS_KEY=minioadmin` を設定し、SDK のデフォルト認証チェーンで読み取る。

### 5. MinIO パススタイル

MinIO はバーチャルホスト型 URL を使わないため、エンドポイント指定時のみ `force_path_style(true)` を設定。

---

## Phase 1: MinIO Docker インフラ

MinIO が `just dev-deps` で起動し、Web Console でアクセスできる状態にする。

### 変更ファイル

| ファイル | 操作 |
|---------|------|
| `infra/docker/docker-compose.yaml` | 変更: minio + minio-init サービス、minio_data ボリューム追加 |
| `infra/docker/docker-compose.api-test.yaml` | 変更: minio + minio-init サービス、minio_api_test_data ボリューム追加 |
| `scripts/env/generate.sh` | 変更: MinIO ポート変数・S3 環境変数を3ファイルに追加 |

### 詳細

**docker-compose.yaml に追加:**

```yaml
# MinIO（ローカル S3 互換ストレージ）
# 本番環境: Amazon S3
# AWS SDK の endpoint_url を MinIO に向けることで互換動作
minio:
  image: minio/minio:latest
  command: server /data --console-address ":9001"
  environment:
    MINIO_ROOT_USER: minioadmin
    MINIO_ROOT_PASSWORD: minioadmin
  ports:
    - "${MINIO_API_PORT}:9000"
    - "${MINIO_CONSOLE_PORT}:9001"
  volumes:
    - minio_data:/data
  healthcheck:
    test: ["CMD", "mc", "ready", "local"]
    interval: 5s
    timeout: 5s
    retries: 5
    start_period: 10s
  restart: unless-stopped

# MinIO 初期設定（バケット自動作成）
minio-init:
  image: minio/mc:latest
  depends_on:
    minio:
      condition: service_healthy
  entrypoint: >
    /bin/sh -c "
    mc alias set local http://minio:9000 minioadmin minioadmin;
    mc mb --ignore-existing local/ringiflow-dev-documents;
    "
```

volumes に `minio_data:` を追加。

**docker-compose.api-test.yaml に追加:**

同構造で `API_TEST_MINIO_API_PORT`, `API_TEST_MINIO_CONSOLE_PORT` を使用。volumes に `minio_api_test_data:` を追加。

**generate.sh に追加するポート:**

基準ポート:
- `BASE_MINIO_API_PORT=19000`
- `BASE_MINIO_CONSOLE_PORT=19001`
- `BASE_API_TEST_MINIO_API_PORT=19002`
- `BASE_API_TEST_MINIO_CONSOLE_PORT=19003`

ルート `.env` に追加:
```
MINIO_API_PORT=<base + offset>
MINIO_CONSOLE_PORT=<base + offset>
API_TEST_MINIO_API_PORT=<base + offset>
API_TEST_MINIO_CONSOLE_PORT=<base + offset>
```

`backend/.env` に追加:
```
S3_ENDPOINT_URL=http://localhost:<MINIO_API_PORT>
S3_BUCKET_NAME=ringiflow-dev-documents
AWS_ACCESS_KEY_ID=minioadmin
AWS_SECRET_ACCESS_KEY=minioadmin
```

`backend/.env.api-test` に追加:
```
S3_ENDPOINT_URL=http://localhost:<API_TEST_MINIO_API_PORT>
S3_BUCKET_NAME=ringiflow-dev-documents
AWS_ACCESS_KEY_ID=minioadmin
AWS_SECRET_ACCESS_KEY=minioadmin
```

### 確認事項

- パターン: Docker Compose サービス定義 → `infra/docker/docker-compose.yaml`（postgres, redis, dynamodb）
- パターン: ポートオフセット計算 → `scripts/env/generate.sh` L30-67
- パターン: API テスト用 compose → `infra/docker/docker-compose.api-test.yaml`
- パターン: `.env` ファイル生成 → `scripts/env/generate.sh` L69-225

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | `just dev-deps` で MinIO が起動する | 正常系 | 手動検証 |
| 2 | Web Console（`http://localhost:19001`）にアクセスできる | 正常系 | 手動検証 |
| 3 | `minio-init` がバケットを自動作成する | 正常系 | 手動検証 |

### テストリスト

ユニットテスト（該当なし — インフラ設定のみ）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証: `just setup-env && just dev-deps` → MinIO 起動 → Web Console 確認

---

## Phase 2: S3 クライアントモジュール

Presigned URL 生成と HEAD Object チェックを提供する S3 クライアントを infra クレートに追加する。

### 変更ファイル

| ファイル | 操作 |
|---------|------|
| `backend/Cargo.toml` | 変更: workspace に `aws-sdk-s3 = "1"` 追加 |
| `backend/crates/infra/Cargo.toml` | 変更: `aws-sdk-s3.workspace = true` 追加 |
| `backend/crates/infra/src/s3.rs` | 新規: S3Client トレイト + AwsS3Client 実装 |
| `backend/crates/infra/src/error.rs` | 変更: `S3(String)` バリアント追加 |
| `backend/crates/infra/src/lib.rs` | 変更: `pub mod s3;` + re-export 追加 |

### S3Client API

```rust
// backend/crates/infra/src/s3.rs

#[async_trait]
pub trait S3Client: Send + Sync {
    /// Presigned PUT URL を生成する（アップロード用）
    async fn generate_presigned_put_url(
        &self,
        s3_key: &str,
        content_type: &str,
        content_length: i64,
        expires_in: Duration,
    ) -> Result<String, InfraError>;

    /// Presigned GET URL を生成する（ダウンロード用）
    async fn generate_presigned_get_url(
        &self,
        s3_key: &str,
        expires_in: Duration,
    ) -> Result<String, InfraError>;

    /// オブジェクトの存在を確認する（HEAD Object）
    async fn head_object(&self, s3_key: &str) -> Result<bool, InfraError>;
}

pub struct AwsS3Client {
    client: aws_sdk_s3::Client,
    bucket_name: String,
}

/// S3 クライアントを作成する
///
/// endpoint が Some → MinIO（force_path_style + 環境変数認証）
/// endpoint が None → AWS S3（SDK デフォルト）
pub async fn create_client(endpoint: Option<&str>) -> aws_sdk_s3::Client { ... }
```

### 確認事項

- 型: `InfraError` のバリアント一覧 → `backend/crates/infra/src/error.rs`
- パターン: `dynamodb.rs` の `create_client` → `backend/crates/infra/src/dynamodb.rs` L59-71
- パターン: `lib.rs` のモジュール登録・re-export → `backend/crates/infra/src/lib.rs` L52-66
- ライブラリ: `aws_sdk_s3::presigning::PresigningConfig` → Grep 既存使用 or docs.rs
- ライブラリ: `aws_sdk_s3::Client` の `put_object()`, `get_object()`, `head_object()` → docs.rs
- ライブラリ: `aws_sdk_s3::config::Builder::force_path_style()` → docs.rs

### 操作パス

操作パス: 該当なし（ドメインロジックのみ）

### テストリスト

ユニットテスト:
- [ ] `create_client` が `Some(endpoint)` でクライアントを作成できる
- [ ] `create_client` が `None` でクライアントを作成できる

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

## Phase 3: Core Service S3 設定

Core Service が S3 設定を環境変数から読み込み、S3 クライアントを初期化する。

### 変更ファイル

| ファイル | 操作 |
|---------|------|
| `backend/apps/core-service/src/config.rs` | 変更: S3 設定フィールド追加 |
| `backend/apps/core-service/src/main.rs` | 変更: S3 クライアント初期化追加 |

### Config 追加

```rust
pub struct CoreConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    /// S3 エンドポイント URL（MinIO 使用時に設定、未設定で AWS S3 デフォルト）
    pub s3_endpoint_url: Option<String>,
    /// S3 バケット名
    pub s3_bucket_name: String,
}
```

`s3_endpoint_url`: `env::var("S3_ENDPOINT_URL").ok()`
`s3_bucket_name`: `env::var("S3_BUCKET_NAME").expect(...)`

### main.rs 追加

DB 初期化の後、ルーター構築の前:

```rust
let s3_client_inner = ringiflow_infra::s3::create_client(
    config.s3_endpoint_url.as_deref()
).await;
let _s3_client: Arc<dyn ringiflow_infra::s3::S3Client> = Arc::new(
    ringiflow_infra::s3::AwsS3Client::new(s3_client_inner, config.s3_bucket_name.clone())
);
tracing::info!("S3 クライアントを初期化しました");
```

注: `_s3_client` はこの Story では未使用（#881+ でハンドラ State に注入）。

### 確認事項

- パターン: `CoreConfig::from_env()` の required/optional パターン → `backend/apps/core-service/src/config.rs`
- パターン: BFF の `dynamodb_endpoint` 読み込み → `backend/apps/bff/src/config.rs` L51-53
- パターン: `main.rs` でのクライアント初期化 → `backend/apps/core-service/src/main.rs` L178-187

### 操作パス

操作パス: 該当なし（設定読み込みのみ）

### テストリスト

ユニットテスト（該当なし — `from_env` は環境変数依存のため既存パターンに従い省略）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

## Phase 4: MinIO 統合テスト

Presigned URL が実際に MinIO で動作することを統合テストで検証する。

### 変更ファイル

| ファイル | 操作 |
|---------|------|
| `backend/crates/infra/Cargo.toml` | 変更: dev-dependencies に `reqwest` 追加 |
| `backend/crates/infra/tests/s3_test.rs` | 新規: MinIO 統合テスト |

### テスト構造

`dynamodb_test.rs` パターンに準拠:

```rust
// エンドポイント解決（S3_ENDPOINT_URL → MINIO_API_PORT → フォールバック）
fn s3_endpoint() -> String {
    std::env::var("S3_ENDPOINT_URL").unwrap_or_else(|_| {
        let port = std::env::var("MINIO_API_PORT").unwrap_or_else(|_| "19000".to_string());
        format!("http://localhost:{port}")
    })
}

fn s3_bucket() -> String {
    std::env::var("S3_BUCKET_NAME")
        .unwrap_or_else(|_| "ringiflow-dev-documents".to_string())
}
```

テストごとに UUID v7 ベースの S3 キーで分離。`reqwest::Client` で Presigned URL を実行。

### 確認事項

- パターン: `dynamodb_test.rs` のテスト構造 → `backend/crates/infra/tests/dynamodb_test.rs`
- ライブラリ: `reqwest::Client` の PUT/GET → Grep 既存使用
- パターン: UUID v7 によるテスト分離 → `dynamodb_test.rs` L44

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | Presigned PUT URL で MinIO にアップロード | 正常系 | 統合テスト |
| 2 | Presigned GET URL で MinIO からダウンロード | 正常系 | 統合テスト |
| 3 | HEAD Object でアップロード済みファイルを確認 | 正常系 | 統合テスト |
| 4 | HEAD Object で存在しないファイルに false | 正常系 | 統合テスト |

### テストリスト

ユニットテスト（該当なし）

統合テスト（MinIO 必要）:
- [ ] Presigned PUT URL が有効な URL を返し、MinIO にファイルを PUT できる
- [ ] Presigned GET URL が有効な URL を返し、MinIO からファイルを GET でき内容が一致する
- [ ] `head_object` がアップロード済みオブジェクトに `true` を返す
- [ ] `head_object` が存在しないオブジェクトに `false` を返す
- [ ] PUT → HEAD → GET の完全フロー

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | MinIO は `force_path_style` が必要 | 技術的前提 | `create_client` でエンドポイント指定時のみ有効化 |
| 2回目 | クレデンシャルをハードコードすると本番で問題 | 既存手段の見落とし | SDK デフォルト認証チェーン + `.env` で対応 |
| 3回目 | 統合テストに `reqwest` dev-dep が必要 | 不完全なパス | infra Cargo.toml に追加 |
| 4回目 | API テスト環境の MinIO ポートが未定義 | 未定義 | `API_TEST_MINIO_*` ポート変数を追加 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 完了基準4項目すべてカバー | OK | PUT(Ph2+4), GET(Ph2+4), dev-deps MinIO(Ph1), S3_ENDPOINT_URL切替(Ph2+3) |
| 2 | 曖昧さ排除 | 不確定な記述ゼロ | OK | 全ファイルパス・ポート番号・API シグネチャが具体的 |
| 3 | 設計判断の完結性 | 全差異に判断が記載 | OK | トレイト配置、エラー型、認証方式、パススタイル、エンドポイント Option |
| 4 | スコープ境界 | 対象・対象外が明記 | OK | IN/OUT を明記、対象外の Story 番号を列挙 |
| 5 | 技術的前提 | 前提が考慮済み | OK | force_path_style、SDK 認証チェーン、reqwest dev-dep |
| 6 | 既存ドキュメント整合 | 矛盾なし | OK | 詳細設計書 17_ドキュメント管理設計.md と整合 |

## 検証方法

1. `just setup-env` で `.env` ファイル再生成 → MinIO ポート変数が含まれることを確認
2. `just dev-deps` → MinIO コンテナ起動確認、`http://localhost:19001` で Web Console アクセス
3. `cd backend && cargo build -p ringiflow-infra` → コンパイル通過
4. `cd backend && cargo test -p ringiflow-infra --test s3_test` → 統合テスト全通過
5. `just check-all` → 全チェック通過
