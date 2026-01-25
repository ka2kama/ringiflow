# E2E テスト

BFF の E2E API テストを [hurl](https://hurl.dev/) で実装している。

## 概要

Auth Service 分離により、認証フローが BFF → Core API → Auth Service と複数サービスにまたがる。
Unit テストやスタブを使った Integration テストでは検証できないサービス間通信を、実際のサービスを起動した状態でテストする。

## 前提条件

以下がインストールされていること:

- hurl（`mise use -g hurl` または `cargo install hurl`）

インストール確認:

```bash
just check-tools
```

## テスト実行手順

### 1. 依存サービスを起動

```bash
just dev-deps
```

### 2. データベースをリセット（シードデータ含む）

```bash
just reset-db
```

### 3. サービスを起動（3 つのターミナルで実行）

```bash
# ターミナル 1
just dev-bff

# ターミナル 2
just dev-core-service

# ターミナル 3
just dev-auth-service
```

### 4. E2E テストを実行

```bash
just test-e2e
```

## ディレクトリ構成

```
tests/e2e/
├── README.md           # 本ファイル
├── hurl/
│   ├── vars.env        # 共通変数（URL、テナント ID、テストユーザー）
│   ├── health.hurl     # ヘルスチェック
│   └── auth/
│       ├── login.hurl  # ログインテスト
│       ├── me.hurl     # ユーザー情報取得テスト
│       ├── logout.hurl # ログアウトテスト
│       └── csrf.hurl   # CSRF トークン取得テスト
└── scripts/
    └── wait-for-healthy.sh  # サービス起動待機スクリプト
```

## テストケース

| エンドポイント | テストケース | ファイル |
|---------------|-------------|---------|
| GET /health | 正常レスポンス | health.hurl |
| POST /auth/login | 正常ログイン | auth/login.hurl |
| POST /auth/login | パスワード不一致（401） | auth/login.hurl |
| POST /auth/login | ユーザー不存在（401） | auth/login.hurl |
| POST /auth/login | テナント ID なし（400） | auth/login.hurl |
| GET /auth/me | 認証済み | auth/me.hurl |
| GET /auth/me | 未認証（401） | auth/me.hurl |
| POST /auth/logout | ログアウト | auth/logout.hurl |
| GET /auth/csrf | CSRF トークン取得 | auth/csrf.hurl |

## 変数（vars.env）

| 変数 | 値 | 説明 |
|------|-----|------|
| bff_url | http://localhost:13000 | BFF エンドポイント |
| tenant_id | 00000000-... | 開発用テナント ID |
| admin_email | admin@example.com | 管理者ユーザー |
| user_email | user@example.com | 一般ユーザー |
| password | password123 | テストパスワード |

## トラブルシューティング

### `Connection refused`

サービスが起動していない可能性がある。以下を確認:

```bash
# ヘルスチェック
curl http://localhost:13000/health
curl http://localhost:13001/health
curl http://localhost:13002/health
```

### `401 Unauthorized` でログインが失敗する

シードデータが投入されていない可能性がある。DB をリセット:

```bash
just reset-db
```

## 参考

- [Hurl 公式ドキュメント](https://hurl.dev/docs/manual.html)
- [Issue #98](https://github.com/ka2kama/ringiflow/issues/98)
