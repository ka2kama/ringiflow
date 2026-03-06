variable "aws_region" {
  description = "AWS リージョン"
  type        = string
  default     = "ap-northeast-1"
}

variable "domain_name" {
  description = "SES で検証するドメイン名"
  type        = string
}

variable "cors_allowed_origins" {
  description = "S3 CORS で許可するオリジン"
  type        = list(string)
  default     = ["http://localhost:5173"]
}
