# #879 SES 連携と環境設定

## コンテキスト

### 目的
- Issue: #879
- Want: 本番環境で AWS SES を通じてメール通知を送信できるようにする
- 完了基準:
  - AWS SES API でメールが送信できる
  - 環境変数 `NOTIFICATION_BACKEND=ses` で SES に切り替わる
  - SES のドメイン検証が Terraform で定義されている

### ブランチ / PR
- ブランチ: `feature/879-ses-integration`
- PR: #1040（Draft）

### As-Is（探索結果の要約）
- `SesNotificationSender` 実装済み: `backend/crates/infra/src/notification/ses.rs`
- `aws-sdk-sesv2` は `Cargo.toml` に追加済み（workspace 依存）
- `aws-config` も infra crate に追加済み
- `NotificationConfig` は `NOTIFICATION_BACKEND` 環境変数を読み込み済み: `backend/apps/core-service/src/config.rs`
- `app_builder.rs:221-222` に `// SES バックエンドは #879 で有効化` コメントあり — "ses" match arm が未実装
- `SesNotificationSender::new(client: Client, from_address: String)` で `aws_sdk_sesv2::Client` が必要
- AWS SDK クライアント初期化パターン: `s3.rs:198-218` — `aws_config::defaults().load().await` → サービス固有 Client
- `build_app()` は同期関数。SES クライアント生成は async のため `main.rs` で行う必要あり
- Terraform ディレクトリは `.gitkeep` のみ（構成未着手）

### 進捗
- [x] Phase 1: SES クライアント初期化と app_builder 配線
- [x] Phase 2: Terraform SES ドメイン検証
- [x] Phase 3: テスト手順文書

## 設計判断

### SES クライアントの生成場所

| 選択肢 | 説明 |
|--------|------|
| **main.rs で条件付き生成（採用）** | S3 クライアントと同じパターン。`build_app` に `Option<aws_sdk_sesv2::Client>` を渡す |
| app_builder 内で async 化 | `build_app` を async にする必要があり、変更範囲が大きい |

理由: S3 クライアントの既存パターンに従い、`main.rs` で async 初期化 → `build_app` に渡す。backend が "ses" のときのみ生成する。

### SES クライアント生成関数の配置

| 選択肢 | 説明 |
|--------|------|
| **`notification/ses.rs` に `create_client()` を追加（採用）** | 送信者と同じモジュールに配置 |
| 独立モジュール（`ses_config.rs` 等） | S3 は `s3.rs` に配置しているが、SES はシンプルなので分離不要 |

理由: `s3.rs` が `create_client` + `AwsS3Client` を同居させているのと同様のパターン。SES は endpoint 切替不要なのでよりシンプル。

## Phase 1: SES クライアント初期化と app_builder 配線

### 確認事項
- 型: `aws_sdk_sesv2::Client` → `backend/crates/infra/src/notification/ses.rs:8`
- パターン: S3 クライアント初期化 → `backend/crates/infra/src/s3.rs:198-218`
- パターン: app_builder の通知バックエンド切替 → `backend/apps/core-service/src/app_builder.rs:207-227`
- パターン: main.rs の S3 クライアント初期化 → `backend/apps/core-service/src/main.rs:104-110`
- ライブラリ: `aws_config::defaults` → Grep 既存使用（`s3.rs:199`）

### 変更内容

1. `backend/crates/infra/src/notification/ses.rs` に `create_client()` を追加
   ```rust
   pub async fn create_client() -> Client {
       let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
           .region(aws_config::Region::new("ap-northeast-1"))
           .load()
           .await;
       Client::new(&config)
   }
   ```

2. `backend/crates/infra/src/notification.rs` で `create_client` を re-export
   ```rust
   pub use ses::{SesNotificationSender, create_ses_client};
   ```
   （関数名は `create_ses_client` として明確化）

3. `backend/apps/core-service/src/main.rs` で条件付き SES クライアント生成
   ```rust
   let ses_client = if config.notification.backend == "ses" {
       let client = ringiflow_infra::notification::create_ses_client().await;
       tracing::info!("SES クライアントを初期化しました");
       Some(client)
   } else {
       None
   };
   ```

4. `build_app` シグネチャ変更: `ses_client: Option<aws_sdk_sesv2::Client>` を追加

5. `app_builder.rs` の match arm に "ses" を追加:
   ```rust
   "ses" => {
       let client = ses_client.expect("NOTIFICATION_BACKEND=ses だが SES クライアントが未初期化");
       tracing::info!("SES バックエンドで通知サービスを初期化します");
       Arc::new(SesNotificationSender::new(client, config.notification.from_address.clone()))
   }
   ```

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | `NOTIFICATION_BACKEND=ses` でサーバー起動 → SES sender が選択される | 正常系 | ユニットテスト |
| 2 | `NOTIFICATION_BACKEND=smtp` でサーバー起動 → SMTP sender が選択される（既存動作維持） | 正常系 | 既存テスト |
| 3 | `NOTIFICATION_BACKEND` 未設定 → Noop sender（既存動作維持） | 正常系 | 既存テスト |

### テストリスト

ユニットテスト:
- [ ] `ses::create_ses_client` が `Send + Sync` な `Client` を返す（コンパイル確認で十分）
- [ ] 既存テスト（`SesNotificationSender` の `Send + Sync` テスト）がパスする

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

注: バックエンド切替のロジックは `app_builder` の match 文であり、統合テストは実際の AWS 接続が必要なため CI では実行不可。ユニットテストとコンパイル確認でカバーする。

## Phase 2: Terraform SES ドメイン検証

### 確認事項
- パターン: Terraform ディレクトリ構造 → `infra/terraform/`（`.gitkeep` のみ）
- ライブラリ: AWS provider の SES リソース → `aws_sesv2_email_identity`, `aws_sesv2_configuration_set`

### 変更内容

`infra/terraform/modules/ses/` に SES モジュールを作成:

1. `main.tf`: SES ドメイン検証リソース
   - `aws_sesv2_email_identity` — ドメイン検証
   - `aws_sesv2_configuration_set` — 送信設定

2. `variables.tf`: 入力変数
   - `domain` — 検証するドメイン
   - `environment` — 環境名

3. `outputs.tf`: 出力値
   - DKIM レコード（Route53 手動設定用）

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

注: Terraform は `terraform validate` で構文チェック。実際のデプロイは手動。

## Phase 3: テスト手順文書

### 確認事項: なし（既知のパターンのみ）

### 変更内容

`docs/60_手順書/` に本番メール送信テスト手順を追加。

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `build_app` が同期関数のため SES クライアントを内部で生成できない | 技術的前提 | main.rs で条件付き生成し `Option<Client>` を渡す設計に |
| 2回目 | `create_client` の名前が S3 と衝突する可能性 | 曖昧 | `create_ses_client` に明確化 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | 完了基準 3 項目すべてに Phase が対応: SES API → Phase 1, 環境変数切替 → Phase 1, Terraform → Phase 2 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 関数名、配置、シグネチャが具体的に記載 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | クライアント生成場所、関数配置の判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象: SES 有効化、Terraform、手順書。対象外: 実際のデプロイ、Route53 設定 |
| 5 | 技術的前提 | 前提が考慮されている | OK | `build_app` が同期関数である制約を考慮 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | 詳細設計書 16_通知機能設計.md の環境切替セクションと整合 |
