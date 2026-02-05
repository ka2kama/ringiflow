# Docker ボリュームとファイル所有権

## 概要

Docker コンテナはデフォルトで root ユーザーとして実行される。そのため、ボリュームマウント時にコンテナがホスト側にファイルやディレクトリを作成すると、それらは root 所有になる。

これにより、ホスト側の一般ユーザーがそのファイルを編集・削除できなくなる問題が発生する。

## 問題の発生パターン

### パターン 1: 存在しないパスへのマウント

```yaml
# docker-compose.yaml
volumes:
  - ./data:/app/data  # ./data が存在しない場合
```

ホスト側に `./data` が存在しない状態でコンテナを起動すると、Docker が自動的にディレクトリを作成する。このとき所有者は root になる。

### パターン 2: コンテナ内でのファイル生成

```yaml
volumes:
  - ./logs:/app/logs  # コンテナ内でログファイルを生成
```

コンテナ内のプロセスが `/app/logs/app.log` を作成すると、ホスト側の `./logs/app.log` は root 所有になる。

## 対処法

### 削除できない場合

```bash
sudo rm -rf <ディレクトリ>
```

### 所有権を変更する場合

```bash
sudo chown -R $(id -u):$(id -g) <ディレクトリ>
```

## 予防策

### 方法 1: user を指定する

```yaml
services:
  app:
    image: myapp
    user: "${UID}:${GID}"
    volumes:
      - ./data:/app/data
```

起動時に環境変数を渡す:

```bash
UID=$(id -u) GID=$(id -g) docker compose up
```

注意: コンテナ内のプロセスが root 権限を必要とする場合は使用できない。

### 方法 2: 事前にディレクトリを作成

```bash
mkdir -p ./data
docker compose up
```

ホスト側で事前にディレクトリを作成しておけば、そのディレクトリの所有者は現在のユーザーのまま維持される。

### 方法 3: .env で UID/GID を設定

```bash
# .env
UID=1000
GID=1000
```

```yaml
# docker-compose.yaml
services:
  app:
    user: "${UID}:${GID}"
```

## プロジェクトでの対応

このプロジェクトでは、Docker Compose の設定でボリュームマウントを使用している。もし root 所有のファイルが作成された場合は、`sudo` で削除または所有権変更で対処する。

## 関連リソース

- [Docker 公式ドキュメント: Use volumes](https://docs.docker.com/storage/volumes/)
- [Docker Compose: user](https://docs.docker.com/compose/compose-file/05-services/#user)
