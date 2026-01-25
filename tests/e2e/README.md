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
├── hurl/
│   ├── vars.env    # 共通変数（URL、テナント ID、テストユーザー）
│   ├── health.hurl # ヘルスチェック
│   └── auth/       # 認証 API テスト
└── scripts/
    └── wait-for-healthy.sh
```

テストファイルは `tests/e2e/hurl/` 配下の `.hurl` ファイルを参照。

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

シードデータが投入されていない可能性がある。DB をリセット:

```bash
just reset-db
```

## 参考

- [Hurl 公式ドキュメント](https://hurl.dev/docs/manual.html)
