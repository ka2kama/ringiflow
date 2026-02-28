# Lightsail OS を Ubuntu 24.04 から AlmaLinux 9.4 へ変更

## Context

PR #91（`feature/lightsail-deploy`）の Lightsail デプロイ構成で、OS を Ubuntu 24.04 LTS から AlmaLinux 9.4 に変更する。

動機: RHEL 系サーバー運用の学習効果。エンタープライズ環境では RHEL が主流であり、SELinux・firewalld・dnf の実戦経験がキャリア上の差別化になる。RingiFlow のドメイン（エンタープライズ向けワークフロー管理）とも整合する。

## 変更対象ファイル

| # | ファイル | 変更内容 | 複雑度 |
|---|---------|---------|--------|
| 1 | `infra/lightsail/setup.sh` | 全面書き直し（dnf, firewalld, Docker CentOS repo） | 高 |
| 2 | `infra/lightsail/docker-compose.yml` | バインドマウントに SELinux `:Z` フラグ追加 | 低 |
| 3 | `infra/lightsail/deploy.sh` | バインドマウントに SELinux `:z` フラグ追加 | 低 |
| 4 | `infra/lightsail/.env.example` | `LIGHTSAIL_USER` を `ec2-user` に変更 | 低 |
| 5 | `infra/lightsail/backup.sh` | cron 例のパスを `ec2-user` に変更 | 低 |
| 6 | `infra/lightsail/README.md` | OS 名・ユーザー名の全置換、SELinux 注意事項追加 | 中 |
| 7 | `docs/70_ADR/030_Lightsail個人環境の構築.md` | OS 名変更、変更履歴追加 | 低 |

## 変更詳細

### 1. `infra/lightsail/setup.sh`（全面書き直し）

Ubuntu → AlmaLinux の主要な差異:

| 項目 | Ubuntu | AlmaLinux |
|------|--------|-----------|
| パッケージ管理 | `apt-get` | `dnf` |
| Docker リポジトリ | `download.docker.com/linux/ubuntu` | `download.docker.com/linux/centos` |
| Docker リポジトリ追加方法 | GPG キー + apt ソース追加 | `dnf config-manager --add-repo` |
| ファイアウォール | `ufw` | `firewalld` |
| デフォルトユーザー | `ubuntu` | `ec2-user`（要検証） |
| SELinux | なし（AppArmor） | **デフォルト有効** |

変更内容:
- コメントのユーザー名: `ubuntu` → `ec2-user`
- システムアップデート: `apt-get update && apt-get upgrade -y` → `dnf update -y`
- Docker インストール:
  - 前提パッケージ: `dnf install -y dnf-plugins-core`
  - リポジトリ追加: `dnf config-manager --add-repo https://download.docker.com/linux/centos/docker-ce.repo`
  - インストール: `dnf install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin`
  - サービス有効化: `systemctl enable --now docker`（Ubuntu では apt インストール時に自動起動）
- ファイアウォール: `ufw` → `firewalld`
  - `systemctl enable --now firewalld`
  - `firewall-cmd --permanent --add-service=ssh`
  - `firewall-cmd --permanent --add-port=80/tcp`
  - `firewall-cmd --reload`
- .env.example のダウンロード URL: `main` ブランチ参照（変更なし）

### 2. `infra/lightsail/docker-compose.yml`（SELinux 対応）

AlmaLinux は SELinux がデフォルト有効。バインドマウントには SELinux ラベルの再設定が必要。

**バインドマウント**（ホストのパスをマウント）:
- `./config/nginx/nginx.conf:/etc/nginx/nginx.conf:ro` → `:ro,Z`
- `./config/nginx/conf.d:/etc/nginx/conf.d:ro` → `:ro,Z`
- `./config/init:/docker-entrypoint-initdb.d:ro` → `:ro,Z`

**名前付きボリューム**（Docker が管理）→ 変更不要:
- `postgres_data`, `redis_data`, `frontend_dist` は Docker が SELinux コンテキストを自動設定

`:z` vs `:Z` の判断:
- `:z` = 共有ラベル（複数コンテナからアクセス可）
- `:Z` = プライベートラベル（単一コンテナ専用、より安全）
- 今回のバインドマウントはすべて単一コンテナ専用 → `:Z` を採用

### 3. `infra/lightsail/deploy.sh`（SELinux 対応）

159行目のフロントエンドファイルコピー用バインドマウント:
```bash
# 変更前
-v ~/ringiflow/frontend:/src:ro \
# 変更後
-v ~/ringiflow/frontend:/src:ro,z \
```

