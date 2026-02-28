# Issue #98: E2E API テストを hurl で追加する

## 概要

BFF の E2E API テストを hurl で追加する。Auth Service 分離により、認証フローが BFF → Core API → Auth Service と複数サービスにまたがるため、実際のサービスを起動した状態でのテストが必要。

## 前提条件の問題

**重要**: 現在 `auth.credentials` テーブルにシードデータがないため、E2E テストでログインができない。マイグレーションの追加が必須。

## 実装計画

### Phase 1: 前提条件の整備

#### 1.1 auth.credentials シードデータの追加

**ファイル**: `backend/migrations/20260125000001_seed_auth_credentials.sql`

```sql
-- 開発用ユーザーの認証情報を auth.credentials に追加
-- パスワード: password123

INSERT INTO auth.credentials (id, user_id, tenant_id, credential_type, credential_data, is_active)
VALUES
    ('00000000-0000-0000-0000-000000000011',
     '00000000-0000-0000-0000-000000000001',
     '00000000-0000-0000-0000-000000000001',
     'password',
     '$argon2id$v=19$m=65536,t=1,p=1$olntqw+EoVpwH4B1vUAI0A$5yCA1izLODgz8nQOInDGwbuQB/AS0sIQDwpmIilve5M',
     true),
    ('00000000-0000-0000-0000-000000000012',
     '00000000-0000-0000-0000-000000000002',
     '00000000-0000-0000-0000-000000000001',
     'password',
     '$argon2id$v=19$m=65536,t=1,p=1$olntqw+EoVpwH4B1vUAI0A$5yCA1izLODgz8nQOInDGwbuQB/AS0sIQDwpmIilve5M',
     true);
```

#### 1.2 hurl インストール確認

**更新ファイル**:
- `justfile` の `check-tools` に hurl 確認を追加
- `docs/04_手順書/01_開発参画/01_開発環境構築.md` に hurl を追加

### Phase 2: テスト基盤の構築

#### 2.1 ディレクトリ構成

```
tests/
└── e2e/
    ├── README.md
    ├── hurl/
    │   ├── vars.env           # 共通変数
    │   ├── health.hurl        # ヘルスチェック
    │   └── auth/
    │       ├── login.hurl     # ログインテスト
    │       ├── me.hurl        # /auth/me テスト
    │       ├── logout.hurl    # ログアウトテスト
    │       └── csrf.hurl      # CSRF トークンテスト
    └── scripts/
        └── wait-for-healthy.sh
```

#### 2.2 justfile への追加コマンド

```just
# E2E テスト（hurl）
test-e2e:
    hurl --test --variables-file tests/e2e/hurl/vars.env tests/e2e/hurl/**/*.hurl

# E2E テスト用サービス起動
e2e-start: dev-deps
    # BFF, Core Service, Auth Service をバックグラウンド起動

# E2E テスト用サービス停止
e2e-stop:
    # PID ファイルから停止
```

### Phase 3: テストケースの実装

#### 対象テストケース

| エンドポイント | テストケース | ファイル |
|---------------|-------------|---------|
| POST /auth/login | 正常ログイン | login.hurl |
| POST /auth/login | パスワード不一致（401） | login.hurl |
| POST /auth/login | ユーザー不存在（401） | login.hurl |
| POST /auth/login | テナント ID なし（400） | login.hurl |
| GET /auth/me | 認証済み | me.hurl |
| GET /auth/me | 未認証（401） | me.hurl |
| POST /auth/logout | ログアウト | logout.hurl |
| GET /auth/csrf | CSRF トークン取得 | csrf.hurl |

### Phase 4: ドキュメント整備

- `tests/e2e/README.md` - テスト実行方法
- Issue #98 のチェックボックス更新

## 修正対象ファイル

| ファイル | 操作 |
|---------|------|
| `backend/migrations/20260125000001_seed_auth_credentials.sql` | 新規作成 |
| `tests/e2e/hurl/vars.env` | 新規作成 |
| `tests/e2e/hurl/health.hurl` | 新規作成 |
| `tests/e2e/hurl/auth/login.hurl` | 新規作成 |
| `tests/e2e/hurl/auth/me.hurl` | 新規作成 |
| `tests/e2e/hurl/auth/logout.hurl` | 新規作成 |
| `tests/e2e/hurl/auth/csrf.hurl` | 新規作成 |
| `tests/e2e/scripts/wait-for-healthy.sh` | 新規作成 |
| `tests/e2e/README.md` | 新規作成 |
| `justfile` | E2E テストコマンド追加 |
| `docs/04_手順書/01_開発参画/01_開発環境構築.md` | hurl 追加 |

## 検証方法

```bash
# 1. 依存サービス起動
just dev-deps

# 2. マイグレーション実行
just reset-db

# 3. 3つのターミナルでサービス起動
just dev-bff           # ターミナル 1
just dev-core-service  # ターミナル 2
just dev-auth-service  # ターミナル 3

# 4. E2E テスト実行
just test-e2e
```

## CI 統合（オプション）

今回は CI への統合はスコープ外とし、ローカル実行を優先する。テストが安定してから CI に統合を検討。

## 備考

- hurl は宣言的な HTTP テストツールで、curl に似た構文でテストを記述できる
- 変数キャプチャ機能により、ログイン後のセッション ID を後続リクエストで使用可能
- Issue の実装方針に沿った構成を採用
