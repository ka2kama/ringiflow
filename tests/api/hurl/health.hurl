# ヘルスチェック API テスト
# GET /health
#
# サービスの稼働状態を確認する。認証不要。

# =============================================================================
# 正常系: サービスが稼働している
# =============================================================================
# Given: BFF サービスが起動している
# When: GET /health にリクエストする
# Then: "healthy" ステータスとバージョン情報が返される

GET {{bff_url}}/health

HTTP 200
[Asserts]
header "x-request-id" matches "^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$"
jsonpath "$.status" == "healthy"
jsonpath "$.version" exists
