# ワークフロー作成 API テスト
# POST /api/v1/workflows
#
# ワークフロー作成（下書き）の E2E テスト。BFF → Core Service の連携を検証する。

# =============================================================================
# 前提: ログインしてセッションを確立
# =============================================================================

POST {{bff_url}}/auth/login
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
GET {{bff_url}}/auth/csrf
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Captures]
csrf_token: jsonpath "$.data.token"

# =============================================================================
# 正常系: ワークフローを作成できる
# =============================================================================
# Given: 公開済みのワークフロー定義が存在する
# When: 有効なフォームデータでワークフローを作成する
# Then: ワークフローインスタンスが作成され、201 Created が返される

POST {{bff_url}}/api/v1/workflows
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "definition_id": "{{workflow_definition_id}}",
    "title": "経費申請",
    "form_data": {
        "title": "交通費",
        "description": "出張時の交通費"
    }
}

HTTP 201
[Asserts]
# ワークフローインスタンスの情報が正しく返される
jsonpath "$.data.id" exists
jsonpath "$.data.title" == "経費申請"
jsonpath "$.data.definition_id" == "{{workflow_definition_id}}"
jsonpath "$.data.status" == "Draft"
jsonpath "$.data.form_data.title" == "交通費"
jsonpath "$.data.form_data.description" == "出張時の交通費"
jsonpath "$.data.initiated_by" == "{{admin_id}}"
jsonpath "$.data.current_step_id" == null
jsonpath "$.data.submitted_at" == null
jsonpath "$.data.created_at" exists
jsonpath "$.data.updated_at" exists

[Captures]
workflow_id: jsonpath "$.data.id"

# =============================================================================
# 異常系: 未認証では作成できない（CSRF トークンなしで 403）
# =============================================================================
# Given: ログインしていない
# When: ワークフロー作成を試みる
# Then: 403 Forbidden が返される（CSRF ミドルウェアが先にチェック）

POST {{bff_url}}/api/v1/workflows
X-Tenant-ID: {{tenant_id}}
Content-Type: application/json
{
    "definition_id": "{{workflow_definition_id}}",
    "title": "経費申請",
    "form_data": {}
}

HTTP 403
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/csrf-validation-failed"
jsonpath "$.status" == 403

# =============================================================================
# 異常系: CSRF トークンなしでは作成できない
# =============================================================================
# Given: ログインしているが CSRF トークンがない
# When: ワークフロー作成を試みる
# Then: 403 Forbidden が返される

POST {{bff_url}}/api/v1/workflows
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "definition_id": "{{workflow_definition_id}}",
    "title": "経費申請",
    "form_data": {}
}

HTTP 403
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/csrf-validation-failed"
jsonpath "$.status" == 403

# =============================================================================
# 異常系: 存在しないワークフロー定義では作成できない
# =============================================================================
# Given: 存在しないワークフロー定義 ID
# When: ワークフロー作成を試みる
# Then: 404 Not Found が返される

POST {{bff_url}}/api/v1/workflows
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "definition_id": "99999999-9999-9999-9999-999999999999",
    "title": "経費申請",
    "form_data": {}
}

HTTP 404
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/workflow-definition-not-found"
jsonpath "$.status" == 404
