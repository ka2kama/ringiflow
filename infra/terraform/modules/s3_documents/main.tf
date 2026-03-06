# S3 ドキュメントバケットモジュール
#
# テナントのドキュメント（ワークフロー添付ファイル、フォルダ内ファイル）を
# 格納する S3 バケットを作成する。
#
# テナント隔離: {tenant_id}/ プレフィックスで論理分離
# テナント退会: プレフィックス削除で一括削除
#
# 参照: docs/40_詳細設計書/17_ドキュメント管理設計.md

# S3 バケット
resource "aws_s3_bucket" "documents" {
  bucket = "ringiflow-${var.environment}-documents"
}

# パブリックアクセスブロック（全ブロック有効）
resource "aws_s3_bucket_public_access_block" "documents" {
  bucket = aws_s3_bucket.documents.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# サーバーサイド暗号化（SSE-S3: AES-256）
resource "aws_s3_bucket_server_side_encryption_configuration" "documents" {
  bucket = aws_s3_bucket.documents.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
    bucket_key_enabled = true
  }
}

# CORS 設定（Presigned URL によるブラウザ直接アップロード用）
resource "aws_s3_bucket_cors_configuration" "documents" {
  bucket = aws_s3_bucket.documents.id

  cors_rule {
    allowed_headers = ["*"]
    allowed_methods = ["GET", "PUT"]
    allowed_origins = var.cors_allowed_origins
    expose_headers  = ["ETag"]
    max_age_seconds = 3600
  }
}
