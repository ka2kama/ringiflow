# hurl

[hurl](https://hurl.dev/) は Rust 製の HTTP テストツール。
HTTP リクエストとレスポンスの期待値を `.hurl` ファイルに記述し、API テストを実行する。

## hurl ファイルとは

`.hurl` ファイルは、HTTP リクエストとレスポンスの期待値をテキスト形式で記述したファイル。
curl コマンドに似た構文で、人間が読み書きしやすい。

```hurl
# これが .hurl ファイルの内容
# 1つのリクエスト + レスポンス検証 = 1つのテストケース

# ========== リクエスト部分 ==========
POST http://localhost:3000/auth/login
Content-Type: application/json
{
    "email": "user@example.com",
    "password": "password123"
}

# ========== レスポンス検証部分 ==========
HTTP 200
[Asserts]
jsonpath "$.data.user.email" == "user@example.com"
```

### 構成要素

```
┌─────────────────────────────────────┐
│ POST http://localhost:3000/api/xxx  │ ← HTTPメソッド + URL
│ Content-Type: application/json      │ ← ヘッダー（複数可）
│ X-Custom-Header: value              │
│ {                                   │ ← リクエストボディ（JSON等）
│     "key": "value"                  │
│ }                                   │
├─────────────────────────────────────┤
│ HTTP 200                            │ ← 期待するステータスコード
│ [Asserts]                           │ ← アサーション開始
│ jsonpath "$.data.id" exists         │ ← 検証ルール（複数可）
│ header "Set-Cookie" contains "xxx"  │
└─────────────────────────────────────┘
```

### 複数リクエストのチェーン

1つの `.hurl` ファイルに複数のリクエストを記述できる。
これにより「ログイン → 認証が必要な API を呼ぶ → ログアウト」のようなフローをテストできる。

```hurl
# 1. ログイン
POST http://localhost:3000/auth/login
{ "email": "user@example.com", "password": "pass" }
HTTP 200

# 2. 認証が必要な API（Cookie は自動送信される）
GET http://localhost:3000/auth/me
HTTP 200

# 3. ログアウト
POST http://localhost:3000/auth/logout
HTTP 204
```

## なぜ hurl を使うのか

| 比較対象 | 違い |
|---------|------|
| **curl** | curl は HTTP クライアント。レスポンスの検証はできない。hurl は検証まで含めたテストツール |
| **Postman** | Postman は GUI。hurl はテキストファイルなので Git 管理しやすく、CI で実行しやすい |
| **言語内テスト** | Rust/Go 等で書く統合テストは言語依存。hurl は言語非依存で、どのプロジェクトでも使える |

## インストール

```bash
# mise（推奨）
mise use -g hurl

# cargo
cargo install hurl

# Homebrew
brew install hurl
```

確認:

```bash
hurl --version
```

## 基本的な使い方

### テスト実行

```bash
# 単一ファイル
hurl --test login.hurl

# 複数ファイル
hurl --test auth/*.hurl

# 再帰的に全ファイル
hurl --test **/*.hurl
```

### 変数ファイル

ハードコードを避けるため、変数ファイルを使う。

```env
# vars.env
bff_url=http://localhost:14000
admin_email=admin@example.com
password=password123
```

```hurl
# login.hurl
POST {{bff_url}}/auth/login
{
    "email": "{{admin_email}}",
    "password": "{{password}}"
}
HTTP 200
```

```bash
hurl --test --variables-file vars.env login.hurl
```

### 値のキャプチャ

レスポンスから値を取り出して、後続のリクエストで使う。

```hurl
# CSRF トークンを取得
GET {{bff_url}}/auth/csrf
HTTP 200
[Captures]
csrf_token: jsonpath "$.data.token"

# キャプチャした値をヘッダーに使用
POST {{bff_url}}/auth/logout
X-CSRF-Token: {{csrf_token}}
HTTP 204
```

## アサーション

```hurl
HTTP 200
[Asserts]
# ステータスコード
status == 200

# ヘッダー
header "Content-Type" == "application/json"
header "Set-Cookie" contains "session_id="

# JSON パス
jsonpath "$.data.user.email" == "admin@example.com"
jsonpath "$.data.user.name" exists
jsonpath "$.errors" not exists

# Cookie
cookie "session_id" exists
cookie "session_id[HttpOnly]" exists
```

## アサーション一覧と使い分け

→ ルール: [`.claude/rules/api-test.md`](../../../.claude/rules/api-test.md)

### アサーション種別

| アサーション | 対象型 | 用途 | 例 |
|-------------|--------|------|-----|
| `==` | 全型 | 厳密一致。決定的な値に使う | `jsonpath "$.status" == "Draft"` |
| `matches` | 文字列 | 正規表現パターン検証。非決定的な値の形式確認に使う | `jsonpath "$.created_at" matches "^\\d{4}-\\d{2}-\\d{2}T"` |
| `contains` | 文字列 | 部分文字列の包含確認。ヘッダー値の検証に使う | `header "Set-Cookie" contains "HttpOnly"` |
| `startsWith` | 文字列 | プレフィックス一致 | `jsonpath "$.id" startsWith "WF-"` |
| `count` | 配列 | 要素数の検証。配列型の確認に使う | `jsonpath "$.roles" count >= 0` |
| `exists` | 全型 | フィールドの存在確認（null でも通る）。最終手段 | `jsonpath "$.data.id" exists` |
| `not exists` | 全型 | フィールドの非存在確認 | `jsonpath "$.errors" not exists` |
| `isString` | 全型 | 型チェック（`isInteger`, `isBoolean`, `isCollection` 等もあり） | `jsonpath "$.name" isString` |

注意:
- `matches` は**文字列型専用**。配列やオブジェクトには使えない
- `count` は配列型専用。文字列やオブジェクトには使えない
- `exists` は null 値でも通るため、「値がある」ことの証明にはならない

### よく使う正規表現パターン

| パターン | 用途 | 正規表現 |
|---------|------|---------|
| 表示用 ID | `WF-42` 形式 | `"^WF-\\d+$"` |
| RFC 3339 タイムスタンプ（先頭） | `2026-01-15T...` 形式 | `"^\\d{4}-\\d{2}-\\d{2}T"` |
| CSRF トークン | 64文字の16進数 | `"^[a-f0-9]{64}$"` |
| UUID | `550e8400-...` 形式 | `"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"` |

## 注意点: Cookie の自動管理

hurl は**同一ファイル内で Cookie を自動管理**する。
これは便利だが、テスト設計で注意が必要。

### 問題になるケース

```hurl
# login.hurl（問題のある書き方）

# ログイン（Set-Cookie: session_id=xxx が返る）
POST {{bff_url}}/auth/login
HTTP 200

# 未認証のテスト → 期待: 401
# 実際: 200（直前の Cookie が自動送信される）
GET {{bff_url}}/auth/me
HTTP 401  # ← 失敗する！
```

### 解決策: ファイルを分ける

認証状態が異なるテストは別ファイルにする。

```
tests/api/hurl/auth/
├── me.hurl                 # 認証済み前提
└── me_unauthorized.hurl    # 未認証前提（Cookie なし）
```

## テストの書き方（推奨）

Given-When-Then 形式でコメントを書くと、意図が明確になる。

```hurl
# =============================================================================
# 正常系: 有効な認証情報でログインできる
# =============================================================================
# Given: シードデータの管理者ユーザーが存在する
# When: 正しいメールアドレスとパスワードでログインする
# Then: ユーザー情報とセッション Cookie が返される

POST {{bff_url}}/auth/login
Content-Type: application/json
{
    "email": "{{admin_email}}",
    "password": "{{password}}"
}

HTTP 200
[Asserts]
jsonpath "$.data.user.email" == "admin@example.com"
header "Set-Cookie" contains "session_id="
header "Set-Cookie" contains "HttpOnly"
```

## プロジェクトでの構成例

```
tests/api/
├── hurl/
│   ├── vars.env              # 共通変数
│   ├── health.hurl           # ヘルスチェック
│   └── auth/                 # 認証関連
│       ├── login.hurl        # ログイン（正常系・異常系）
│       ├── logout.hurl       # ログアウト
│       ├── me.hurl           # 認証済みユーザー情報取得
│       ├── me_unauthorized.hurl
│       ├── csrf.hurl
│       └── csrf_unauthorized.hurl
└── README.md
```

## 参考

- [Hurl 公式サイト](https://hurl.dev/)
- [Hurl ドキュメント](https://hurl.dev/docs/manual.html)
- [Hurl サンプル集](https://hurl.dev/docs/samples.html)
- [Hurl GitHub](https://github.com/Orange-OpenSource/hurl)
