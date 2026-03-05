# SES DKIM 検証用 DNS レコード
#
# terraform apply 後に出力されるトークンを DNS に CNAME レコードとして設定する:
#   Name:  {token}._domainkey.{domain_name}
#   Value: {token}.dkim.amazonses.com
output "ses_dkim_tokens" {
  description = "SES DKIM 検証用トークン"
  value       = module.ses.dkim_tokens
}

output "ses_configuration_set_name" {
  description = "SES Configuration Set 名"
  value       = module.ses.configuration_set_name
}

output "ses_domain_name" {
  description = "SES 検証対象ドメイン名"
  value       = module.ses.domain_name
}
