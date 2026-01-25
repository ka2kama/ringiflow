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
# CSRF トークンを取得
# -----------------------------------------------------------------------------

GET {{bff_url}}/auth/csrf
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Captures]
csrf_token: jsonpath "$.data.token"


# -----------------------------------------------------------------------------
# 正常系: ログアウト（CSRF トークン付き）
# -----------------------------------------------------------------------------

POST {{bff_url}}/auth/logout
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
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
