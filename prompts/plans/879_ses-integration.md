# #879 SES 連携と環境設定

## コンテキスト

### 目的
- Issue: #879
- Want: 本番環境で AWS SES を通じてメール通知を送信できるようにする
- 完了基準:
  - AWS SES API でメールが送信できる
  - 環境変数 `NOTIFICATION_BACKEND=ses` で SES に切り替わる
  - ~~SES のドメイン検証が Terraform で定義されている~~ → 別 Issue に分離

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

### 進捗
- [x] Phase 1: SES クライアント初期化と app_builder 配線
- ~~Phase 2: Terraform SES ドメイン検証~~ → 別 Issue に分離
- ~~Phase 3: テスト手順文書~~ → 別 Issue に分離（Terraform デプロイ後に作成）

### スコープ変更の経緯

Terraform インフラ基盤（provider、state backend、.gitignore）が未整備であることが判明。SES モジュール単体では適用できないため、Terraform 関連（Phase 2, 3）を別の前提 Issue に分離した。

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

### SesClient 型エイリアス

`app_builder.rs` が `aws-sdk-sesv2` crate に直接依存するのを避けるため、infra crate で `pub type SesClient = aws_sdk_sesv2::Client` を定義。S3 の `AwsS3Client`（独自ラッパー型）とは異なり、SES はラッパーが不要なため型エイリアスで対応。

## Phase 1: SES クライアント初期化と app_builder 配線

### 確認事項
- 型: `aws_sdk_sesv2::Client` → `backend/crates/infra/src/notification/ses.rs:8`
- パターン: S3 クライアント初期化 → `backend/crates/infra/src/s3.rs:198-218`
- パターン: app_builder の通知バックエンド切替 → `backend/apps/core-service/src/app_builder.rs:207-227`
- パターン: main.rs の S3 クライアント初期化 → `backend/apps/core-service/src/main.rs:104-110`
- ライブラリ: `aws_config::defaults` → Grep 既存使用（`s3.rs:199`）

### 変更内容

1. `backend/crates/infra/src/notification/ses.rs` に `create_ses_client()` を追加
2. `backend/crates/infra/src/notification.rs` で re-export + `SesClient` 型エイリアス追加
3. `backend/apps/core-service/src/main.rs` で条件付き SES クライアント生成
4. `build_app` シグネチャ変更: `ses_client: Option<ringiflow_infra::notification::SesClient>` を追加
5. `app_builder.rs` の match arm に "ses" を追加

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | `NOTIFICATION_BACKEND=ses` でサーバー起動 → SES sender が選択される | 正常系 | コンパイル確認（実 AWS 接続が必要なためユニットテスト不可） |
| 2 | `NOTIFICATION_BACKEND=smtp` でサーバー起動 → SMTP sender が選択される（既存動作維持） | 正常系 | 既存テスト |
| 3 | `NOTIFICATION_BACKEND` 未設定 → Noop sender（既存動作維持） | 正常系 | 既存テスト |

### テストリスト

ユニットテスト:
- [x] `ses::create_ses_client` が `Send + Sync` な `Client` を返す（コンパイル確認で十分）
- [x] 既存テスト（`SesNotificationSender` の `Send + Sync` テスト）がパスする

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

注: バックエンド切替のロジックは `app_builder` の match 文であり、統合テストは実際の AWS 接続が必要なため CI では実行不可。ユニットテストとコンパイル確認でカバーする。

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `build_app` が同期関数のため SES クライアントを内部で生成できない | 技術的前提 | main.rs で条件付き生成し `Option<Client>` を渡す設計に |
| 2回目 | `create_client` の名前が S3 と衝突する可能性 | 曖昧 | `create_ses_client` に明確化 |
| 3回目 | `app_builder.rs` が `aws-sdk-sesv2` crate に直接依存する | アーキテクチャ不整合 | infra crate で `SesClient` 型エイリアスを定義して re-export |
| 4回目 | Terraform インフラ基盤が未整備 | 不完全なパス | Phase 2, 3 を別 Issue に分離 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | Phase 1 で SES API 有効化と環境変数切替をカバー。Terraform は別 Issue に分離済み |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 関数名、配置、シグネチャが具体的に記載 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | クライアント生成場所、関数配置、型エイリアスの判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象: SES 有効化。対象外: Terraform、テスト手順書（別 Issue） |
| 5 | 技術的前提 | 前提が考慮されている | OK | `build_app` が同期関数である制約を考慮 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | 詳細設計書 16_通知機能設計.md の環境切替セクションと整合 |
