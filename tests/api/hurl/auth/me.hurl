# ユーザー情報取得 API テスト（認証済み）
# GET /api/v1/auth/me
#
# ログイン済みユーザーが自分の情報を取得できることを検証する。

# =============================================================================
# 前提: ログインしてセッションを取得
# =============================================================================

POST {{bff_url}}/api/v1/auth/login
X-Tenant-ID: {{tenant_id}}
Content-Type: application/json
{
    "email": "{{user_email}}",
    "password": "{{password}}"
}

HTTP 200
[Captures]
session_cookie: cookie "session_id"


# =============================================================================
# 正常系: 認証済みユーザーは自分の情報を取得できる
# =============================================================================
# Given: ログイン済みのセッションがある
# When: GET /api/v1/auth/me にセッション Cookie を送信する
# Then: ユーザー情報（ID、メール、名前、ロール、権限）が返される

GET {{bff_url}}/api/v1/auth/me
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Asserts]
jsonpath "$.data.id" exists
jsonpath "$.data.email" == "user@example.com"
jsonpath "$.data.name" == "一般ユーザー"
jsonpath "$.data.tenant_id" == "00000000-0000-0000-0000-000000000001"
jsonpath "$.data.tenant_name" exists
# フロントエンドでの権限チェックに使用
jsonpath "$.data.roles" exists
jsonpath "$.data.permissions" exists
