# ヘルスチェック API テスト
# GET /health

GET {{bff_url}}/health

HTTP 200
[Asserts]
jsonpath "$.status" == "healthy"
jsonpath "$.version" exists
