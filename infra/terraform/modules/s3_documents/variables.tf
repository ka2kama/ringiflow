variable "environment" {
  description = "環境名（dev, stg, prod）"
  type        = string
}

variable "cors_allowed_origins" {
  description = "CORS で許可するオリジン（Presigned URL アップロード用）"
  type        = list(string)
}
