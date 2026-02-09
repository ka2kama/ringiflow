# ダッシュボード統計 API テスト
# GET /api/v1/dashboard/stats
#
# ログインユーザーのダッシュボード統計情報を取得する。

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
# 正常系: ダッシュボード統計を取得できる
# =============================================================================
# Given: ログインユーザーが存在する
# When: GET /api/v1/dashboard/stats にリクエストする
# Then: 統計情報が返される（各カウントは非負整数）

GET {{bff_url}}/api/v1/dashboard/stats
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Asserts]
# DashboardStats required フィールドすべて検証
# カウント値はテスト実行順序に依存するため >= 0 で検証
jsonpath "$.data.pending_tasks" >= 0
jsonpath "$.data.my_workflows_in_progress" >= 0
jsonpath "$.data.completed_today" >= 0
