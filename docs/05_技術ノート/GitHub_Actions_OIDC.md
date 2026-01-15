# GitHub Actions OIDC 認証

## 概要

OIDC (OpenID Connect) は認証のための標準プロトコル。
GitHub Actions では、外部サービスへの安全な認証に使用される。

## 仕組み

```text
GitHub Actions ─── OIDC token ───▶ 外部サービス
                    │
                    └─ 「このワークフローは本当に指定リポジトリから実行されている」
                       という証明（短命トークン）
```

## API キーとの比較

| 方式 | 仕組み | リスク |
|------|--------|--------|
| API キー | Secrets に保存した固定キー | 漏洩時に無期限で悪用可能 |
| OIDC | 短命のトークンを都度発行 | トークンは数分で失効 |

## GitHub Actions での設定

OIDC を使用するには、ワークフローに `permissions` を設定する:

```yaml
permissions:
  id-token: write  # OIDC token の取得に必要
  contents: read   # リポジトリの読み取り
```

## RingiFlow での使用箇所

- Claude Code Action: Anthropic API への認証に OIDC を使用

## 参考

- [GitHub Docs: About security hardening with OpenID Connect](https://docs.github.com/en/actions/security-for-github-actions/security-hardening-your-deployments/about-security-hardening-with-openid-connect)
