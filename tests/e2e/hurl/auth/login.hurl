# ログイン API テスト
# POST /auth/login

# -----------------------------------------------------------------------------
# 正常系: 正常ログイン
# -----------------------------------------------------------------------------

POST {{bff_url}}/auth/login
X-Tenant-ID: {{tenant_id}}
Content-Type: application/json
{
    "email": "{{admin_email}}",
    "password": "{{password}}"
}

HTTP 200
[Asserts]
jsonpath "$.data.user.id" exists
jsonpath "$.data.user.email" == "admin@example.com"
jsonpath "$.data.user.name" == "管理者"
jsonpath "$.data.user.tenant_id" == "00000000-0000-0000-0000-000000000001"
jsonpath "$.data.user.roles" exists
header "Set-Cookie" contains "session_id="
header "Set-Cookie" contains "HttpOnly"

# セッション ID をキャプチャ（後続テストで使用）
[Captures]
session_cookie: cookie "session_id"


# -----------------------------------------------------------------------------
# 異常系: パスワード不一致（401）
# -----------------------------------------------------------------------------

POST {{bff_url}}/auth/login
X-Tenant-ID: {{tenant_id}}
Content-Type: application/json
{
    "email": "{{admin_email}}",
    "password": "wrongpassword"
}

HTTP 401
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/authentication-failed"
jsonpath "$.title" == "Authentication Failed"
jsonpath "$.status" == 401


# -----------------------------------------------------------------------------
# 異常系: ユーザー不存在（401）
# -----------------------------------------------------------------------------

POST {{bff_url}}/auth/login
X-Tenant-ID: {{tenant_id}}
Content-Type: application/json
{
    "email": "notfound@example.com",
    "password": "{{password}}"
}

HTTP 401
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/authentication-failed"
jsonpath "$.status" == 401


# -----------------------------------------------------------------------------
# 異常系: テナント ID なし（400）
# -----------------------------------------------------------------------------

POST {{bff_url}}/auth/login
Content-Type: application/json
{
    "email": "{{admin_email}}",
    "password": "{{password}}"
}

HTTP 400
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/validation-error"
jsonpath "$.title" == "Validation Error"
jsonpath "$.status" == 400
