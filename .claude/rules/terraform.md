---
paths:
  - "**/*.tf"
  - "**/*.tfvars"
  - "**/*.tfvars.example"
---

# Terraform 実装ルール

このルールは Terraform ファイル（`*.tf`、`*.tfvars`）を編集する際に適用される。

## ディレクトリ構造

→ 基本設計書: `docs/30_基本設計書/02_プロジェクト構造設計.md` > Terraform 構造

```
infra/terraform/
├── environments/          # 環境別 root module
│   ├── dev/
│   ├── stg/
│   └── prod/
└── modules/               # 再利用可能なモジュール
    └── <module-name>/
        ├── main.tf
        ├── variables.tf
        └── outputs.tf
```

### ファイル構成規約

各ディレクトリ（root module、child module 共通）:

| ファイル | 内容 |
|---------|------|
| `versions.tf` | `terraform` ブロック（`required_version`、`required_providers`） |
| `backend.tf` | State backend 設定（root module のみ） |
| `providers.tf` | Provider 設定（root module のみ） |
| `main.tf` | リソース定義、モジュール呼び出し |
| `variables.tf` | 入力変数 |
| `outputs.tf` | 出力値 |
| `terraform.tfvars.example` | 変数テンプレート（root module のみ、`.tfvars` 自体は gitignore） |

## バージョン制約

| 対象 | 制約形式 | 理由 |
|------|---------|------|
| Terraform 本体 | `>= X.Y, < (X+1).0` | メジャーバージョンの breaking changes を防止 |
| Provider | `~> X.0` | マイナーバージョンの自動更新を許容、メジャー固定 |

## State 管理

- Backend: S3 + DynamoDB（ロック）
- バケット名: `ringiflow-terraform-state-{ACCOUNT_ID}`（グローバル一意性のため AWS アカウント ID を含める）
- State key: `{environment}/terraform.tfstate`（環境ごとに分離）
- `.terraform.lock.hcl` はバージョン管理対象（HashiCorp 推奨）

## フォーマットと検証

| コマンド | 用途 | タイミング |
|---------|------|-----------|
| `terraform fmt -check -recursive` | フォーマット検証 | コミット前 |
| `terraform validate` | 構文検証 | コミット前 |
| `tflint` | Lint（ベストプラクティス検証） | コミット前 |

ローカル: `just lint-terraform`（fmt チェック + validate + tflint）。CI: `.tf` ファイル変更時に自動実行。
→ 詳細: [ナレッジベース: Terraform](../../docs/80_ナレッジベース/devtools/Terraform.md)

## モジュール設計

| 原則 | 内容 |
|------|------|
| 単一責務 | 1 モジュール = 1 AWS サービスまたは 1 機能単位 |
| 入力は明示的に | 暗黙のデフォルト値を避け、`variables.tf` で型・説明を定義 |
| 出力は利用者視点 | 呼び出し元が必要とする値のみ output する |
| ハードコードしない | リージョン、アカウント ID、ドメイン名等は変数化 |

## セキュリティ

| 項目 | ルール |
|------|--------|
| シークレット | `.tfvars` に記述、Git にコミットしない（`.gitignore` で除外済み） |
| State | S3 暗号化（`encrypt = true`）、バケットのパブリックアクセスブロック |
| IAM | 最小権限の原則。`*` リソースの使用を避ける |
| タグ | `default_tags` で全リソースに `Project`, `Environment`, `ManagedBy` を付与 |

## リソース定義のベストプラクティス

### SES

- SESv2 API（`aws_sesv2_*`）を使用する。旧 SES v1（`aws_ses_*`）は使用しない
- Configuration Set には `suppression_options`（BOUNCE, COMPLAINT）を設定する
- ドメイン検証は `aws_sesv2_email_identity` で DKIM 署名を有効化する

### 今後のリソース追加時

新しい AWS サービスのリソースを追加する際:

1. Terraform Registry の公式ドキュメントで属性・引数を確認する
2. AWS のベストプラクティスガイドを参照する（暗号化、ログ、タグ等）
3. 既存モジュールのパターンに従う

## 実装前の確認（pre-implementation.md の補足）

`.tf` ファイルを変更する際、通常の確認事項に加えて以下を確認する:

| 確認項目 | 確認方法 |
|---------|---------|
| リソースの引数・属性 | Terraform Registry の公式ドキュメント |
| AWS サービスのベストプラクティス | AWS Well-Architected Framework、サービス別ドキュメント |
| 既存モジュールとの整合 | `infra/terraform/modules/` 配下の既存パターン |
| State への影響 | `terraform plan` の出力（destroy/recreate の有無） |

Terraform はローカルにコンパイラ相当（`terraform validate`）がない場合、公式ドキュメントの事前確認が通常以上に重要。

## 禁止事項

- Terraform Registry のドキュメントを確認せずにリソース定義を推測で書くこと
- `.tfvars` をバージョン管理にコミットすること
- `terraform apply` を確認なしに実行すること（`plan` を先に確認する）
- Provider バージョンを固定しないこと（`version` 制約の省略）

## 参照

- [プロジェクト構造設計 > Terraform 構造](../../docs/30_基本設計書/02_プロジェクト構造設計.md)
- [Terraform 基盤構築手順](../../docs/60_手順書/02_プロジェクト構築/02_Terraform基盤構築.md)
- [SES メール送信テスト手順](../../docs/60_手順書/03_運用/02_SESメール送信テスト.md)
