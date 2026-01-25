# API テスト

BFF の API テストを [hurl](https://hurl.dev/) で実装している。

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

## テスト実行

### ワンコマンドで実行（推奨）

```bash
just test-api
```

このコマンドは以下を自動で行う:

1. API テスト専用の DB/Redis を起動（開発環境とは独立）
2. マイグレーション実行（シードデータ含む）
3. 3 つのサービス（BFF, Core, Auth）を起動
4. hurl テストを実行
5. テスト終了後、サービスを停止

### 手動で実行

```bash
# 1. API テスト用 DB/Redis を起動
just api-test-deps

# 2. DB をリセット
just api-test-reset-db

# 3. 別ターミナルでサービスを起動（.env.api-test を使用）
cd backend
env $(cat .env.api-test | grep -v '^#' | xargs) cargo run -p ringiflow-bff
env $(cat .env.api-test | grep -v '^#' | xargs) cargo run -p ringiflow-core-service
env $(cat .env.api-test | grep -v '^#' | xargs) cargo run -p ringiflow-auth-service

# 4. テスト実行
hurl --test --variables-file tests/api/hurl/vars.env tests/api/hurl/**/*.hurl

# 5. 後片付け
just api-test-stop   # コンテナ停止
just api-test-clean  # コンテナ + データ削除
```

## ディレクトリ構成

```
tests/api/
├── hurl/
│   ├── vars.env    # 共通変数（URL、テナント ID、テストユーザー）
│   ├── health.hurl # ヘルスチェック
│   └── auth/       # 認証 API テスト
└── scripts/
    └── wait-for-healthy.sh
```

テストファイルは `tests/api/hurl/` 配下の `.hurl` ファイルを参照。

## 開発環境との分離

API テストは専用の DB/Redis を使用するため、開発中のデータに影響しない。

| 環境 | PostgreSQL | Redis |
|------|-----------|-------|
| 開発 | localhost:15432 | localhost:16379 |
| API テスト | localhost:15433 | localhost:16380 |

設定ファイル:
- 開発: `backend/.env`
- API テスト: `backend/.env.api-test`

## hurl の注意点

- 同一ファイル内で Cookie は自動管理される
- 認証済み/未認証のテストは別ファイルに分離する

## トラブルシューティング

### `Connection refused`

サービスが起動していない可能性がある。以下を確認:

```bash
curl http://localhost:13000/health
curl http://localhost:13001/health
curl http://localhost:13002/health
```

### `401 Unauthorized` でログインが失敗する

シードデータが投入されていない可能性がある。API テスト用 DB をリセット:

```bash
just api-test-reset-db
```

### 開発用 DB に API テストが接続してしまう

`.env.api-test` を使用せずにサービスを起動している可能性がある。
`just test-api` を使用するか、手動実行時は環境変数を指定する。

## 参考

- [Hurl 公式ドキュメント](https://hurl.dev/docs/manual.html)
