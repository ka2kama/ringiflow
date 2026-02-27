# ユーザー作成 API テスト
# POST /api/v1/users
#
# ユーザー作成のテスト。BFF → Core Service の連携を検証する。

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
# 正常系: ユーザーを作成できる
# =============================================================================
# Given: 管理者でログインしている
# When: 有効なデータでユーザーを作成する
# Then: ユーザーが作成され、201 Created が返される

POST {{bff_url}}/api/v1/users
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "email": "new-test-user@example.com",
    "name": "テストユーザー",
    "role_id": "{{user_role_id}}"
}

HTTP 201
[Asserts]
# required フィールド
jsonpath "$.data.id" exists
jsonpath "$.data.display_id" matches "^USER-\\d+$"
jsonpath "$.data.display_number" >= 1
jsonpath "$.data.name" == "テストユーザー"
jsonpath "$.data.email" == "new-test-user@example.com"
jsonpath "$.data.role" == "user"
jsonpath "$.data.initial_password" exists

[Captures]
new_user_display_number: jsonpath "$.data.display_number"

# =============================================================================
# 異常系: バリデーションエラー（email なし）
# =============================================================================
# Given: 管理者でログインしている
# When: email なしでユーザー作成を試みる
# Then: 422 Unprocessable Entity が返される（axum の Json rejection デフォルト）

POST {{bff_url}}/api/v1/users
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "name": "テストユーザー2",
    "role_id": "{{user_role_id}}"
}

HTTP 422

# =============================================================================
# 異常系: CSRF トークンなしでは作成できない
# =============================================================================
# Given: ログインしているが CSRF トークンがない
# When: ユーザー作成を試みる
# Then: 403 Forbidden が返される

POST {{bff_url}}/api/v1/users
X-Tenant-ID: {{tenant_id}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "email": "no-csrf@example.com",
    "name": "CSRF テスト",
    "role_id": "{{user_role_id}}"
}

HTTP 403
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/csrf-validation-failed"
jsonpath "$.status" == 403

# =============================================================================
# 異常系: 入力値境界 — 空文字の名前
# =============================================================================
# Given: 管理者でログインしている
# When: 空文字の名前でユーザー作成を試みる
# Then: 400 Bad Request が返される（UserName の空文字バリデーション）

POST {{bff_url}}/api/v1/users
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "email": "empty-name@example.com",
    "name": "",
    "role_id": "{{user_role_id}}"
}

HTTP 400
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/validation-error"
jsonpath "$.status" == 400

# =============================================================================
# 異常系: 入力値境界 — 最大長超過の名前（101文字）
# =============================================================================
# Given: 管理者でログインしている
# When: 101 文字の名前でユーザー作成を試みる
# Then: 400 Bad Request が返される（UserName の最大長バリデーション: 100 文字）

POST {{bff_url}}/api/v1/users
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "email": "long-name@example.com",
    "name": "あいうえおかきくけこあいうえおかきくけこあいうえおかきくけこあいうえおかきくけこあいうえおかきくけこあいうえおかきくけこあいうえおかきくけこあいうえおかきくけこあいうえおかきくけこあいうえおかきくけこa",
    "role_id": "{{user_role_id}}"
}

HTTP 400
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/validation-error"
jsonpath "$.status" == 400

# =============================================================================
# 正常系: 入力値境界 — HTML タグを含む名前（500 にならないことを確認）
# =============================================================================
# Given: 管理者でログインしている
# When: HTML タグを含む名前でユーザーを作成する
# Then: 201 Created が返される（特殊文字はバリデーションを通過する）

POST {{bff_url}}/api/v1/users
X-Tenant-ID: {{tenant_id}}
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
Content-Type: application/json
{
    "email": "html-name@example.com",
    "name": "<b>テスト</b>ユーザー",
    "role_id": "{{user_role_id}}"
}

HTTP 201
[Asserts]
jsonpath "$.data.name" == "<b>テスト</b>ユーザー"
