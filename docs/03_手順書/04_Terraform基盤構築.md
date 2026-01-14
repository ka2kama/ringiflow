# Terraform 基盤構築手順

## 目的

AWS インフラストラクチャをコードで管理するための Terraform 基盤を構築する。
この手順では Phase 0 として最小限の構成（State 管理 + モジュール構造）を作成する。

## 前提条件

- `06_CICD構築.md` が完了していること
- Terraform 1.10+ がインストール済みであること
- AWS CLI が設定済みであること（開発用 AWS アカウント）

```bash
# バージョン確認
terraform --version
# 出力例: Terraform v1.10.x

aws --version
# 出力例: aws-cli/2.x.x

# AWS 認証確認
aws sts get-caller-identity
# アカウント情報が表示されること
```

**注意**: 本番環境への Terraform 適用は Phase 1 以降で行う。Phase 0 ではファイル構造の準備のみ。

---

## 概要

Terraform で AWS リソースを IaC として管理する。

→ 参照: [`/infra/terraform/README.md`](/infra/terraform/README.md)

---

## 1. ディレクトリ構造の確認

```bash
find infra/terraform -type d | sort
```

期待される出力:

```
infra/terraform
infra/terraform/environments
infra/terraform/environments/dev
infra/terraform/environments/prod
infra/terraform/environments/stg
infra/terraform/modules
infra/terraform/modules/ecs
infra/terraform/modules/network
infra/terraform/modules/rds
infra/terraform/modules/redis
```

→ 参照: [`/infra/terraform/README.md`](/infra/terraform/README.md)

---

## 2. State 管理用リソースの手動作成

Terraform State を保存するための S3 バケットと DynamoDB テーブルを作成する。

**重要**: これらのリソースは Terraform 自体では管理できないため、AWS CLI で手動作成する。

### AWS アカウント ID の取得

```bash
AWS_ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)
echo "AWS Account ID: $AWS_ACCOUNT_ID"
```

### S3 バケットの作成

```bash
# バケット名
BUCKET_NAME="ringiflow-terraform-state-${AWS_ACCOUNT_ID}"

# バケット作成
aws s3api create-bucket \
  --bucket "$BUCKET_NAME" \
  --region ap-northeast-1 \
  --create-bucket-configuration LocationConstraint=ap-northeast-1

# バージョニング有効化
aws s3api put-bucket-versioning \
  --bucket "$BUCKET_NAME" \
  --versioning-configuration Status=Enabled

# 暗号化設定
aws s3api put-bucket-encryption \
  --bucket "$BUCKET_NAME" \
  --server-side-encryption-configuration '{
    "Rules": [
      {
        "ApplyServerSideEncryptionByDefault": {
          "SSEAlgorithm": "AES256"
        }
      }
    ]
  }'

# パブリックアクセスブロック
aws s3api put-public-access-block \
  --bucket "$BUCKET_NAME" \
  --public-access-block-configuration '{
    "BlockPublicAcls": true,
    "IgnorePublicAcls": true,
    "BlockPublicPolicy": true,
    "RestrictPublicBuckets": true
  }'

echo "S3 bucket created: $BUCKET_NAME"
```

### DynamoDB テーブルの作成

```bash
# テーブル作成
aws dynamodb create-table \
  --table-name ringiflow-terraform-lock \
  --attribute-definitions AttributeName=LockID,AttributeType=S \
  --key-schema AttributeName=LockID,KeyType=HASH \
  --billing-mode PAY_PER_REQUEST \
  --region ap-northeast-1

echo "DynamoDB table created: ringiflow-terraform-lock"
```

### 作成確認

```bash
# S3 バケット確認
aws s3 ls | grep ringiflow-terraform-state

# DynamoDB テーブル確認
aws dynamodb describe-table --table-name ringiflow-terraform-lock --query 'Table.TableName'
```

---

## 3. 環境別設定ファイルの確認

`infra/terraform/environments/dev/` に設定ファイルが存在することを確認する。

