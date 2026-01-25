# ログアウト API テスト
# POST /auth/logout

# -----------------------------------------------------------------------------
# 前提: ログインしてセッションを取得
# -----------------------------------------------------------------------------

POST {{bff_url}}/auth/login
X-Tenant-ID: {{tenant_id}}
Content-Type: application/json
{
    "email": "{{admin_email}}",
    "password": "{{password}}"
}

HTTP 200
[Captures]
session_cookie: cookie "session_id"


# -----------------------------------------------------------------------------
# 正常系: ログアウト
# -----------------------------------------------------------------------------

POST {{bff_url}}/auth/logout
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 204
[Asserts]
# Cookie がクリアされる（Max-Age=0）
header "Set-Cookie" contains "session_id="
header "Set-Cookie" contains "Max-Age=0"


# -----------------------------------------------------------------------------
# ログアウト後は /auth/me が 401 を返す
# -----------------------------------------------------------------------------

GET {{bff_url}}/auth/me
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 401
