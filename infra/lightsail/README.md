# RingiFlow Lightsail デプロイ手順書

AWS Lightsail 単一インスタンスに RingiFlow をデプロイする手順。
Cloudflare を前段に配置し、SSL 終端・CDN・DDoS 防御を提供する。

## 前提条件

- AWS アカウント
- Cloudflare アカウント
- 独自ドメイン（Cloudflare で DNS 管理）
- ローカル環境に Docker がインストール済み

## アーキテクチャ

```mermaid
graph TB
    subgraph Internet
        User["ユーザー"]
    end

    subgraph Cloudflare
        CF["Cloudflare<br/>SSL終端・CDN・DDoS防御"]
    end

    subgraph Lightsail["Lightsail ($10/月)"]
        subgraph Frontend["frontend network"]
            Nginx["Nginx<br/>:80"]
            BFF["BFF<br/>:13000"]
        end
        subgraph Backend["backend network (internal)"]
            CoreService["Core Service<br/>:13001"]
            AuthService["Auth Service<br/>:13002"]
            Postgres["PostgreSQL<br/>:5432"]
            Redis["Redis<br/>:6379"]
        end
    end

    User -->|HTTPS| CF
    CF -->|HTTP :80| Nginx
    Nginx -->|/api/*| BFF
    BFF --> CoreService
    BFF --> AuthService
    CoreService --> Postgres
    AuthService --> Postgres
    BFF --> Redis
```

## 手順

### 1. Lightsail インスタンスの作成

