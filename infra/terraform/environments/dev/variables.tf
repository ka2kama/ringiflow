variable "aws_region" {
  description = "AWS リージョン"
  type        = string
  default     = "ap-northeast-1"
}

variable "domain_name" {
  description = "SES で検証するドメイン名"
  type        = string
}
