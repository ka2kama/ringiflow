# SES ドメイン検証
module "ses" {
  source = "../../modules/ses"

  domain_name = var.domain_name
  environment = "dev"
}
