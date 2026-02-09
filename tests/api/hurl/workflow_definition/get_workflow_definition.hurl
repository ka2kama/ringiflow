# ワークフロー定義詳細 API テスト
# GET /api/v1/workflow-definitions/{id}
#
# 指定した ID のワークフロー定義を取得する。

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
# 正常系: ワークフロー定義詳細を取得できる
# =============================================================================
# Given: 公開済みのワークフロー定義（シードデータ）が存在する
# When: ID を指定して GET /api/v1/workflow-definitions/{id} にリクエストする
# Then: ワークフロー定義の詳細が返される

GET {{bff_url}}/api/v1/workflow-definitions/{{workflow_definition_id}}
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Asserts]
# required フィールドすべて検証
jsonpath "$.data.id" == "{{workflow_definition_id}}"
jsonpath "$.data.name" == "汎用申請"
jsonpath "$.data.status" == "Published"
jsonpath "$.data.version" >= 1
jsonpath "$.data.definition" exists
jsonpath "$.data.created_by" matches "^[a-f0-9-]{36}$"
jsonpath "$.data.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"

# nullable フィールド
jsonpath "$.data.description" exists

# =============================================================================
# 異常系: 存在しない ID は 404
# =============================================================================
# Given: 存在しないワークフロー定義 ID
# When: GET /api/v1/workflow-definitions/{id} にリクエストする
# Then: 404 Not Found が返される

GET {{bff_url}}/api/v1/workflow-definitions/99999999-9999-9999-9999-999999999999
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 404
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/workflow-definition-not-found"
jsonpath "$.status" == 404
