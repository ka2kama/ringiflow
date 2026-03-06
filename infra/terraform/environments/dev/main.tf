# SES ドメイン検証
module "ses" {
  source = "../../modules/ses"

  domain_name = var.domain_name
  environment = "dev"
}

# S3 ドキュメントバケット
module "s3_documents" {
  source = "../../modules/s3_documents"

  environment          = "dev"
  cors_allowed_origins = var.cors_allowed_origins
}
