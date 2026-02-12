# ワークフローコメント API テスト
# POST /api/v1/workflows/{display_number}/comments
# GET /api/v1/workflows/{display_number}/comments
#
# コメントスレッド機能の E2E テスト。BFF → Core Service の連携を検証する。
# 正常系（申請者・承認者がコメント投稿、一覧取得）
# + 異常系（CSRF エラー、権限エラー、Not Found、バリデーションエラー）

# =============================================================================
# 前提: admin でワークフローを作成・申請し、user に承認タスクを割り当てる
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
admin_csrf_token: jsonpath "$.data.token"

# ワークフローを作成
POST {{bff_url}}/api/v1/workflows
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_cookie}}
Content-Type: application/json
{
    "definition_id": "{{workflow_definition_id}}",
    "title": "経費申請（コメントテスト用）",
    "form_data": {
        "title": "交通費",
        "description": "出張時の交通費"
    }
}

HTTP 201
[Captures]
workflow_display_number: jsonpath "$.data.display_number"

# ワークフローを申請（user に割り当て）
POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/submit
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_cookie}}
Content-Type: application/json
{
    "approvers": [
        { "step_id": "approval", "assigned_to": "{{user_id}}" }
    ]
}

HTTP 200

# =============================================================================
# 正常系: 申請者（admin）がコメントを投稿できる
# =============================================================================
# Given: admin が申請したワークフロー
# When: admin がコメントを投稿する
# Then: 201 Created + コメントデータが返される

POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/comments
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_cookie}}
Content-Type: application/json
{
    "body": "承認をお願いします。至急対応ください。"
}

HTTP 201
[Asserts]
jsonpath "$.data.id" matches "^[0-9a-f]{8}-"
jsonpath "$.data.posted_by.id" == "{{admin_id}}"
jsonpath "$.data.posted_by.name" == "{{admin_name}}"
jsonpath "$.data.body" == "承認をお願いします。至急対応ください。"
jsonpath "$.data.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"

# =============================================================================
# 異常系: CSRF トークンなしではコメントを投稿できない
# =============================================================================
# Given: ログインしているが CSRF トークンがない
# When: コメント投稿を試みる
# Then: 403 Forbidden が返される

POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/comments
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{admin_session_cookie}}
Content-Type: application/json
{
    "body": "CSRF なしのコメント"
}

HTTP 403
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/csrf-validation-failed"
jsonpath "$.status" == 403

# =============================================================================
# 異常系: 空文字のコメント本文はエラー
# =============================================================================
# Given: コメント本文が空文字
# When: コメント投稿を試みる
# Then: 400 Bad Request が返される

POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/comments
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_cookie}}
Content-Type: application/json
{
    "body": ""
}

HTTP 400
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/validation-error"
jsonpath "$.status" == 400

# =============================================================================
# user でログインしてコメントを投稿
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

# CSRF トークンを取得
GET {{bff_url}}/api/v1/auth/csrf
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{user_session_cookie}}

HTTP 200
[Captures]
user_csrf_token: jsonpath "$.data.token"

# =============================================================================
# 正常系: 承認者（user）がコメントを投稿できる
# =============================================================================
# Given: user がワークフローの承認者である
# When: user がコメントを投稿する
# Then: 201 Created + コメントデータが返される

POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/comments
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{user_csrf_token}}
Cookie: session_id={{user_session_cookie}}
Content-Type: application/json
{
    "body": "確認しました。問題ありません。"
}

HTTP 201
[Asserts]
jsonpath "$.data.id" matches "^[0-9a-f]{8}-"
jsonpath "$.data.posted_by.id" == "{{user_id}}"
jsonpath "$.data.posted_by.name" == "{{user_name}}"
jsonpath "$.data.body" == "確認しました。問題ありません。"
jsonpath "$.data.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"

# =============================================================================
# 正常系: コメント一覧を時系列で取得できる
# =============================================================================
# Given: admin と user がそれぞれコメントを投稿済み
# When: コメント一覧を取得する
# Then: 200 OK + 2件のコメントが時系列昇順で返される

GET {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/comments
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{admin_session_cookie}}

HTTP 200
[Asserts]
jsonpath "$.data" count == 2
# 1件目: admin のコメント（先に投稿）
jsonpath "$.data[0].posted_by.id" == "{{admin_id}}"
jsonpath "$.data[0].posted_by.name" == "{{admin_name}}"
jsonpath "$.data[0].body" == "承認をお願いします。至急対応ください。"
jsonpath "$.data[0].created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
# 2件目: user のコメント（後に投稿）
jsonpath "$.data[1].posted_by.id" == "{{user_id}}"
jsonpath "$.data[1].posted_by.name" == "{{user_name}}"
jsonpath "$.data[1].body" == "確認しました。問題ありません。"
jsonpath "$.data[1].created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"

# =============================================================================
# 異常系: 存在しないワークフローにはコメントできない
# =============================================================================
# Given: 存在しない display_number
# When: コメント投稿を試みる
# Then: 404 Not Found が返される

POST {{bff_url}}/api/v1/workflows/99999999/comments
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{admin_csrf_token}}
Cookie: session_id={{admin_session_cookie}}
Content-Type: application/json
{
    "body": "存在しないワークフローへのコメント"
}

HTTP 404
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/workflow-instance-not-found"
jsonpath "$.status" == 404

# =============================================================================
# 異常系: 存在しないワークフローのコメント一覧は取得できない
# =============================================================================
# Given: 存在しない display_number
# When: コメント一覧取得を試みる
# Then: 404 Not Found が返される

GET {{bff_url}}/api/v1/workflows/99999999/comments
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{admin_session_cookie}}

HTTP 404
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/workflow-instance-not-found"
jsonpath "$.status" == 404
