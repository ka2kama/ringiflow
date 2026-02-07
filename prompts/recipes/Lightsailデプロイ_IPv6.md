# Lightsail デプロイ（IPv6-only）

## いつ使うか

IPv6-only の Lightsail インスタンスにデプロイする際のハマりポイントと回避策。

## 手順

### deploy.sh の実行

```bash
cd infra/lightsail
./deploy.sh
```

### マイグレーション（IPv6-only 環境）

IPv6-only 環境では Docker コンテナからインターネットに出られないため、README の sqlx-cli 方式は使えない。代わりに psql で直接実行する:

```bash
# ローカルからマイグレーションファイルを転送
scp -r backend/migrations/ "ec2-user@[IPv6アドレス]:/home/ec2-user/ringiflow/migrations/"

# Lightsail 上で psql 経由で実行
ssh ec2-user@IPv6アドレス
cd ~/ringiflow
ls migrations/*.sql | sort | while read f; do echo "Running $f..."; docker exec -i ringiflow-postgres psql -U ringiflow -d ringiflow_prod < "$f"; done
```

### SCP で IPv6 アドレスを使う

zsh ではグロブ展開を防ぐためにクォートが必要:

```bash
scp file "ec2-user@[2406:da14:...]:/path"
```

### DNS キャッシュの問題

ネームサーバー変更後に NXDOMAIN がキャッシュされることがある:

```bash
# キャッシュをフラッシュ
sudo resolvectl flush-caches

# Cloudflare の NS に直接問い合わせて確認
dig demo.ka2kama.com @bjorn.ns.cloudflare.com
```

## なぜこの方法か

- IPv6-only は通常の Docker 運用と挙動が異なり、ドキュメントも少ない
- SCP の IPv6 対応は直感的でなく、シェルによっても挙動が違う
- DNS キャッシュは NXDOMAIN を長時間保持するため、手動フラッシュが必要なことがある
