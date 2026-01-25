# ログイン API テスト
# POST /auth/login
#
# ログイン機能の E2E テスト。BFF → Core API → Auth Service の連携を検証する。

# =============================================================================
# 正常系: 有効な認証情報でログインできる
# =============================================================================
# Given: シードデータの管理者ユーザー（admin@example.com）が存在する
# When: 正しいメールアドレスとパスワードでログインする
# Then: ユーザー情報とセッション Cookie が返される

POST {{bff_url}}/auth/login
X-Tenant-ID: {{tenant_id}}
Content-Type: application/json
{
    "email": "{{admin_email}}",
    "password": "{{password}}"
}

HTTP 200
[Asserts]
# ユーザー情報が正しく返される
jsonpath "$.data.user.id" exists
jsonpath "$.data.user.email" == "admin@example.com"
jsonpath "$.data.user.name" == "管理者"
jsonpath "$.data.user.tenant_id" == "00000000-0000-0000-0000-000000000001"
jsonpath "$.data.user.roles" exists
# セッション Cookie が HttpOnly で設定される（XSS 対策）
header "Set-Cookie" contains "session_id="
header "Set-Cookie" contains "HttpOnly"


# =============================================================================
# 異常系: パスワードが間違っている場合は認証失敗
# =============================================================================
# Given: 有効なユーザーが存在する
# When: 間違ったパスワードでログインする
# Then: 401 Unauthorized が返される（セキュリティのためユーザー存在は漏らさない）

POST {{bff_url}}/auth/login
X-Tenant-ID: {{tenant_id}}
Content-Type: application/json
{
    "email": "{{admin_email}}",
    "password": "wrongpassword"
}

HTTP 401
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/authentication-failed"
jsonpath "$.title" == "Authentication Failed"
jsonpath "$.status" == 401


# =============================================================================
# 異常系: 存在しないユーザーでも同じエラーを返す
# =============================================================================
# Given: 存在しないメールアドレス
# When: ログインを試みる
# Then: 401 Unauthorized が返される（タイミング攻撃対策として同じレスポンス）

POST {{bff_url}}/auth/login
X-Tenant-ID: {{tenant_id}}
Content-Type: application/json
{
    "email": "notfound@example.com",
    "password": "{{password}}"
}

HTTP 401
[Asserts]
# パスワード不一致と同じエラーを返す（ユーザー列挙攻撃の防止）
jsonpath "$.type" == "https://ringiflow.example.com/errors/authentication-failed"
jsonpath "$.status" == 401


# =============================================================================
# 異常系: テナント ID ヘッダーが必須
# =============================================================================
# Given: マルチテナント環境
# When: X-Tenant-ID ヘッダーなしでログインする
# Then: 400 Bad Request が返される

POST {{bff_url}}/auth/login
Content-Type: application/json
{
    "email": "{{admin_email}}",
    "password": "{{password}}"
}

HTTP 400
[Asserts]
jsonpath "$.type" == "https://ringiflow.example.com/errors/validation-error"
jsonpath "$.title" == "Validation Error"
jsonpath "$.status" == 400
