# レディネスチェック API テスト
# GET /health/ready
#
# 依存サービス（DB、Redis、Core API）を含めた稼働状態を確認する。認証不要。

# =============================================================================
# 正常系: 全依存サービスが稼働中
# =============================================================================
# Given: BFF、Core Service、DB、Redis が起動している
# When: GET /health/ready にリクエストする
# Then: "ready" ステータスと各サービスの稼働状態が返される

GET {{bff_url}}/health/ready

HTTP 200
[Asserts]
jsonpath "$.status" == "ready"
jsonpath "$.checks.database" == "ok"
jsonpath "$.checks.redis" == "ok"
jsonpath "$.checks.core_api" == "ok"
