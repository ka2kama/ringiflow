# ユーザー更新 API テスト
# PATCH /api/v1/users/{display_number}
#
# ユーザー情報（名前、ロール）を更新するテスト。

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
    "email": "update-test-user@example.com",
    "name": "更新前ユーザー",
    "role_id": "{{user_role_id}}"
}

HTTP 201
[Captures]
target_user_display_number: jsonpath "$.data.display_number"

# =============================================================================
# 正常系: ユーザー名を更新できる
# =============================================================================
# Given: テスト用ユーザーが存在する
# When: 名前を変更する
# Then: 更新が反映され、200 OK が返される

PATCH {{bff_url}}/api/v1/users/{{target_user_display_number}}
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "name": "更新後ユーザー"
}

HTTP 200
[Asserts]
# required フィールド（UserResponseData）
jsonpath "$.data.id" exists
jsonpath "$.data.name" == "更新後ユーザー"
jsonpath "$.data.email" == "update-test-user@example.com"
jsonpath "$.data.status" == "active"

# =============================================================================
# 正常系: ロールを変更できる
# =============================================================================
# Given: テスト用ユーザーが user ロールで作成済み
# When: ロールを tenant_admin に変更する
# Then: 更新が反映され、200 OK が返される

PATCH {{bff_url}}/api/v1/users/{{target_user_display_number}}
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "role_id": "{{tenant_admin_role_id}}"
}

HTTP 200
[Asserts]
jsonpath "$.data.id" exists
jsonpath "$.data.name" == "更新後ユーザー"
jsonpath "$.data.email" == "update-test-user@example.com"
jsonpath "$.data.status" == "active"

# =============================================================================
# 異常系: 存在しないユーザーの更新 → 404
# =============================================================================
# Given: 管理者でログインしている
# When: 存在しない display_number でユーザー更新を試みる
# Then: 404 Not Found が返される

PATCH {{bff_url}}/api/v1/users/99999999
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "name": "存在しないユーザー"
}

HTTP 404
