# ワークフロー定義アーカイブ API テスト
# POST /api/v1/workflow-definitions/{id}/archive
#
# 状態遷移テスト: Draft → Published → Archived の全フローを検証する。

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
# セットアップ 1: Draft 定義を作成する
# =============================================================================

POST {{bff_url}}/api/v1/workflow-definitions
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "name": "アーカイブ対象の申請",
    "definition": {
        "steps": [
            {"id": "start", "type": "start", "name": "開始"},
            {"id": "approval_1", "type": "approval", "name": "承認"},
            {"id": "end_approved", "type": "end", "name": "承認完了", "status": "approved"},
            {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
        ],
        "transitions": [
            {"from": "start", "to": "approval_1"},
            {"from": "approval_1", "to": "end_approved", "trigger": "approve"},
            {"from": "approval_1", "to": "end_rejected", "trigger": "reject"}
        ]
    }
}

HTTP 201
[Captures]
def_id: jsonpath "$.data.id"
def_name: jsonpath "$.data.name"
[Asserts]
jsonpath "$.data.status" == "Draft"
jsonpath "$.data.version" == 1

# =============================================================================
# セットアップ 2: Draft → Published に遷移する
# =============================================================================

POST {{bff_url}}/api/v1/workflow-definitions/{{def_id}}/publish
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "version": 1
}

HTTP 200
[Asserts]
jsonpath "$.data.status" == "Published"
jsonpath "$.data.version" == 2

# =============================================================================
# 正常系: Published 状態の定義をアーカイブできる（Published → Archived）
# =============================================================================
# Given: Published 状態のワークフロー定義が存在する（version=2）
# When: POST /api/v1/workflow-definitions/{id}/archive にリクエストする
# Then: Archived 状態に遷移し、200 OK が返される（version=3）

POST {{bff_url}}/api/v1/workflow-definitions/{{def_id}}/archive
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "version": 2
}

HTTP 200
[Asserts]
# required フィールド（WorkflowDefinitionData）
jsonpath "$.data.id" == "{{def_id}}"
jsonpath "$.data.name" == "{{def_name}}"
jsonpath "$.data.version" == 3
jsonpath "$.data.status" == "Archived"
jsonpath "$.data.definition" exists
jsonpath "$.data.created_by" matches "^[a-f0-9-]{36}$"
jsonpath "$.data.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data.updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"

# nullable フィールド
jsonpath "$.data.description" exists

# =============================================================================
# 異常系: 存在しない定義のアーカイブ → 404
# =============================================================================
# Given: 管理者でログインしている
# When: 存在しない UUID でワークフロー定義のアーカイブを試みる
# Then: 404 Not Found が返される

POST {{bff_url}}/api/v1/workflow-definitions/99999999-9999-9999-9999-999999999999/archive
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "version": 1
}

HTTP 404
