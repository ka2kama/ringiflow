# ロール一覧 API テスト
# GET /api/v1/roles
#
# テナント内のロール一覧を取得する。system_admin は除外される。

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
# 正常系: ロール一覧を取得できる
# =============================================================================
# Given: テナントに tenant_admin と user のシステムロールが存在する
# When: GET /api/v1/roles にリクエストする
# Then: system_admin を除くロールの配列が返される

GET {{bff_url}}/api/v1/roles
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Asserts]
# 配列型で 2 件以上（seed データに tenant_admin と user が存在）
jsonpath "$.data" isCollection
jsonpath "$.data" count >= 2

# 各要素の required フィールド（RoleItemData）
# tenant_admin（seed: id=00000000-...-000000000002）
jsonpath "$.data[0].id" == "{{tenant_admin_role_id}}"
jsonpath "$.data[0].name" == "tenant_admin"
jsonpath "$.data[0].permissions" isCollection
jsonpath "$.data[0].permissions" count >= 1
jsonpath "$.data[0].is_system" == true
jsonpath "$.data[0].user_count" >= 1

# user（seed: id=00000000-...-000000000003）
jsonpath "$.data[1].id" == "{{user_role_id}}"
jsonpath "$.data[1].name" == "user"
jsonpath "$.data[1].permissions" isCollection
jsonpath "$.data[1].permissions" count >= 1
jsonpath "$.data[1].is_system" == true
jsonpath "$.data[1].user_count" >= 1
