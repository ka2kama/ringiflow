# SES ドメイン検証モジュール
#
# ドメインの Email Identity を作成し、DKIM 署名を有効化する。
# DKIM レコードは outputs で出力され、DNS に手動で設定する。
#
# 参照: docs/40_詳細設計書/16_通知機能設計.md

# ドメインの Email Identity（DKIM 署名付き）
resource "aws_sesv2_email_identity" "domain" {
  email_identity = var.domain_name

  dkim_signing_attributes {
    next_signing_key_length = "RSA_2048_BIT"
  }
}

# Configuration Set（送信メトリクス追跡）
resource "aws_sesv2_configuration_set" "main" {
  configuration_set_name = "ringiflow-${var.environment}"

  reputation_options {
    reputation_metrics_enabled = true
  }

  sending_options {
    sending_enabled = true
  }
}
