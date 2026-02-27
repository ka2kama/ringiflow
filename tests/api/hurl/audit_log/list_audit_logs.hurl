# 監査ログ一覧 API テスト
# GET /api/v1/audit-logs
#
# テナント内の監査ログ一覧を取得する（カーソルベースページネーション）。
# DynamoDB にはシードデータがないため、テスト内でロール作成を行い監査ログを生成する。

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
# 準備: 監査ログを生成するためにロールを 2 件作成する
# =============================================================================

POST {{bff_url}}/api/v1/roles
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "name": "監査ログテスト用ロール1",
    "permissions": ["workflow:read"]
}

HTTP 201

POST {{bff_url}}/api/v1/roles
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "name": "監査ログテスト用ロール2",
    "permissions": ["task:read"]
}

HTTP 201

# =============================================================================
# 正常系: 監査ログ一覧を取得できる
# =============================================================================
# Given: ロール作成操作により監査ログが 2 件以上存在する
# When: GET /api/v1/audit-logs にリクエストする
# Then: 監査ログの配列が返され、required フィールドがすべて含まれる

GET {{bff_url}}/api/v1/audit-logs
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Asserts]
# PaginatedResponse 構造
jsonpath "$.data" isCollection
jsonpath "$.data" count >= 2

# 最新エントリ（data[0]）の required フィールド検証（AuditLogItemData）
# 監査ログは新しい順 → data[0] は直近のロール作成
jsonpath "$.data[0].id" exists
jsonpath "$.data[0].actor_id" == "{{admin_id}}"
jsonpath "$.data[0].actor_name" == "{{admin_name}}"
jsonpath "$.data[0].action" == "role.create"
jsonpath "$.data[0].result" == "success"
jsonpath "$.data[0].resource_type" == "role"
jsonpath "$.data[0].resource_id" exists
jsonpath "$.data[0].created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"

# =============================================================================
# 正常系: limit パラメータでページネーションが機能する
# =============================================================================
# Given: 監査ログが 2 件以上存在する
# When: limit=1 で GET /api/v1/audit-logs にリクエストする
# Then: 1 件のみ返され、next_cursor が非 null である

GET {{bff_url}}/api/v1/audit-logs?limit=1
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Asserts]
jsonpath "$.data" isCollection
jsonpath "$.data" count == 1
jsonpath "$.next_cursor" != null

# =============================================================================
# 異常系: ページネーション無効値 — limit=0
# =============================================================================
# Given: 監査ログが存在する
# When: limit=0 で GET /api/v1/audit-logs にリクエストする
# Then: 安全なデフォルト値にフォールバックして 200 で応答する

GET {{bff_url}}/api/v1/audit-logs?limit=0
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Asserts]
jsonpath "$.data" isCollection

# =============================================================================
# 異常系: ページネーション無効値 — limit=-1
# =============================================================================
# Given: 監査ログが存在する
# When: limit=-1 で GET /api/v1/audit-logs にリクエストする
# Then: 安全なデフォルト値にフォールバックして 200 で応答する

GET {{bff_url}}/api/v1/audit-logs?limit=-1
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Asserts]
jsonpath "$.data" isCollection

# =============================================================================
# 準正常系: 不正な cursor パラメータで 400 Bad Request が返る
# =============================================================================
# Given: 認証済みユーザー
# When: 不正な cursor 文字列で GET /api/v1/audit-logs にリクエストする
# Then: 400 Bad Request（RFC 9457 形式）が返される

GET {{bff_url}}/api/v1/audit-logs?cursor=invalid-cursor-string
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 400
[Asserts]
jsonpath "$.type" endsWith "/validation-error"
jsonpath "$.title" == "Validation Error"
jsonpath "$.status" == 400
jsonpath "$.detail" == "カーソルの形式が不正です"
