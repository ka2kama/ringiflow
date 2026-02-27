# ロール更新 API テスト
# PATCH /api/v1/roles/{role_id}
#
# カスタムロールの更新テスト。名前と権限の変更を検証する。

# =============================================================================
# 前提: ログインしてセッションを確立し、テスト用ロールを作成
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

# テスト用カスタムロールを作成
POST {{bff_url}}/api/v1/roles
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "name": "更新テスト用ロール",
    "description": "更新前の説明",
    "permissions": ["workflow:read"]
}

HTTP 201
[Captures]
target_role_id: jsonpath "$.data.id"

# =============================================================================
# 正常系: ロール名と権限を更新できる
# =============================================================================
# Given: カスタムロールが存在する
# When: 名前と権限を変更する
# Then: 更新が反映され、200 OK が返される

PATCH {{bff_url}}/api/v1/roles/{{target_role_id}}
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "name": "更新後ロール",
    "description": "更新後の説明",
    "permissions": ["workflow:read", "workflow:create", "task:read"]
}

HTTP 200
[Asserts]
# required フィールド（RoleDetailData）
jsonpath "$.data.id" == "{{target_role_id}}"
jsonpath "$.data.name" == "更新後ロール"
jsonpath "$.data.description" == "更新後の説明"
jsonpath "$.data.permissions" isCollection
jsonpath "$.data.permissions" count == 3
jsonpath "$.data.is_system" == false
jsonpath "$.data.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"

# =============================================================================
# 異常系: 存在しないロールの更新 → 404
# =============================================================================
# Given: 管理者でログインしている
# When: 存在しない UUID でロール更新を試みる
# Then: 404 Not Found が返される

PATCH {{bff_url}}/api/v1/roles/99999999-9999-9999-9999-999999999999
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "name": "存在しないロール",
    "permissions": ["workflow:read"]
}

HTTP 404
