# CSRF トークン取得 API テスト
# GET /auth/csrf

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
# 正常系: CSRF トークンを取得
# -----------------------------------------------------------------------------

GET {{bff_url}}/auth/csrf
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Asserts]
jsonpath "$.data.token" exists
jsonpath "$.data.token" matches "^[a-f0-9]{64}$"


# -----------------------------------------------------------------------------
# 異常系: 未認証で 401
# -----------------------------------------------------------------------------

GET {{bff_url}}/auth/csrf
X-Tenant-ID: {{tenant_id}}

HTTP 401
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/unauthorized"
jsonpath "$.status" == 401
