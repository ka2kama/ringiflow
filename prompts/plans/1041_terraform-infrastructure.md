# Terraform インフラ基盤の整備と SES ドメイン検証

## コンテキスト

### 目的
- Issue: #1041
- Want: Terraform で SES リソースを管理可能にし、`terraform init/validate` が動作する基盤を整備する
- 完了基準:
  - `terraform init` が成功する
  - `terraform validate` が成功する
  - SES ドメイン検証リソースが定義されている
  - DKIM レコードが outputs で出力される
  - SES メール送信テスト手順が文書化されている

### ブランチ / PR
- ブランチ: `feature/1041-terraform-infrastructure`
- PR: #1042（Draft）

### As-Is（探索結果の要約）

Terraform ディレクトリ構造:
- `infra/terraform/environments/{dev,stg,prod}/.gitkeep` — 空
- `infra/terraform/modules/network/.gitkeep` — 空
- `.gitignore` に Terraform テンプレート（`.terraform/`, `*.tfstate`, `*.tfvars` 等）は設定済み

基本設計書（`docs/30_基本設計書/02_プロジェクト構造設計.md`）で定義済みの構造:
- `environments/{dev,stg,prod}/` — 環境別設定（main.tf, variables.tf, outputs.tf, terraform.tfvars）
- `modules/` — 再利用可能なモジュール
- State backend: S3 + DynamoDB（`ringiflow-terraform-state-{account-id}`, `ringiflow-terraform-lock`）

通知機能設計（`docs/40_詳細設計書/16_通知機能設計.md`）:
- SES は通知基盤の本番メール送信に使用
- `SesNotificationSender` は #879 で配線済み
- ドメイン: `ringiflow.example.com`（環境変数 `SES_FROM_ADDRESS=noreply@ringiflow.example.com`）

制約:
- ローカル環境に Terraform がインストールされていない
- `terraform init/validate` はローカルでは検証不可（AWS アカウントも必要）
- この Issue は設定ファイルの作成が主目的。実際の `apply` は手動で行う

### 進捗
- [ ] Phase 1: Terraform 基盤設定
- [ ] Phase 2: SES ドメイン検証モジュール
- [ ] Phase 3: SES メール送信テスト手順

## 設計判断

### 1. State backend のブートストラップ問題

S3 バケットと DynamoDB テーブル（state backend 自身）は Terraform で管理できない（鶏と卵の問題）。

| 選択肢 | 説明 |
|--------|------|
| **手動作成を前提（採用）** | state backend リソースは AWS コンソールまたは AWS CLI で手動作成し、手順書に記載 |
| ブートストラップスクリプト | シェルスクリプトで S3 バケット・DynamoDB テーブルを作成 |
| local backend で開始 | 初期は local backend で、後から S3 に移行 |

採用理由:
- state backend は一度だけ作成すればよく、自動化の価値が低い
- 手順書で十分カバーできる
- ブートストラップスクリプトは実行忘れ等のリスクがありシンプルさに欠ける

→ `backend "s3"` 設定は記述するが、実際のバケット作成手順は手順書に記載する

### 2. dev 環境のみ実装

| 選択肢 | 説明 |
|--------|------|
| **dev のみ（採用）** | 現時点で必要な dev 環境のみ実装 |
| dev + stg + prod | 全環境を一括で実装 |

採用理由:
- YAGNI: stg/prod は現時点で不要
- dev で動作確認後に他環境に展開するのが安全

### 3. Terraform バージョン

| 選択肢 | 説明 |
|--------|------|
| **>= 1.0（採用）** | 広い互換性を持たせる |
| ~> 1.10（最新固定） | 最新機能を使用可能 |

採用理由:
- 基盤設定では最新機能は不要
- チームメンバーの環境差異を許容するため、最小バージョンのみ指定

### 4. `.tfvars` と変数のデフォルト値

`.tfvars` は `.gitignore` で除外済み。環境別の値は `terraform.tfvars.example` をテンプレートとして提供する。

## スコープ

### 対象
- `infra/terraform/environments/dev/` — provider, backend, SES モジュール呼び出し
- `infra/terraform/modules/ses/` — SES ドメイン検証モジュール
- `docs/60_手順書/03_運用/02_SESメール送信テスト.md` — テスト手順書

