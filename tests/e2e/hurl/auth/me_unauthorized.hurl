# 現在のユーザー情報取得 API テスト（未認証）
# GET /auth/me

# -----------------------------------------------------------------------------
# 異常系: 未認証（401）
# Cookie なしでリクエスト
# -----------------------------------------------------------------------------

GET {{bff_url}}/auth/me
X-Tenant-ID: {{tenant_id}}

HTTP 401
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/unauthorized"
jsonpath "$.title" == "Unauthorized"
jsonpath "$.status" == 401
