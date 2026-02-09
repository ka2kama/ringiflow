# ユーザー一覧 API テスト
# GET /api/v1/users
#
# テナント内のアクティブなユーザー一覧を取得する。

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
# 正常系: ユーザー一覧を取得できる
# =============================================================================
# Given: テナントに複数のアクティブなユーザー（シードデータ）が存在する
# When: GET /api/v1/users にリクエストする
# Then: ユーザーの配列が返される

GET {{bff_url}}/api/v1/users
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Asserts]
# 配列型で 2 件以上（シードデータに admin と user が存在）
jsonpath "$.data" isCollection
jsonpath "$.data" count >= 2

# 各要素の required フィールド（display_number 昇順: admin=1, user=2）
jsonpath "$.data[0].id" == "{{admin_id}}"
jsonpath "$.data[0].display_id" matches "^USER-\\d+$"
jsonpath "$.data[0].display_number" == 1
jsonpath "$.data[0].name" == "{{admin_name}}"
jsonpath "$.data[0].email" == "{{admin_email}}"

jsonpath "$.data[1].id" == "{{user_id}}"
jsonpath "$.data[1].display_id" matches "^USER-\\d+$"
jsonpath "$.data[1].display_number" == 2
jsonpath "$.data[1].name" == "{{user_name}}"
jsonpath "$.data[1].email" == "{{user_email}}"
