# ロール詳細取得 API テスト
# GET /api/v1/roles/{role_id}
#
# ロール ID でロール詳細を取得するテスト。

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
# 正常系: シードデータのロール詳細を取得できる
# =============================================================================
# Given: シードデータに tenant_admin ロールが存在する
# When: GET /api/v1/roles/{role_id} にリクエストする
# Then: ロール詳細が返される

GET {{bff_url}}/api/v1/roles/{{tenant_admin_role_id}}
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Asserts]
# required フィールド（RoleDetailData）
jsonpath "$.data.id" == "{{tenant_admin_role_id}}"
jsonpath "$.data.name" == "tenant_admin"
jsonpath "$.data.description" == "テナント管理者"
jsonpath "$.data.permissions" isCollection
jsonpath "$.data.permissions" count >= 1
jsonpath "$.data.is_system" == true
jsonpath "$.data.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"

# =============================================================================
# 異常系: 存在しないロール ID は 404
# =============================================================================
# Given: 指定の UUID に対応するロールは存在しない
# When: GET /api/v1/roles/{role_id} にリクエストする
# Then: 404 Not Found が返される

GET {{bff_url}}/api/v1/roles/00000000-0000-0000-0000-999999999999
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 404
[Asserts]
jsonpath "$.status" == 404
