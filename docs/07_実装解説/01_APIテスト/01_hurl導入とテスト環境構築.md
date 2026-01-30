# hurl 導入と API テスト環境構築

## 概要

BFF の API テストを [hurl](https://hurl.dev/) で実装した。
Auth Service 分離により認証フローが複数サービスにまたがるようになり、実際のサービス間通信を含めたテストが必要になった。

### 対応 Issue

- [#98 E2E API テストを hurl で追加する](https://github.com/ka2kama/ringiflow/issues/98)

### 実装の経緯

当初「E2E テスト」として設計したが、実装過程で「API テスト」に名称を変更した。
理由は後述の「設計解説 1」を参照。

## 実装したコンポーネント

### テストファイル

| ファイル | 責務 |
|---------|------|
| [`tests/api/hurl/health.hurl`](../../../tests/api/hurl/health.hurl) | ヘルスチェック |
| [`tests/api/hurl/auth/login.hurl`](../../../tests/api/hurl/auth/login.hurl) | ログイン（正常系・異常系） |
| [`tests/api/hurl/auth/logout.hurl`](../../../tests/api/hurl/auth/logout.hurl) | ログアウト |
| [`tests/api/hurl/auth/me.hurl`](../../../tests/api/hurl/auth/me.hurl) | 認証済みユーザー情報取得 |
| [`tests/api/hurl/auth/me_unauthorized.hurl`](../../../tests/api/hurl/auth/me_unauthorized.hurl) | 未認証時の 401 確認 |
| [`tests/api/hurl/auth/csrf.hurl`](../../../tests/api/hurl/auth/csrf.hurl) | CSRF トークン取得 |
| [`tests/api/hurl/auth/csrf_unauthorized.hurl`](../../../tests/api/hurl/auth/csrf_unauthorized.hurl) | 未認証時の 401 確認 |

### 環境設定

| ファイル | 責務 |
|---------|------|
| [`tests/api/hurl/vars.env`](../../../tests/api/hurl/vars.env) | hurl 用共通変数 |
| [`backend/.env.api-test`](../../../backend/.env.api-test) | API テスト用環境変数 |
| [`infra/docker/docker-compose.api-test.yaml`](../../../infra/docker/docker-compose.api-test.yaml) | API テスト用 DB/Redis |

### justfile コマンド

| コマンド | 説明 |
|---------|------|
| `just test-api` | API テスト実行（ワンコマンド） |
| `just api-test-deps` | DB/Redis 起動 |
| `just api-test-reset-db` | DB リセット |
| `just api-test-stop` | コンテナ停止 |
| `just api-test-clean` | コンテナ + データ削除 |

## テスト実行

```bash
# ワンコマンドで実行（推奨）
just test-api

# 手動実行
just api-test-deps
just api-test-reset-db
# （別ターミナルでサービス起動）
hurl --test --variables-file tests/api/hurl/vars.env tests/api/hurl/**/*.hurl
```

## CI 統合

GitHub Actions で自動実行される（`.github/workflows/ci.yaml`）。

- トリガー: `backend/**` または `tests/api/**` の変更時
- `api-test` ジョブとして、他のテスト（rust, rust-integration, elm）と並列実行

## 関連ドキュメント

- [`tests/api/README.md`](../../../tests/api/README.md) - テスト実行手順とトラブルシューティング
- [ナレッジベース: hurl](../../06_ナレッジベース/devtools/hurl.md) - hurl の使い方

---

## 設計解説

### 1. なぜ「E2E」ではなく「API テスト」か

**場所**: ディレクトリ名 `tests/api/`

**経緯**:
当初 `tests/e2e/` として設計したが、実装途中で名称を変更した。

**理由**:
「E2E（End-to-End）テスト」は一般的に**ユーザー視点でのテスト**を指す。
ブラウザを操作して画面遷移やフォーム入力を検証するテストが典型例。

今回実装したのは：
- API レベルでのリクエスト/レスポンス検証
- サービス間通信の動作確認
- 認証フロー（セッション、Cookie、CSRF）の検証

これは「API Integration Test」または「API テスト」と呼ぶのが適切。

**テストピラミッドでの位置づけ**:

```
                    /\
                   /  \      API テスト（今回実装）
                  /    \     全サービス起動、HTTP レベル
                 /──────\
                /        \   rust-integration
               /          \  Repository/Session + BFF 部分統合
              /────────────\
             /              \ rust (unit)
            /                \ モック使用、高速
           /──────────────────\
```

**テスト種別の詳細**:

| テスト | 実行コマンド | 本物 | スタブ/モック | 検証対象 |
|-------|-------------|------|--------------|---------|
| rust (unit) | `cargo test --lib --bins` | なし | 全部 | 各関数/モジュールの動作 |
| rust-integration | `cargo test --test '*'` | DB, Redis | 外部サービス | Repository/Session 層 |
| api-test | `just test-api` | 全部 | なし | サービス間連携 |

rust-integration の対象テスト:
- `backend/crates/infra/tests/user_repository_test.rs` - PostgreSQL との結合
- `backend/crates/infra/tests/session_test.rs` - Redis との結合
- `backend/apps/bff/tests/auth_integration_test.rs` - BFF 認証フロー（外部サービスはスタブ）

**代替案**:
- `tests/integration/` - Rust の統合テストと紛らわしい
- `tests/e2e/` - 用語として不正確
- `tests/api/` - 採用。何をテストしているか明確

### 2. hurl の Cookie 自動管理と認証テストの罠

**場所**: [`tests/api/hurl/auth/me_unauthorized.hurl`](../../../tests/api/hurl/auth/me_unauthorized.hurl)

**問題**:
最初、認証済み/未認証テストを同一ファイルに書いた：

```hurl
# login.hurl（当初の設計）

# 正常系: ログイン成功
POST {{bff_url}}/auth/login
{ "email": "admin@example.com", "password": "password123" }
HTTP 200

# 異常系: 未認証で /auth/me にアクセス → 401 を期待
GET {{bff_url}}/auth/me
HTTP 401  # ← 実際は 200 が返る！
```

**原因**:
hurl は**同一ファイル内で Cookie を自動管理**する。
直前のログインで取得した `session_id` Cookie が自動送信され、認証済みとして扱われた。

**解決策**:
認証状態が異なるテストは**別ファイルに分離**する。

```
tests/api/hurl/auth/
├── me.hurl                 # 認証済み前提
└── me_unauthorized.hurl    # 未認証前提（Cookie なし）
```

**学び**:
hurl の便利機能（Cookie 自動管理）が、テストの意図と矛盾するケースがある。
ツールの動作を理解した上で、テスト設計を行う必要がある。

### 3. CSRF トークンの取得と送信

**場所**: [`tests/api/hurl/auth/logout.hurl`](../../../tests/api/hurl/auth/logout.hurl)

**問題**:
`POST /auth/logout` が 403 Forbidden を返した。

**原因**:
BFF の CSRF ミドルウェアが、状態変更リクエスト（POST/PUT/DELETE）に対して `X-CSRF-Token` ヘッダーを要求していた。

**解決策**:
logout 前に CSRF トークンを取得し、ヘッダーに含める。

```hurl
# Given: ログイン済み
POST {{bff_url}}/auth/login
# ...
[Captures]
session_cookie: cookie "session_id"

# CSRF トークンを取得
GET {{bff_url}}/auth/csrf
Cookie: session_id={{session_cookie}}
HTTP 200
[Captures]
csrf_token: jsonpath "$.data.token"

# When: ログアウト（CSRF トークン付き）
POST {{bff_url}}/auth/logout
X-CSRF-Token: {{csrf_token}}
Cookie: session_id={{session_cookie}}
HTTP 204
```

**学び**:
API テストでは、実際のクライアント（ブラウザ/SPA）と同じ手順を踏む必要がある。
CSRF 保護のような「暗黙の前提」を忘れずにテストに含める。

### 4. テスト環境の完全分離

**場所**: [`backend/.env.api-test`](../../../backend/.env.api-test)、[`infra/docker/docker-compose.api-test.yaml`](../../../infra/docker/docker-compose.api-test.yaml)

**問題**:
当初、開発用の DB/Redis をテストでも使う設計だった。
しかし以下の問題が予想された：

1. 開発中にデータを変更すると、テストが壊れる
2. テストがデータを変更すると、開発に影響する
3. `just reset-db` するたびに開発データが消える

**解決策**:
API テスト専用の環境を完全分離した。

| コンポーネント | 開発 | API テスト |
|---------------|------|-----------|
| PostgreSQL | 15432 | 15433 |
| Redis | 16379 | 16380 |
| BFF | 13000 | 14000 |
| Core Service | 13001 | 14001 |
| Auth Service | 13002 | 14002 |

**Docker Compose のプロジェクト分離**:

```bash
# 開発環境
docker compose -p ringiflow up -d
# → コンテナ: ringiflow-postgres-1, ringiflow-redis-1

# API テスト環境
docker compose -p ringiflow-api-test -f docker-compose.api-test.yaml up -d
# → コンテナ: ringiflow-api-test-postgres-1, ringiflow-api-test-redis-1
```

`-p` オプションでプロジェクト名を分けることで、コンテナ名・ボリューム名・ネットワーク名がすべて分離される。

**学び**:
テスト環境は「後から分離」より「最初から分離」の方が楽。
ポート番号やコンテナ名の競合を避ける設計を最初から考慮する。

### 5. Given-When-Then 形式によるテストの意図明示

**場所**: 全ての `.hurl` ファイル

**問題**:
hurl のテストは HTTP リクエスト/レスポンスの羅列になりがち。
何を目的に、何をテストしているのかが分かりにくい。

**解決策**:
各テストケースに Given-When-Then コメントを追加。

```hurl
# =============================================================================
# 正常系: 有効な認証情報でログインできる
# =============================================================================
# Given: シードデータの管理者ユーザー（admin@example.com）が存在する
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
```

**構成**:
- **タイトル行**: 何をテストしているか（正常系/異常系 + 概要）
- **Given**: 前提条件（データ、状態）
- **When**: 実行するアクション
- **Then**: 期待する結果

**学び**:
テストコードは「ドキュメント」でもある。
将来の自分や他の開発者が読んだとき、意図が伝わる書き方を心がける。

### 6. 将来の外部サービス呼び出しへの対応

**現状**:
内部サービス間通信（BFF → Core → Auth）のみ。
環境変数で URL を注入する設計になっている。

**将来の拡張**:
外部サービス（メール送信、決済など）を呼び出す場合は、モックサーバーを使う。

```yaml
# docker-compose.api-test.yaml（将来の拡張例）
services:
  mock-server:
    image: mockserver/mockserver
    ports:
      - "14100:1080"
```

```bash
# .env.api-test
SENDGRID_URL=http://localhost:14100/sendgrid
```

**設計のポイント**:
- 外部サービス URL をハードコードしない
- 環境変数で注入可能にしておく
- API テスト時はモックに差し替え

この設計は今回の実装で既に採用されており、将来の拡張にも対応できる。
