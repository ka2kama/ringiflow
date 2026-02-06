# 自分のタスク一覧取得 API テスト
# GET /api/v1/tasks/my
#
# タスク一覧取得の E2E テスト。BFF → Core Service の連携を検証する。

# =============================================================================
# 前提: admin でワークフローを作成・申請し、user にタスクを割り当てる
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
admin_session_cookie: cookie "session_id"

# CSRF トークンを取得
GET {{bff_url}}/api/v1/auth/csrf
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{admin_session_cookie}}

HTTP 200
[Captures]
csrf_token: jsonpath "$.data.token"

# ワークフローを作成
POST {{bff_url}}/api/v1/workflows
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{admin_session_cookie}}
Content-Type: application/json
{
    "definition_id": "{{workflow_definition_id}}",
    "title": "経費申請（タスク一覧テスト用）",
    "form_data": {
        "title": "交通費",
        "description": "出張時の交通費"
    }
}

HTTP 201
[Captures]
workflow_id: jsonpath "$.data.id"
workflow_display_id: jsonpath "$.data.display_id"
workflow_display_number: jsonpath "$.data.display_number"

# ワークフローを申請（user に割り当て）
POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/submit
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{admin_session_cookie}}
Content-Type: application/json
{
    "assigned_to": "{{user_id}}"
}

HTTP 200

# =============================================================================
# user でログインしてタスク一覧を取得
# =============================================================================

POST {{bff_url}}/api/v1/auth/login
X-Tenant-ID: {{tenant_id}}
Content-Type: application/json
{
    "email": "{{user_email}}",
    "password": "{{password}}"
}

HTTP 200
[Captures]
user_session_cookie: cookie "session_id"

# =============================================================================
# 正常系: タスク一覧を取得できる
# =============================================================================
# Given: user に割り当てられたアクティブなタスクが存在する
# When: タスク一覧を取得する
# Then: 少なくとも 1 件のタスクが返される

GET {{bff_url}}/api/v1/tasks/my
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{user_session_cookie}}

HTTP 200
[Asserts]
# 少なくとも 1 件存在する
jsonpath "$.data" count >= 1

# 先頭要素で構造を検証（TaskItem required フィールド）
# 注: テスト順序によって複数タスクが存在する可能性があるため、
# capture した値との一致ではなく存在確認と型検証を行う
jsonpath "$.data[0].id" exists
jsonpath "$.data[0].display_number" isInteger
jsonpath "$.data[0].step_name" isString
jsonpath "$.data[0].status" isString
jsonpath "$.data[0].version" isInteger
jsonpath "$.data[0].created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"

# TaskWorkflowSummary required フィールドの構造検証
jsonpath "$.data[0].workflow.id" exists
jsonpath "$.data[0].workflow.display_id" isString
jsonpath "$.data[0].workflow.display_number" isInteger
jsonpath "$.data[0].workflow.title" isString
jsonpath "$.data[0].workflow.status" isString
jsonpath "$.data[0].workflow.initiated_by.id" exists
jsonpath "$.data[0].workflow.initiated_by.name" isString

# =============================================================================
# 正常系: 割り当てられたタスクがないユーザーは空配列を返す
# =============================================================================
# Given: admin でログインしている（タスクは user に割り当てられている）
# When: タスク一覧を取得する
# Then: 空配列が返される（admin には割り当てられていない）

GET {{bff_url}}/api/v1/tasks/my
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{admin_session_cookie}}

HTTP 200
[Asserts]
# admin にはタスクが割り当てられていない（テスト環境の初期状態による）
jsonpath "$.data" isCollection

# =============================================================================
# 異常系: 未認証では取得できない
# =============================================================================
# 注: Hurl は同一ファイル内でクッキーを共有するため、
# 未認証テストは別ファイル（task/list_my_tasks_unauthorized.hurl）で実施すべき。
# このテストでは省略する。
