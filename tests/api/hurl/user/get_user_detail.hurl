# ユーザー詳細取得 API テスト
# GET /api/v1/users/{display_number}
#
# 表示用連番でユーザー詳細を取得するテスト。

# =============================================================================
# 前提: ログインしてセッションを確立
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
# 正常系: シードデータのユーザー詳細を取得できる
# =============================================================================
# Given: シードデータに admin ユーザー（display_number=1）が存在する
# When: GET /api/v1/users/1 にリクエストする
# Then: ユーザー詳細が返される

GET {{bff_url}}/api/v1/users/1
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Asserts]
# required フィールド
jsonpath "$.id" == "{{admin_id}}"
jsonpath "$.display_id" matches "^USR-\\d{6}$"
jsonpath "$.display_number" == 1
jsonpath "$.name" == "{{admin_name}}"
jsonpath "$.email" == "{{admin_email}}"
jsonpath "$.status" == "active"
jsonpath "$.roles" isCollection
jsonpath "$.roles" count >= 1
jsonpath "$.permissions" isCollection
jsonpath "$.tenant_name" == "Development Tenant"

# =============================================================================
# 異常系: 存在しないユーザーは 404
# =============================================================================
# Given: display_number=99999 のユーザーは存在しない
# When: GET /api/v1/users/99999 にリクエストする
# Then: 404 Not Found が返される

GET {{bff_url}}/api/v1/users/99999
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 404
[Asserts]
jsonpath "$.status" == 404
