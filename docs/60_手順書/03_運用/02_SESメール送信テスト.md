# SES メール送信テスト手順

本番環境で AWS SES を通じたメール送信を検証する手順。

## 前提条件

- AWS CLI が設定済みであること
- SES ドメイン検証が完了していること（DKIM レコードが DNS に設定済み）
- SES がサンドボックスモードの場合、送信先メールアドレスが検証済みであること

```bash
# AWS 認証確認
aws sts get-caller-identity

# SES ドメイン検証状態の確認
aws sesv2 get-email-identity --email-identity <ドメイン名> --region ap-northeast-1 \
  --query '{Status: VerificationStatus, DkimStatus: DkimAttributes.Status}'
```

期待する出力:

```json
{
  "Status": "SUCCESS",
  "DkimStatus": "SUCCESS"
}
```

---

## 1. Terraform Apply（初回のみ）

### 1.1 変数ファイルの準備

```bash
cd infra/terraform/environments/dev

# テンプレートからコピー
cp terraform.tfvars.example terraform.tfvars

# ドメイン名を編集
vi terraform.tfvars
```

### 1.2 Terraform 初期化と適用

```bash
# backend.tf の S3 バケットが作成済みであること
# → 手順: docs/60_手順書/02_プロジェクト構築/02_Terraform基盤構築.md

terraform init
terraform plan
terraform apply
```

### 1.3 DKIM レコードの設定

`terraform apply` の出力から DKIM トークンを確認する:

```bash
terraform output ses_dkim_tokens
```

出力される 3 つのトークンを DNS に CNAME レコードとして設定する:

| レコードタイプ | Name | Value |
|-------------|------|-------|
| CNAME | `{token1}._domainkey.{domain}` | `{token1}.dkim.amazonses.com` |
| CNAME | `{token2}._domainkey.{domain}` | `{token2}.dkim.amazonses.com` |
| CNAME | `{token3}._domainkey.{domain}` | `{token3}.dkim.amazonses.com` |

DNS 反映後（数分〜数時間）、SES コンソールでドメイン検証状態が「Verified」になることを確認する。

---

## 2. SES サンドボックスの確認

新規 AWS アカウントの SES はサンドボックスモードで動作する。サンドボックスでは検証済みメールアドレスにのみ送信可能。

### 2.1 サンドボックス状態の確認

```bash
aws sesv2 get-account --region ap-northeast-1 \
  --query '{ProductionAccess: ProductionAccessEnabled, SendingEnabled: SendingEnabled}'
```

- `ProductionAccess: false`: サンドボックスモード（送信先の事前検証が必要）
- `ProductionAccess: true`: 本番アクセス（任意のアドレスに送信可能）

### 2.2 テスト用メールアドレスの検証（サンドボックス時）

```bash
aws sesv2 create-email-identity \
  --email-identity test@example.com \
  --region ap-northeast-1
```

検証メールが届くので、メール内のリンクをクリックして検証を完了する。

---

## 3. テストメール送信（AWS CLI）

```bash
aws sesv2 send-email \
  --from-email-address "noreply@<ドメイン名>" \
  --destination '{"ToAddresses":["<送信先メールアドレス>"]}' \
  --content '{
    "Simple": {
      "Subject": {"Data": "[RingiFlow] SES テストメール"},
      "Body": {
        "Text": {"Data": "SES からのテストメール送信に成功しました。"},
        "Html": {"Data": "<h1>SES テストメール</h1><p>SES からのテストメール送信に成功しました。</p>"}
      }
    }
  }' \
  --configuration-set-name ringiflow-dev \
  --region ap-northeast-1
```

成功時の出力:

```json
{
    "MessageId": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
}
```

### 送信結果の確認

1. 送信先メールボックスでメールが受信されていること
2. メールヘッダに DKIM 署名が含まれていること（`dkim=pass`）
3. SPF が pass していること

---

## 4. アプリケーション経由のテスト

Core Service の環境変数を SES に切り替えて起動する。

```bash
NOTIFICATION_BACKEND=ses \
NOTIFICATION_FROM_ADDRESS=noreply@<ドメイン名> \
just dev-core
```

ワークフローの承認操作を実行し、通知メールが送信されることを確認する:

1. ワークフローを申請する（承認依頼メールが送信される）
2. ワークフローを承認する（承認完了メールが送信される）
3. `notification_logs` テーブルで送信結果を確認:

```sql
SELECT event_type, recipient_email, status, error_message, sent_at
FROM notification_logs
ORDER BY sent_at DESC
LIMIT 10;
```

> ローカル開発では `NOTIFICATION_BACKEND=smtp`（Mailpit）を使用するのが通常の開発フロー。SES での送信テストは本番環境に近い検証が必要な場合にのみ実施する。

---

## トラブルシューティング

### メールが届かない

```bash
# SES の送信統計を確認
aws sesv2 get-account --region ap-northeast-1 \
  --query '{SendingEnabled: SendingEnabled, ProductionAccessEnabled: ProductionAccessEnabled}'

# 直近の送信イベントを確認（CloudWatch が必要）
aws sesv2 get-configuration-set \
  --configuration-set-name ringiflow-dev \
  --region ap-northeast-1
```

| 症状 | 確認事項 |
|------|---------|
| サンドボックスで未検証アドレスに送信 | 送信先メールアドレスを検証する |
| ドメイン検証未完了 | DNS の CNAME レコードを確認する |
| 送信制限超過 | SES のクォータを確認する |
| スパムフォルダ | 受信メールのスパムフォルダを確認する |
| `MessageRejected` | ドメイン検証状態、サンドボックスモード |
| `AccessDenied` | IAM ポリシーに `ses:SendEmail` があるか |
| タイムアウト | VPC からインターネットへの疎通（NAT Gateway） |
| `notification_logs` に `failed` | Core Service ログで詳細エラーを確認 |

### DKIM 検証が完了しない

```bash
# DKIM ステータスの確認
aws sesv2 get-email-identity \
  --email-identity <ドメイン名> \
  --region ap-northeast-1 \
  --query 'DkimAttributes.Status'
```

- `PENDING`: DNS レコードの反映待ち（最大 72 時間）
- `SUCCESS`: 検証完了
- `FAILED`: CNAME レコードの設定を再確認

---

## 関連ドキュメント

- [Terraform 基盤構築手順](../02_プロジェクト構築/02_Terraform基盤構築.md)
- [通知機能設計](../../40_詳細設計書/16_通知機能設計.md)
- [Terraform SES モジュール](../../../infra/terraform/modules/ses/)

## 変更履歴

| 日付 | 変更内容 | 担当 |
|------|---------|------|
| 2026-03-05 | 初版作成（#1041） | - |
