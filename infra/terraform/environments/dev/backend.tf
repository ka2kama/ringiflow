# State backend（S3 + DynamoDB）
#
# 前提: S3 バケットと DynamoDB テーブルは手動で作成する
# → 手順書: docs/60_手順書/02_プロジェクト構築/02_Terraform基盤構築.md
#
# ACCOUNT_ID を実際の AWS アカウント ID に置き換えること
terraform {
  backend "s3" {
    bucket         = "ringiflow-terraform-state-ACCOUNT_ID"
    key            = "dev/terraform.tfstate"
    region         = "ap-northeast-1"
    dynamodb_table = "ringiflow-terraform-lock"
    encrypt        = true
  }
}
