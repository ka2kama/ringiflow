# API テスト

BFF の API テストを [hurl](https://hurl.dev/) で実装している。

## 概要

Auth Service 分離により、認証フローが BFF → Core API → Auth Service と複数サービスにまたがる。
Unit テストやスタブを使った Integration テストでは検証できないサービス間通信を、実際のサービスを起動した状態でテストする。

## テストピラミッドでの位置づけ

API テストは統合テスト層の中で最も広いスコープを持つ。→ [テストピラミッドの概念整理](../../docs/80_ナレッジベース/methodology/テストピラミッド.md)

| テスト種別 | ピラミッド層 | 実行コマンド | 本物 | スタブ/モック |
|-----------|-----------|-------------|------|--------------|
| Unit | ユニット | `cargo test --lib --bins` | なし | 全部モック |
| Integration | 統合 | `cargo test --test '*'` | DB, Redis | 外部サービスはスタブ |
| API（本ディレクトリ） | 統合 | `just test-api` | 全部 | なし |

API テストは「サービス間連携が正しく動くか」を検証する。
Integration テストは「各層が DB/Redis と正しく連携するか」を検証する。

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

API テストは専用の DB/Redis/ポートを使用するため、開発環境と並行して実行可能。

| コンポーネント | 開発 | API テスト |
|---------------|------|-----------|
| PostgreSQL | localhost:15432 | localhost:15433 |
| Redis | localhost:16379 | localhost:16380 |
| BFF | localhost:13000 | localhost:14000 |
| Core Service | localhost:13001 | localhost:14001 |
| Auth Service | localhost:13002 | localhost:14002 |

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
curl http://localhost:14000/health
curl http://localhost:14001/health
curl http://localhost:14002/health
```

### `401 Unauthorized` でログインが失敗する

シードデータが投入されていない可能性がある。API テスト用 DB をリセット:

```bash
just api-test-reset-db
```

### 開発用 DB に API テストが接続してしまう

`.env.api-test` を使用せずにサービスを起動している可能性がある。
`just test-api` を使用するか、手動実行時は環境変数を指定する。

## CI

GitHub Actions で自動実行される。

- トリガー: `backend/**` または `tests/api/**` の変更時
- ワークフロー: `.github/workflows/ci.yaml` の `api-test` ジョブ

## 参考

- [Hurl 公式ドキュメント](https://hurl.dev/docs/manual.html)
- [ナレッジベース: hurl](../../docs/80_ナレッジベース/devtools/hurl.md)
