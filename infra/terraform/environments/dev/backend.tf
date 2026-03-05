# State backend（S3 + DynamoDB）
#
# 前提: S3 バケットと DynamoDB テーブルは手動で作成する
# → 手順書: docs/60_手順書/03_運用/01_Terraformセットアップ.md（TODO）
terraform {
  backend "s3" {
    bucket         = "ringiflow-terraform-state"
    key            = "dev/terraform.tfstate"
    region         = "ap-northeast-1"
    dynamodb_table = "ringiflow-terraform-lock"
    encrypt        = true
  }
}
