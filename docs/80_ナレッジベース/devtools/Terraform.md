# Terraform

## 概要

Terraform（テラフォーム）は HashiCorp が開発した IaC（Infrastructure as Code）ツール。
宣言的な HCL（HashiCorp Configuration Language）で AWS 等のクラウドリソースを定義・管理する。

## プロジェクトでの使用

RingiFlow では AWS インフラ（ECS Fargate, Aurora PostgreSQL, ElastiCache Redis 等）の管理に使用。

- ディレクトリ: `infra/terraform/`
- 環境: `environments/dev/`, `environments/stg/`, `environments/prod/`
- モジュール: `modules/` 配下に AWS サービス単位で分離
- ルール: `.claude/rules/terraform.md`

## ツールチェーン

| ツール | バージョン管理 | 用途 |
|--------|--------------|------|
| terraform | mise（`.mise.toml`） | IaC エンジン |
| tflint | mise（`.mise.toml`） | Lint（AWS ベストプラクティス検証） |

```bash
# インストール（mise 経由）
mise install

# バージョン確認
terraform version
tflint --version
```

## 品質検証

### コマンド一覧

| コマンド | 用途 | init 必要 |
|---------|------|----------|
| `terraform fmt -check -recursive` | フォーマット検証 | 不要 |
| `terraform validate` | 構文・型検証 | 必要（`terraform init`） |
| `tflint` | AWS ベストプラクティス検証 | 不要（`--init` でプラグインダウンロード） |

### ローカル実行

```bash
# フォーマット修正
just fmt-terraform

# リント（fmt チェック + validate + tflint）
just lint-terraform

# 全体チェックに含まれる
just check
```

### CI

`.tf` ファイルを変更した PR で自動実行される（`ci.yaml` の `terraform` ジョブ）。

CI では `terraform init -backend=false` を使用し、State バックエンド（S3）への接続なしにプロバイダーをダウンロードして `terraform validate` を実行する。

### lefthook（pre-commit）

`.tf` ファイルをステージングすると `terraform fmt -check` が自動実行される。

## terraform validate と terraform init の関係

`terraform validate` はプロバイダーのスキーマ情報を使って型チェックを行うため、`terraform init` でプロバイダーをダウンロード済みである必要がある。

| 環境 | init 方式 | 理由 |
|------|----------|------|
| CI | `terraform init -backend=false` | State バックエンド不要。プロバイダーのみダウンロード |
| ローカル | 手動（必要に応じて） | init は重い操作。`just lint-terraform` は init 済み環境のみ validate |

ローカルで validate を実行したい場合:

```bash
cd infra/terraform/environments/dev
terraform init -backend=false
terraform validate
```

## tflint 設定

設定ファイル: `infra/terraform/.tflint.hcl`

AWS プラグイン（`tflint-ruleset-aws`）を使用。AWS 固有のベストプラクティス違反を検出する:

- 存在しないインスタンスタイプの指定
- 非推奨のリソース属性の使用
- セキュリティグループの過度に広いルール

## 関連リソース

- [Terraform 公式ドキュメント](https://developer.hashicorp.com/terraform/docs)
- [tflint](https://github.com/terraform-linters/tflint)
- [tflint-ruleset-aws](https://github.com/terraform-linters/tflint-ruleset-aws)
- プロジェクト内:
  - `.claude/rules/terraform.md` — 実装ルール
  - `docs/60_手順書/02_プロジェクト構築/02_Terraform基盤構築.md` — 基盤構築手順