1. [AWS Lightsail コンソール](https://lightsail.aws.amazon.com/)にアクセス
2. 「インスタンスの作成」をクリック
3. 以下の設定でインスタンスを作成:

| 項目 | 設定値 |
|------|--------|
| リージョン | 東京 (ap-northeast-1) |
| プラットフォーム | Linux/Unix |
| ブループリント | OS のみ → Ubuntu 24.04 LTS |
| インスタンスプラン | $10 USD/月（2GB RAM, 60GB SSD） |
| インスタンス名 | ringiflow-prod |

4. SSH キーをダウンロードして保存:

```bash
mv ~/Downloads/LightsailDefaultKey-ap-northeast-1.pem ~/.ssh/lightsail-key.pem
chmod 600 ~/.ssh/lightsail-key.pem
```

5. 「ネットワーキング」タブで静的 IP を作成してアタッチ

### 2. ファイアウォールの設定

Lightsail コンソールの「ネットワーキング」タブで以下のルールを設定:

| アプリケーション | プロトコル | ポート |
|------------------|------------|--------|
| SSH | TCP | 22 |
| HTTP | TCP | 80 |

HTTPS (443) は不要（Cloudflare で終端するため）。

### 3. Lightsail インスタンスの初期セットアップ

```bash
# SSH 接続
ssh -i ~/.ssh/lightsail-key.pem ubuntu@<LIGHTSAIL_IP>

# セットアップスクリプトを実行
curl -fsSL https://raw.githubusercontent.com/ka2kama/ringiflow/main/infra/lightsail/setup.sh | bash

# 一度ログアウトして再ログイン（docker グループを有効化）
exit
ssh -i ~/.ssh/lightsail-key.pem ubuntu@<LIGHTSAIL_IP>

# Docker が使えることを確認
docker --version
```

### 4. 環境変数の設定

Lightsail 上で .env ファイルを作成:

```bash
cd ~/ringiflow
cp .env.example .env
vim .env
```

以下の値を設定:

```bash
# PostgreSQL（強力なパスワードを生成）
POSTGRES_USER=ringiflow
POSTGRES_PASSWORD=$(openssl rand -base64 24)
POSTGRES_DB=ringiflow_prod

# Redis（強力なパスワードを生成）
REDIS_PASSWORD=$(openssl rand -base64 24)

# ログレベル
RUST_LOG=info
```

パスワードは安全な場所に控えておくこと。

### 5. Cloudflare の設定

#### 5.1 DNS レコードの追加

1. [Cloudflare ダッシュボード](https://dash.cloudflare.com/)にアクセス
2. 対象ドメインを選択
3. DNS → レコードを追加:

| タイプ | 名前 | コンテンツ | プロキシ状態 |
|--------|------|------------|--------------|
| A | @ または app | Lightsail の静的 IP | プロキシ済み（オレンジ雲） |

#### 5.2 SSL/TLS の設定

1. SSL/TLS → 概要
2. 暗号化モード: **Full** を選択

Full モードの理由:
- オリジン（Lightsail）側で証明書管理が不要
- Cloudflare ↔ オリジン間も暗号化（自己署名証明書 OK）

#### 5.3 キャッシュルールの設定

1. キャッシュ → キャッシュルール
2. 以下のルールを作成:

API キャッシュバイパス:

| 項目 | 設定 |
|------|------|
| ルール名 | API Cache Bypass |
| 条件 | URI パスが `/api/` で始まる |
| アクション | キャッシュをバイパス |

#### 5.4 セキュリティ設定（推奨）

1. セキュリティ → 設定
   - セキュリティレベル: 中
   - チャレンジ有効期間: 1日

2. セキュリティ → ボット
   - ボットファイトモード: オン

### 6. ローカルからデプロイ

#### 6.1 デプロイ設定

ローカルの `infra/lightsail/.env` を作成:

```bash
cd infra/lightsail
cp .env.example .env
vim .env
```

デプロイ用の設定のみ記入:

```bash
LIGHTSAIL_HOST=<LIGHTSAIL_IP または ドメイン>
LIGHTSAIL_USER=ubuntu
LIGHTSAIL_SSH_KEY=~/.ssh/lightsail-key.pem
```

#### 6.2 デプロイ実行

```bash
./deploy.sh
```

処理内容:
1. Docker イメージをローカルでビルド
2. イメージを tar.gz にエクスポート
3. SCP で Lightsail に転送
4. Lightsail 上で docker load + compose up

#### 6.3 動作確認

```bash
# ヘルスチェック（HTTP）
curl http://<LIGHTSAIL_IP>/health

# Cloudflare 経由（HTTPS）
curl https://your-domain.com/health
curl https://your-domain.com/api/health
```

### 7. マイグレーションの実行

初回デプロイ時およびスキーマ変更があった場合は、マイグレーションを手動実行する。

#### 7.1 マイグレーションファイルの転送

ローカルから:

```bash
scp -i ~/.ssh/lightsail-key.pem -r backend/migrations/ ubuntu@<LIGHTSAIL_IP>:~/ringiflow/migrations/
```

#### 7.2 マイグレーション実行

Lightsail 上で Docker 経由で実行:

```bash
ssh -i ~/.ssh/lightsail-key.pem ubuntu@<LIGHTSAIL_IP>
cd ~/ringiflow

# sqlx-cli を含む Rust イメージでマイグレーション実行
docker run --rm \
    --network ringiflow-backend \
    -v "$(pwd)/migrations:/app/migrations" \
    -e DATABASE_URL="postgres://<POSTGRES_USER>:<POSTGRES_PASSWORD>@postgres:5432/<POSTGRES_DB>" \
    rust:1.84-slim-bookworm \
    bash -c "cargo install sqlx-cli --no-default-features --features postgres && sqlx migrate run --source /app/migrations"
```

注意: 初回は sqlx-cli のインストールに時間がかかる。頻繁に実行する場合は sqlx-cli を含む専用イメージの作成を推奨。

## 運用

### ログ確認

```bash
ssh -i ~/.ssh/lightsail-key.pem ubuntu@<LIGHTSAIL_IP>
cd ~/ringiflow

# 全サービスのログ
docker compose logs -f

# 特定サービスのログ
docker compose logs -f bff
docker compose logs -f core-service
docker compose logs -f auth-service
docker compose logs -f nginx
```

### バックアップ

手動実行:

```bash
ssh -i ~/.ssh/lightsail-key.pem ubuntu@<LIGHTSAIL_IP>
cd ~/ringiflow
./backup.sh
```

自動バックアップ（cron 設定）:

```bash
crontab -e
# 以下を追加（毎日 AM 3:00）
0 3 * * * /home/ubuntu/ringiflow/backup.sh >> /home/ubuntu/ringiflow/logs/backup.log 2>&1
```

### リストア

```bash
cd ~/ringiflow
./backup.sh --restore
```

### 再デプロイ

コード変更後、ローカルから:

```bash
./deploy.sh
```

ビルド済みイメージを使う場合:

```bash
./deploy.sh --skip-build
```

## トラブルシューティング

### コンテナが起動しない

```bash
# コンテナのステータス確認
docker compose ps

# 特定コンテナのログ確認
docker compose logs bff
docker compose logs core-service
docker compose logs auth-service

# 環境変数の確認
docker compose config
```

### ヘルスチェックが失敗する

```bash
# Nginx 設定の構文チェック
docker exec ringiflow-nginx nginx -t

# BFF へ直接アクセス（Nginx コンテナ内の wget を利用）
docker exec ringiflow-nginx wget -qO- http://bff:13000/health

# Core Service の疎通確認（BFF コンテナ内から）
docker exec ringiflow-bff bash -c 'echo > /dev/tcp/core-service/13001 && echo OK'

# Auth Service の疎通確認（BFF コンテナ内から）
docker exec ringiflow-bff bash -c 'echo > /dev/tcp/auth-service/13002 && echo OK'
```

### Cloudflare 経由でアクセスできない

1. DNS 設定を確認（プロキシ状態がオレンジ雲になっているか）
2. SSL/TLS モードが Full になっているか確認
3. Lightsail のファイアウォールで Port 80 が開いているか確認

### ディスク容量が不足

```bash
# 使用状況確認
df -h

# Docker の不要データを削除
docker system prune -a

# 古いバックアップを削除
find ~/ringiflow/backup -mtime +7 -delete
```

## コスト

| サービス | 月額 |
|----------|------|
| Lightsail 2GB | $10 |
| Cloudflare Free | $0 |
| 合計 | $10 |

## 関連ドキュメント

- [ADR-030: Lightsail 個人環境の構築](../../docs/05_ADR/030_Lightsail個人環境の構築.md)
- [docker-compose.yml](./docker-compose.yml)