### 対象外
- stg/prod 環境の設定
- state backend（S3 バケット、DynamoDB テーブル）の実リソース作成
- `terraform apply` の実行
- modules/network/ 等の他モジュール
- CI/CD パイプライン（Terraform の自動適用）

---

## Phase 1: Terraform 基盤設定

`environments/dev/` に Terraform の基盤設定（provider, backend, variables）を作成する。

#### 確認事項
- パターン: 基本設計書の Terraform 構造 → `docs/30_基本設計書/02_プロジェクト構造設計.md` L523-588
- ライブラリ: AWS provider の設定パターン → Terraform Registry hashicorp/aws

#### 操作パス: 該当なし（操作パスが存在しない）

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証: `terraform validate` が成功すること（Terraform インストール後に手動検証）

#### 成果物

1. `infra/terraform/environments/dev/versions.tf` — Terraform と provider のバージョン制約
2. `infra/terraform/environments/dev/backend.tf` — S3 state backend 設定
3. `infra/terraform/environments/dev/providers.tf` — AWS provider 設定
4. `infra/terraform/environments/dev/variables.tf` — 共通変数定義
5. `infra/terraform/environments/dev/terraform.tfvars.example` — 変数テンプレート
6. `.gitkeep` ファイルの削除（`environments/dev/`, `environments/stg/`, `environments/prod/`, `modules/network/`）

## Phase 2: SES ドメイン検証モジュール

`modules/ses/` に SES ドメイン検証モジュールを作成し、`environments/dev/` から呼び出す。

#### 確認事項
- ライブラリ: `aws_sesv2_email_identity` リソースの設定 → Terraform Registry aws_sesv2_email_identity
- ライブラリ: `aws_sesv2_configuration_set` リソースの設定 → Terraform Registry aws_sesv2_configuration_set
- パターン: 通知機能設計の SES 設定 → `docs/40_詳細設計書/16_通知機能設計.md` L186-200

#### 操作パス: 該当なし（操作パスが存在しない）

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証: `terraform validate` が成功すること、DKIM レコードが outputs に含まれること

#### 成果物

1. `infra/terraform/modules/ses/main.tf` — SES ドメイン検証リソース定義
2. `infra/terraform/modules/ses/variables.tf` — モジュール入力変数
3. `infra/terraform/modules/ses/outputs.tf` — DKIM レコード等の出力
4. `infra/terraform/environments/dev/main.tf` — SES モジュール呼び出し
5. `infra/terraform/environments/dev/outputs.tf` — 環境レベルの出力

## Phase 3: SES メール送信テスト手順

SES のドメイン検証後にメール送信をテストする手順を文書化する。

#### 確認事項
- パターン: 既存の手順書のフォーマット → `docs/60_手順書/` 配下の既存ファイル

#### 操作パス: 該当なし（ドキュメントのみ）

#### テストリスト: 該当なし（ドキュメント作成のみ）

#### 成果物

1. `docs/60_手順書/03_運用/02_SESメール送信テスト.md`

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | Terraform 未インストールで validate 不可 | 技術的前提 | 制約として記録、手動検証を前提とする |
| 1回目 | State backend の S3/DynamoDB が未作成 | 不完全なパス | ブートストラップは手動作成を前提とし、手順書に記載 |
| 1回目 | `.tfvars` が gitignore 対象 | 競合・エッジケース | `.tfvars.example` をテンプレートとして提供 |
| 1回目 | `.gitkeep` ファイルが残る | 未定義 | Phase 1 で削除対象に追加 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | Issue の完了基準 5 項目すべてに対応する Phase がある |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase の成果物が具体的なファイルパスで列挙されている |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | state backend、環境スコープ、バージョン制約、tfvars の 4 つの判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象（dev 環境、SES モジュール、手順書）と対象外（stg/prod、apply、CI/CD）を明記 |
| 5 | 技術的前提 | 前提が考慮されている | OK | Terraform 未インストール、AWS アカウント不要（設定ファイル作成のみ）を考慮 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | 基本設計書の Terraform 構造、通知機能設計の SES 設定と整合 |
