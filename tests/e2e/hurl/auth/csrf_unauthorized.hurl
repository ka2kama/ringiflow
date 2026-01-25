# CSRF トークン取得 API テスト（未認証）
# GET /auth/csrf
#
# 未認証のリクエストが適切に拒否されることを検証する。

# =============================================================================
# 異常系: 未認証のリクエストは拒否される
# =============================================================================
# Given: ログインしていない（セッション Cookie なし）
# When: GET /auth/csrf にリクエストする
# Then: 401 Unauthorized が返される

GET {{bff_url}}/auth/csrf
X-Tenant-ID: {{tenant_id}}

HTTP 401
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/unauthorized"
jsonpath "$.status" == 401
