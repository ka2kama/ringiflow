# 現在のユーザー情報取得 API テスト
# GET /auth/me

# -----------------------------------------------------------------------------
# 前提: ログインしてセッションを取得
# -----------------------------------------------------------------------------

POST {{bff_url}}/auth/login
X-Tenant-ID: {{tenant_id}}
Content-Type: application/json
{
    "email": "{{user_email}}",
    "password": "{{password}}"
}

HTTP 200
[Captures]
session_cookie: cookie "session_id"


# -----------------------------------------------------------------------------
# 正常系: 認証済みでユーザー情報を取得
# -----------------------------------------------------------------------------

GET {{bff_url}}/auth/me
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Asserts]
jsonpath "$.data.id" exists
jsonpath "$.data.email" == "user@example.com"
jsonpath "$.data.name" == "一般ユーザー"
jsonpath "$.data.tenant_id" == "00000000-0000-0000-0000-000000000001"
jsonpath "$.data.tenant_name" exists
jsonpath "$.data.roles" exists
jsonpath "$.data.permissions" exists


# -----------------------------------------------------------------------------
# 異常系: 未認証（401）
# -----------------------------------------------------------------------------

GET {{bff_url}}/auth/me
X-Tenant-ID: {{tenant_id}}

HTTP 401
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/unauthorized"
jsonpath "$.title" == "Unauthorized"
jsonpath "$.status" == 401