`:z`（小文字）を使用する理由: 一時的な busybox コンテナで、他のコンテナと共有される可能性がある `frontend` ディレクトリのため。

### 4. `infra/lightsail/.env.example`

```bash
# 変更前
LIGHTSAIL_USER=ubuntu
# 変更後
LIGHTSAIL_USER=ec2-user
```

### 5. `infra/lightsail/backup.sh`

12行目の cron 例コメント:
```bash
# 変更前
#   0 3 * * * /home/ubuntu/ringiflow/backup.sh >> /home/ubuntu/ringiflow/logs/backup.log 2>&1
# 変更後
#   0 3 * * * /home/ec2-user/ringiflow/backup.sh >> /home/ec2-user/ringiflow/logs/backup.log 2>&1
```

### 6. `infra/lightsail/README.md`

- ブループリント: `Ubuntu 24.04 LTS` → `AlmaLinux 9.4`
- SSH ユーザー: `ubuntu` → `ec2-user`（全箇所、約10箇所）
- SSH キーファイル名: `LightsailDefaultKey-ap-northeast-1.pem`（変更なし、Lightsail 共通）
- cron パス: `/home/ubuntu/` → `/home/ec2-user/`
- setup.sh の curl URL のブランチ名: `main` のまま（マージ後に使う前提）
- SELinux に関する注意事項を追加（トラブルシューティングセクション）

### 7. `docs/70_ADR/030_Lightsail個人環境の構築.md`

- 構成図の `Lightsail ($10/月)` 内に変更なし（OS 名は記載されていない）
- README への参照パスも変更なし
- 変更履歴に `AlmaLinux 9.4 へ移行` を追加

## 設計判断

### SELinux: 有効のまま運用 vs 無効化

| 選択肢 | 評価 |
|--------|------|
| **A. 有効のまま運用（採用）** | 学習効果が高い。`:Z` フラグで対応可能。エンタープライズでは有効が前提 |
| B. 無効化（`setenforce 0`） | 簡単だが学習機会を失う。本番で通用しない |

### デフォルトユーザー名

AlmaLinux on Lightsail のデフォルトユーザーは `ec2-user` と想定。初回 SSH 接続時に確認し、異なる場合は `.env` で調整可能（`LIGHTSAIL_USER` 変数経由）。

## スコープ外

- SELinux ポリシーのカスタマイズ（Docker のデフォルトポリシーで十分）
- firewalld の詳細なゾーン設定（デフォルトゾーンで十分）
- EPEL リポジトリの追加（現時点で必要なパッケージがない）

## 検証方法

```bash
# 1. just check-all が通るか（コード変更なしなので通るはず）
just check-all

# 2. 実際のデプロイで検証（手動）
# - Lightsail で AlmaLinux 9.4 インスタンスを作成
# - setup.sh を実行
# - deploy.sh を実行
# - SELinux が有効のまま全コンテナが起動するか確認
```

### ブラッシュアップループの記録

| ループ | きっかけ | 調査内容 | 結果 |
|-------|---------|---------|------|
| 1回目 | 初版完成 → SELinux の影響範囲を確認 | docker-compose.yml と deploy.sh の全バインドマウントを洗い出し | バインドマウント4箇所に `:Z`/`:z` フラグが必要。名前付きボリュームは変更不要 |
| 2回目 | `:z` vs `:Z` の使い分け確認 | 各マウントのコンテナ間共有状況を確認 | nginx 設定・init SQL は単一コンテナ → `:Z`、deploy.sh の一時マウントは `:z` |
| 3回目 | backup.sh の確認 | OS 固有のコマンドがないか確認 | `docker exec` ベースで OS 非依存。cron 例のパスのみ変更 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Ubuntu 固有の記述がすべて特定されている | OK | 7ファイルの全 Ubuntu 参照を洗い出し済み。nginx/conf.d、init SQL は OS 非依存で変更不要を確認 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | デフォルトユーザー名のみ要検証と明記（`.env` で調整可能） |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | SELinux 有効/無効、`:z`/`:Z` の選択、ファイアウォール方式を明記 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | SELinux ポリシーカスタマイズ、firewalld ゾーン設定、EPEL を対象外に |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | SELinux がバインドマウントに影響する点、Docker が名前付きボリュームの SELinux コンテキストを自動設定する点を確認 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | ADR-030 の「月額 $10」制約に影響なし（AlmaLinux も同じ $10 プラン） |
