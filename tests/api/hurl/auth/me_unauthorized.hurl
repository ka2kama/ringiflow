# ユーザー情報取得 API テスト（未認証）
# GET /api/v1/auth/me
#
# 未認証のリクエストが適切に拒否されることを検証する。

# =============================================================================
# 異常系: 未認証のリクエストは拒否される
# =============================================================================
# Given: ログインしていない（セッション Cookie なし）
# When: GET /api/v1/auth/me にリクエストする
# Then: 401 Unauthorized が返される

GET {{bff_url}}/api/v1/auth/me
X-Tenant-ID: {{tenant_id}}

HTTP 401
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/unauthorized"
jsonpath "$.title" == "Unauthorized"
jsonpath "$.status" == 401