→ 参照:
- [`/infra/terraform/environments/dev/backend.tf`](/infra/terraform/environments/dev/backend.tf)
- [`/infra/terraform/environments/dev/main.tf`](/infra/terraform/environments/dev/main.tf)
- [`/infra/terraform/environments/dev/variables.tf`](/infra/terraform/environments/dev/variables.tf)
- [`/infra/terraform/environments/dev/outputs.tf`](/infra/terraform/environments/dev/outputs.tf)
- [`/infra/terraform/environments/dev/terraform.tfvars`](/infra/terraform/environments/dev/terraform.tfvars)

**注意**: `backend.tf` の S3 バケット名を自身の AWS アカウント ID に置き換えること。

---

## 4. モジュールの確認

`infra/terraform/modules/` 配下にモジュールが存在することを確認する。

→ 参照:
- [`/infra/terraform/modules/network/`](/infra/terraform/modules/network/)
- [`/infra/terraform/modules/ecs/`](/infra/terraform/modules/ecs/)
- [`/infra/terraform/modules/rds/`](/infra/terraform/modules/rds/)
- [`/infra/terraform/modules/redis/`](/infra/terraform/modules/redis/)

---

## 5. .gitignore の確認

Terraform 固有のファイルが gitignore されていることを確認する。

→ 参照: [`/infra/terraform/.gitignore`](/infra/terraform/.gitignore)

---

## 6. Terraform 初期化（検証のみ）

Phase 0 では実際のリソース作成は行わないが、構文チェックのため初期化を実行する。

### backend.tf の一時修正

State バックエンドをローカルに変更して検証:

```bash
cd infra/terraform/environments/dev

# backend.tf を一時的にコメントアウト
mv backend.tf backend.tf.bak
```

### 初期化

```bash
terraform init
```

期待される出力:

```
Initializing the backend...
Initializing provider plugins...
- Finding hashicorp/aws versions matching "~> 5.0"...
- Installing hashicorp/aws v5.x.x...
Terraform has been successfully initialized!
```

### 構文検証

```bash
terraform validate
```

期待される出力:

```
Success! The configuration is valid.
```

### フォーマットチェック

```bash
terraform fmt -check -recursive
```

### backend.tf の復元

```bash
mv backend.tf.bak backend.tf
rm -rf .terraform terraform.tfstate*
```

---

## 7. 完了確認チェックリスト

| 項目 | 確認コマンド | 期待結果 |
|------|-------------|----------|
| ディレクトリ構造 | `find infra/terraform -type d` | 期待する構造が表示 |
| dev main.tf | `cat infra/terraform/environments/dev/main.tf` | provider 設定が表示 |
| network モジュール | `cat infra/terraform/modules/network/main.tf` | VPC リソースが表示 |
| 構文検証 | `cd infra/terraform/environments/dev && terraform validate` | Success |
| フォーマット | `terraform fmt -check -recursive infra/terraform/` | 終了コード 0 |

---

## トラブルシューティング

### `terraform init` でプロバイダーエラー

```bash
# プロバイダーキャッシュをクリア
rm -rf .terraform
terraform init
```

### AWS 認証エラー

```bash
# 認証情報を確認
aws sts get-caller-identity

# 認証情報を再設定
aws configure
```

### State ロックエラー

```bash
# DynamoDB のロックを確認
aws dynamodb scan --table-name ringiflow-terraform-lock

# 必要に応じてロックを手動解除（注意: 他の操作が進行中でないことを確認）
terraform force-unlock LOCK_ID
```

### S3 バケット名の競合

```bash
# バケット名は全世界で一意である必要がある
# アカウント ID を含めることで競合を回避
```

---

## 次のステップ

Terraform 基盤構築が完了したら、Phase 0 のすべての作業が完了。

`00_Phase0_概要.md` の完了条件を確認し、Phase 1（MVP 実装）に進む。

---

## 変更履歴

| 日付 | 変更内容 | 担当 |
|------|---------|------|
| 2026-01-13 | 初版作成 | - |
