# YAML ファイル規約

YAML ファイルを作成・編集する際のルール。

## 拡張子

YAML ファイルの拡張子は `.yaml` を使用する。`.yml` は使用しない。

理由: [YAML 公式 FAQ](https://yaml.org/faq.html) で `.yaml` が推奨されている。

```yaml
# 良い例
config.yaml
docker-compose.yaml
ci.yaml

# 悪い例
config.yml
docker-compose.yml
ci.yml
```

## 適用対象

- GitHub Actions ワークフロー（`.github/workflows/`）
- Docker Compose 設定（`infra/docker/`）
- Dependabot 設定（`.github/dependabot.yaml`）
- lefthook 設定（`lefthook.yaml`）
- OpenAPI 仕様（`openapi/`）
- その他すべての YAML ファイル
