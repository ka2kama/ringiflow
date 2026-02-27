# ユーザーステータス変更 API テスト
# PATCH /api/v1/users/{display_number}/status
#
# ユーザーのステータスを変更するテスト（active → inactive）。

# =============================================================================
# 前提: ログインしてセッションを確立し、テスト用ユーザーを作成
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

# CSRF トークンを取得
GET {{bff_url}}/api/v1/auth/csrf
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Captures]
csrf_token: jsonpath "$.data.token"

# テスト用ユーザーを作成
POST {{bff_url}}/api/v1/users
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "email": "status-test-user@example.com",
    "name": "ステータステスト",
    "role_id": "{{user_role_id}}"
}

HTTP 201
[Captures]
target_user_display_number: jsonpath "$.data.display_number"

# =============================================================================
# 正常系: ユーザーを無効化できる
# =============================================================================
# Given: active なテスト用ユーザーが存在する
# When: ステータスを inactive に変更する
# Then: ステータスが変更され、200 OK が返される

PATCH {{bff_url}}/api/v1/users/{{target_user_display_number}}/status
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "status": "inactive"
}

HTTP 200
[Asserts]
# required フィールド（UserResponseData）
jsonpath "$.data.id" exists
jsonpath "$.data.name" == "ステータステスト"
jsonpath "$.data.email" == "status-test-user@example.com"
jsonpath "$.data.status" == "inactive"

# =============================================================================
# 正常系: ユーザーを再有効化できる
# =============================================================================
# Given: inactive なテスト用ユーザーが存在する
# When: ステータスを active に変更する
# Then: ステータスが変更され、200 OK が返される

PATCH {{bff_url}}/api/v1/users/{{target_user_display_number}}/status
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "status": "active"
}

HTTP 200
[Asserts]
jsonpath "$.data.id" exists
jsonpath "$.data.name" == "ステータステスト"
jsonpath "$.data.email" == "status-test-user@example.com"
jsonpath "$.data.status" == "active"

# =============================================================================
# 異常系: 存在しないユーザーのステータス変更 → 404
# =============================================================================
# Given: 管理者でログインしている
# When: 存在しない display_number でステータス変更を試みる
# Then: 404 Not Found が返される

PATCH {{bff_url}}/api/v1/users/99999999/status
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "status": "inactive"
}

HTTP 404
