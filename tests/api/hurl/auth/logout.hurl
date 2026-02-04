# ログアウト API テスト
# POST /api/v1/auth/logout
#
# ログアウト機能とセッション無効化を検証する。

# =============================================================================
# 前提: ログインしてセッションを取得
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
# 前提: CSRF トークンを取得（状態変更リクエストに必要）
# =============================================================================

GET {{bff_url}}/api/v1/auth/csrf
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 200
[Captures]
csrf_token: jsonpath "$.data.token"


# =============================================================================
# 正常系: ログアウトでセッションが無効化される
# =============================================================================
# Given: ログイン済みのセッションと CSRF トークンがある
# When: POST /api/v1/auth/logout に CSRF トークンを付けてリクエストする
# Then: 204 No Content が返され、セッション Cookie がクリアされる

POST {{bff_url}}/api/v1/auth/logout
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}

HTTP 204
[Asserts]
# Cookie がクリアされる（Max-Age=0 で無効化、セキュリティ属性維持）
header "Set-Cookie" contains "session_id="
header "Set-Cookie" contains "Max-Age=0"
header "Set-Cookie" contains "HttpOnly"
header "Set-Cookie" contains "SameSite=Lax"
header "Set-Cookie" contains "Path=/"


# =============================================================================
# 確認: ログアウト後は認証が必要なエンドポイントにアクセスできない
# =============================================================================
# Given: ログアウト済み（セッションが無効化された）
# When: 古いセッション Cookie で /api/v1/auth/me にアクセスする
# Then: 401 Unauthorized が返される（セッションが無効なため）

GET {{bff_url}}/api/v1/auth/me
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}

HTTP 401
