# 自分の申請一覧 API テスト
# GET /api/v1/workflows
#
# ログインユーザーが申請したワークフロー一覧を取得する。

# =============================================================================
# 前提: ログインしてワークフローを作成
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

# ワークフローを作成（一覧に含まれることを保証するため）
POST {{bff_url}}/api/v1/workflows
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "definition_id": "{{workflow_definition_id}}",
    "title": "経費申請（一覧テスト用）",
    "form_data": {
        "title": "交通費",
        "description": "出張時の交通費"
    }
}

HTTP 201

# =============================================================================
# 正常系: 自分の申請一覧を取得できる
# =============================================================================
# Given: ログインユーザーが申請したワークフローが存在する
# When: GET /api/v1/workflows にリクエストする
# Then: ワークフローの配列が返される

GET {{bff_url}}/api/v1/workflows
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Asserts]
# 配列型で 1 件以上
jsonpath "$.data" isCollection
jsonpath "$.data" count >= 1

# 先頭要素の WorkflowInstance required フィールド検証
# 注: 一覧にはシードデータ + テスト作成分が含まれるため、
# data[0] がどのワークフローかは不定。構造検証 + 決定的なフィールドのみ == を使用
jsonpath "$.data[0].id" matches "^[a-f0-9-]{36}$"
jsonpath "$.data[0].display_id" matches "^WF-\\d+$"
jsonpath "$.data[0].display_number" >= 1
jsonpath "$.data[0].title" isString
jsonpath "$.data[0].definition_id" matches "^[a-f0-9-]{36}$"
jsonpath "$.data[0].status" isString
jsonpath "$.data[0].version" >= 1
jsonpath "$.data[0].form_data" exists
jsonpath "$.data[0].initiated_by.id" == "{{admin_id}}"
jsonpath "$.data[0].initiated_by.name" == "{{admin_name}}"
jsonpath "$.data[0].created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
jsonpath "$.data[0].updated_at" matches "^\\d{4}-\\d{2}-\\d{2}T"
