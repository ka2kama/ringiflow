# tflint 設定
# https://github.com/terraform-linters/tflint

plugin "aws" {
  enabled = true
  version = "0.45.0"
  source  = "github.com/terraform-linters/tflint-ruleset-aws"
}
