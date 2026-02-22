# ロール削除 API テスト
# DELETE /api/v1/roles/{role_id}
#
# カスタムロールの削除テスト。正常系と存在しない ID エラー（404）を検証する。

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
    "name": "削除テスト用ロール",
    "permissions": ["workflow:read"]
}

HTTP 201
[Captures]
target_role_id: jsonpath "$.data.id"

# =============================================================================
# 正常系: カスタムロールを削除できる
# =============================================================================
# Given: カスタムロールが存在する
# When: DELETE /api/v1/roles/{role_id} にリクエストする
# Then: 204 No Content が返される

DELETE {{bff_url}}/api/v1/roles/{{target_role_id}}
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}

HTTP 204

# =============================================================================
# 異常系: 存在しないロール ID では 404
# =============================================================================
# Given: 指定の UUID に対応するロールは存在しない
# When: DELETE /api/v1/roles/{role_id} にリクエストする
# Then: 404 Not Found が返される

DELETE {{bff_url}}/api/v1/roles/00000000-0000-0000-0000-999999999999
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}

HTTP 404
[Asserts]
jsonpath "$.status" == 404
