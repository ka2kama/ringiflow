# IPv6-only 環境での Docker 運用

## 概要

IPv6-only のサーバー（Lightsail 等）で Docker を運用する際のハマりポイントと対処法をまとめる。

## Docker ネットワークと IPv6

Docker のデフォルトブリッジネットワークは IPv4 のみで動作する。IPv6-only ホストでは以下の差異が生じる:

| 操作 | 動作 | 理由 |
|------|------|------|
| `docker pull` | 成功 | Docker デーモンがホストのネットワークスタックを直接使用 |
| コンテナ内から外部通信 | 失敗 | ブリッジネットワークが IPv4 のみ。IPv6 ルーティングがない |
| コンテナ間通信 | 成功 | Docker 内部ネットワーク（IPv4）で完結 |

### 影響を受ける操作の例

- コンテナ内での `cargo install`、`npm install`、`pip install`
- コンテナ内からの外部 API 呼び出し
- コンテナ内での DNS 解決（外部ドメイン）

### 影響を受けない操作

- Docker イメージの pull / push（デーモンが直接行う）
- コンテナ間通信（同一 Docker ネットワーク内）
- ホストからコンテナへの通信（ポートマッピング経由）

## SCP と IPv6

SCP はコロン（`:`）をホストとパスの区切り文字として使用するため、IPv6 アドレスのコロンと衝突する。

```bash
# NG: SCP が 2406 をホスト名、da14:... をパスと解釈
scp file user@2406:da14:1dba:...:/path

# OK: 角括弧で IPv6 アドレスを囲む
scp file "user@[2406:da14:1dba:...]:/path"
```

SSH は角括弧なしで動作する:

```bash
# OK: SSH は IPv6 アドレスをそのまま受け付ける
ssh user@2406:da14:1dba:...
```

zsh では角括弧がグロブとして解釈されるため、全体をクォートする必要がある。

## Cloudflare + IPv6-only

Cloudflare のリバースプロキシを使えば、IPv4 しか持たないクライアントからも IPv6-only サーバーにアクセスできる:

```
クライアント(IPv4) → Cloudflare(IPv4/IPv6) → サーバー(IPv6)
```

条件:
- DNS レコードを AAAA（IPv6）で登録
- Proxy status を Proxied（オレンジ雲）に設定

### SSL モードの選択

| モード | Cloudflare → オリジン | 用途 |
|--------|---------------------|------|
| Flexible | HTTP（ポート 80） | オリジンに SSL 設定不要 |
| Full | HTTPS（ポート 443） | 自己署名証明書でも可 |
| Full (Strict) | HTTPS（ポート 443） | 有効な証明書が必要 |

オリジン（Lightsail）で SSL を設定しない場合は **Flexible** を使用する。Full にすると Cloudflare がポート 443 に接続しようとし、522 エラーになる。

## プロジェクトでの使用箇所

- `infra/lightsail/deploy.sh` — `SSH_TARGET` と `SCP_TARGET` を分離して IPv6 対応
- `infra/lightsail/README.md` — Cloudflare SSL/TLS 設定手順

## 関連リソース

- [ADR-030: Lightsail 個人環境の構築](../../docs/05_ADR/030_Lightsail個人環境の構築.md)
- [Docker IPv6 Networking](https://docs.docker.com/config/daemon/ipv6/)
