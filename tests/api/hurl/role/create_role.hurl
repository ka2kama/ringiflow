# ロール作成 API テスト
# POST /api/v1/roles
#
# カスタムロールの作成テスト。正常系と重複名エラー（409）を検証する。

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

# CSRF トークンを取得
GET {{bff_url}}/api/v1/auth/csrf
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Captures]
csrf_token: jsonpath "$.data.token"

# =============================================================================
# 正常系: カスタムロールを作成できる
# =============================================================================
# Given: 管理者でログインしている
# When: 有効なデータでロールを作成する
# Then: ロールが作成され、201 Created が返される

POST {{bff_url}}/api/v1/roles
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "name": "テストロール",
    "description": "API テスト用のカスタムロール",
    "permissions": ["workflow:read", "task:read"]
}

HTTP 201
[Asserts]
# required フィールド（RoleDetailData）
jsonpath "$.data.id" exists
jsonpath "$.data.name" == "テストロール"
jsonpath "$.data.description" == "API テスト用のカスタムロール"
jsonpath "$.data.permissions" isCollection
jsonpath "$.data.permissions" count == 2
jsonpath "$.data.is_system" == false
jsonpath "$.data.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"

# =============================================================================
# 異常系: 同名のロールは作成できない（409 Conflict）
# =============================================================================
# Given: 「テストロール」が既に作成されている
# When: 同じ名前でロール作成を試みる
# Then: 409 Conflict が返される

POST {{bff_url}}/api/v1/roles
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "name": "テストロール",
    "permissions": ["workflow:read"]
}

HTTP 409
[Asserts]
jsonpath "$.status" == 409
