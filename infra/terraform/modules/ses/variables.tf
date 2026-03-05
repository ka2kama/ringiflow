variable "domain_name" {
  description = "SES で検証するドメイン名"
  type        = string
}

variable "environment" {
  description = "環境名（dev, stg, prod）"
  type        = string
}
