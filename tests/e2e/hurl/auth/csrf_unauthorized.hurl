# CSRF トークン取得 API テスト（未認証）
# GET /auth/csrf

# -----------------------------------------------------------------------------
# 異常系: 未認証で 401
# Cookie なしでリクエスト
# -----------------------------------------------------------------------------

GET {{bff_url}}/auth/csrf
X-Tenant-ID: {{tenant_id}}

HTTP 401
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/unauthorized"
jsonpath "$.status" == 401
