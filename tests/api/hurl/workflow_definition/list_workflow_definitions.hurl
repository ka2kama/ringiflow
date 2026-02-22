# ワークフロー定義一覧 API テスト
# GET /api/v1/workflow-definitions
#
# 公開済みのワークフロー定義一覧を取得する。

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
# 正常系: ワークフロー定義一覧を取得できる
# =============================================================================
# Given: 公開済みのワークフロー定義（シードデータ）が存在する
# When: GET /api/v1/workflow-definitions にリクエストする
# Then: ワークフロー定義の配列が返される

GET {{bff_url}}/api/v1/workflow-definitions
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Asserts]
# 配列型（シードデータに「汎用申請」と「2段階承認申請」が存在。他テストの書込で増える場合がある）
jsonpath "$.data" isCollection
jsonpath "$.data" count >= 2

# 先頭要素の構造検証（required フィールドがすべて存在すること）
jsonpath "$.data[0].id" exists
jsonpath "$.data[0].name" isString
jsonpath "$.data[0].status" isString
jsonpath "$.data[0].version" >= 1
jsonpath "$.data[0].definition" exists
jsonpath "$.data[0].created_by" matches "^[a-f0-9-]{36}$"
jsonpath "$.data[0].created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data[0].updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
