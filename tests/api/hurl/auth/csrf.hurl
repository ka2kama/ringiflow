# CSRF トークン取得 API テスト（認証済み）
# GET /api/v1/auth/csrf
#
# 状態変更リクエスト（POST/PUT/PATCH/DELETE）に必要な CSRF トークンを取得する。

# =============================================================================
# 前提: ログインしてセッションを取得
# =============================================================================

POST {{bff_url}}/api/v1/auth/login
X-Tenant-ID: {{tenant_id}}
Content-Type: application/json
{
    "email": "{{admin_email}}",
    "password": "{{password}}"
}

HTTP 200
[Captures]
session_cookie: cookie "session_id"


# =============================================================================
# 正常系: 認証済みユーザーは CSRF トークンを取得できる
# =============================================================================
# Given: ログイン済みのセッションがある
# When: GET /api/v1/auth/csrf にセッション Cookie を送信する
# Then: 64文字の16進数トークンが返される

GET {{bff_url}}/api/v1/auth/csrf
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Asserts]
jsonpath "$.data.token" exists
# 64文字の16進数（256ビット）
jsonpath "$.data.token" matches "^[a-f0-9]{64}$"
