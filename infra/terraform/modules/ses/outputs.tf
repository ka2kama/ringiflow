# DKIM 検証用 DNS レコード
#
# 以下の 3 つの CNAME レコードを DNS に設定する:
#   Name:  {token}._domainkey.{domain_name}
#   Value: {token}.dkim.amazonses.com
output "dkim_tokens" {
  description = "DKIM 検証用トークン（3 つの CNAME レコードを DNS に設定する）"
  value       = aws_sesv2_email_identity.domain.dkim_signing_attributes[0].tokens
}

output "configuration_set_name" {
  description = "SES Configuration Set 名"
  value       = aws_sesv2_configuration_set.main.configuration_set_name
}

output "domain_name" {
  description = "検証対象ドメイン名"
  value       = var.domain_name
}
