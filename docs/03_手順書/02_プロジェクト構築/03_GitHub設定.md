# GitHub 設定

## 目的

GitHub リポジトリの設定をセキュリティ・開発フローの両面から最適化する。
この手順書は一般的なベストプラクティスと、RingiFlow 固有の推奨設定を両方カバーする。

## 前提条件

- GitHub リポジトリへの Admin 権限があること
- CI ワークフローが動作すること

---

## 1. リポジトリ基本設定（Settings > General）

### 1.1 Repository name / Description

```
Settings > General
```

| 項目 | 推奨設定 | 理由 |
|------|---------|------|
| Description | プロジェクトの簡潔な説明 | 検索性向上、README を開かなくても概要がわかる |
| Website | デプロイ先 URL（あれば） | 実際の動作を確認できる |
| Topics | `rust`, `elm`, `workflow`, `saas` 等 | 検索性向上、GitHub Explore での発見 |

### 1.2 Features

```
Settings > General > Features
```

| 機能 | 推奨 | 理由 |
|------|------|------|
| Wikis | ❌ 無効 | ドキュメントは `docs/` で管理。分散を防ぐ |
| Issues | ✅ 有効 | バグ報告・機能要望の受付 |
| Sponsorships | 任意 | OSS の場合は有効化を検討 |
| Preserve this repository | 任意 | Arctic Code Vault への保存 |
| Discussions | 任意 | コミュニティがある場合は有効 |
| Projects | ✅ 有効 | タスク管理に使用する場合 |

### 1.3 Pull Requests

```
Settings > General > Pull Requests
```

| 項目 | 推奨設定 | 理由 |
|------|---------|------|
| Allow merge commits | ✅ 有効 | マージコミットで履歴を明確に |
| Allow squash merging | ✅ 有効 | 細かいコミットをまとめたい場合に便利 |
| Allow rebase merging | ❌ 無効 | 履歴の書き換えは避ける |
| Always suggest updating pull request branches | ✅ 有効 | 最新の main との統合を促す |
| Allow auto-merge | ✅ 有効 | CI 通過後の自動マージ |
| Automatically delete head branches | ✅ 有効 | マージ後のブランチ自動削除 |

**RingiFlow 固有設定:**

このプロジェクトでは Squash merge をデフォルトとする。

```
Default commit message: Pull request title
```

理由:
- 1 PR = 1 コミットで履歴がクリーン
- PR タイトルがそのままコミットメッセージになり、追跡しやすい
- 個々のコミットメッセージは PR 内で確認できる

---

## 2. ブランチ保護（Settings > Branches）

```
Settings > Branches > Add branch ruleset
```

GitHub は従来の Branch protection rules に加え、より柔軟な Rulesets を提供している。
新規プロジェクトでは Rulesets の使用を推奨する。

### 2.1 Rulesets vs Branch Protection Rules

| 観点 | Branch Protection Rules | Rulesets |
|------|------------------------|----------|
| 対象 | 単一ブランチパターン | 複数ブランチ・タグを統合管理 |
| バイパス | 個別設定 | 統一的なバイパス設定 |
| 優先度 | なし（競合時は厳しい方が適用） | 明示的な優先度設定 |
| インポート/エクスポート | 不可 | JSON でエクスポート可能 |
| 推奨度 | レガシー | 推奨 |

### 2.2 main ブランチ保護（Ruleset）

```
Settings > Rules > Rulesets > New ruleset > New branch ruleset
```

**Ruleset 基本設定:**

| 項目 | 設定値 |
|------|--------|
| Ruleset name | `main-protection` |
| Enforcement status | Active |
| Bypass list | （空、または緊急時用に Admin のみ） |

**Target branches:**

```
Add target > Include by pattern > main
```

**Rules:**

| ルール | 設定 | 理由 |
|--------|------|------|
| Restrict deletions | ✅ | main ブランチの削除を防止 |
| Require linear history | ❌ | マージコミットを許可（Squash merge は線形になる） |
| Require deployments to succeed | 任意 | CD パイプラインがある場合 |
| Require signed commits | ❌ | 署名環境の整備が必要、段階的に導入 |
| Require a pull request before merging | ✅ | 直接 push を禁止 |
| ├─ Required approvals | 0〜1 | 個人開発なら 0、チームなら 1 以上 |
| ├─ Dismiss stale pull request approvals | ✅ | 変更後は再承認を要求 |
| ├─ Require review from Code Owners | 任意 | CODEOWNERS 設定時 |
| ├─ Require approval of the most recent push | ✅ | 承認後の追加変更も再承認 |
| └─ Require conversation resolution | ✅ | レビューコメントの解決を強制 |
| Require status checks to pass | ✅ | CI 必須 |
| ├─ Require branches to be up to date | ✅ | 最新の main でテスト済みを保証 |
| └─ Status checks | `CI Success` | ci.yml の ci-success ジョブ |
| Block force pushes | ✅ | 履歴の改変を防止 |

**RingiFlow 固有設定:**

個人開発フェーズのため、Required approvals は 0 に設定。
チーム開発に移行する際は 1 以上に変更する。

### 2.3 develop ブランチ保護（任意）

main ほど厳格でなくてよいが、最低限の保護を推奨。

| ルール | 設定 |
|--------|------|
| Require status checks to pass | ✅ |
| Block force pushes | ✅ |
| Required approvals | 0 |

---

## 3. セキュリティ設定（Settings > Security）

### 3.1 Code security and analysis

```
Settings > Security > Code security and analysis
```

| 機能 | 推奨 | 説明 |
|------|------|------|
| Private vulnerability reporting | ✅ 有効 | セキュリティ問題の非公開報告 |
| Dependency graph | ✅ 有効 | 依存関係の可視化 |
| Dependabot alerts | ✅ 有効 | 脆弱性の通知 |
| Dependabot security updates | ✅ 有効 | セキュリティ修正 PR の自動作成 |
| Grouped security updates | ✅ 有効 | セキュリティ更新を 1 PR にまとめる |
| Dependabot version updates | ✅ 有効 | 通常の依存更新（dependabot.yml で設定済み） |
| Dependabot on Actions runners | 任意 | self-hosted runner 使用時 |
| Code scanning | ✅ 有効 | コードの脆弱性スキャン |
| Secret scanning | ✅ 有効 | シークレットの漏洩検知 |
| Push protection | ✅ 有効 | シークレット含むプッシュをブロック |

### 3.2 Code Scanning（CodeQL）

```
Settings > Security > Code security and analysis > Code scanning > Set up > Default
```

CodeQL は GitHub 提供の静的解析ツール。
セットアップすると `.github/workflows/codeql.yml` が作成される。

**対応言語:**
- JavaScript/TypeScript
- Python
- Ruby
- C/C++
- Go
- Java/Kotlin
- C#
- Swift

**注意:** Rust と Elm は CodeQL 非対応。
- Rust: `cargo clippy`, `cargo audit` で代替
- Elm: 型システムが強力なため、別途ツール不要

### 3.3 Secret Scanning

自動で検知されるシークレットの例:
- AWS Access Key
- GitHub Token
- Google API Key
- Slack Token
- SSH Private Key

**Push Protection が有効な場合:**
シークレットを含むコミットはプッシュ時にブロックされる。

```
$ git push origin feature/xxx
remote: error: GH013: Repository rule violations found for refs/heads/feature/xxx.
remote: - GITHUB PUSH PROTECTION
remote:   —————————————————————————————————————————
remote:   Resolve the following violations before pushing again
remote:
remote:   - Push cannot contain secrets
```

### 3.4 SECURITY.md

セキュリティポリシーを明文化するファイル。

```
Settings > Security > Security policy > Start setup
```

または手動で `.github/SECURITY.md` を作成:

```markdown
# セキュリティポリシー

## サポートバージョン

| バージョン | サポート状況 |
|-----------|-------------|
| 最新版     | ✅          |
| それ以前   | ❌          |

## 脆弱性の報告

セキュリティ脆弱性を発見した場合は、以下の方法で報告してください:

1. **GitHub Security Advisories を使用**（推奨）
   - リポジトリの「Security」タブから「Report a vulnerability」を選択

2. **非公開で連絡**
   - security@example.com 宛にメール

**公開 Issue での報告は避けてください。**

## 対応プロセス

1. 報告受領後 48 時間以内に確認の連絡
2. 調査・修正の実施
3. 修正リリース後に公開
```

---

## 4. Actions 設定（Settings > Actions）

### 4.1 General

```
Settings > Actions > General
```

**Actions permissions:**

| 設定 | 推奨 | 理由 |
|------|------|------|
| Allow all actions and reusable workflows | ❌ | 制限なしはリスク |
| Allow select actions and reusable workflows | ✅ | ホワイトリスト方式 |
| Disable actions | ❌ | CI/CD が使えなくなる |

**許可するアクション（推奨）:**

```
Allow actions created by GitHub ✅
Allow actions by Marketplace verified creators ✅
Allow specified actions and reusable workflows:
  actions/*
  dtolnay/rust-toolchain@*
  extractions/setup-just@*
  dorny/paths-filter@*
  pnpm/action-setup@*
```

**Workflow permissions:**

| 設定 | 推奨 | 理由 |
|------|------|------|
| Read repository contents and packages permissions | ✅ | 最小権限の原則 |
| Read and write permissions | ❌ | 必要な場合のみ個別に設定 |
| Allow GitHub Actions to create and approve PRs | ❌ | 自動化 PR が必要な場合のみ |

### 4.2 Runners

```
Settings > Actions > Runners
```

通常は GitHub-hosted runners で十分。
以下の場合に self-hosted runners を検討:

- ビルド時間の短縮が必要
- 特殊なハードウェア要件
- プライベートネットワークへのアクセスが必要

### 4.3 Secrets and variables

```
Settings > Secrets and variables > Actions
```

**Secrets（暗号化される）:**
- `AWS_ACCESS_KEY_ID`
- `AWS_SECRET_ACCESS_KEY`
- `DOCKER_PASSWORD`

**Variables（暗号化されない）:**
- `AWS_REGION`
- `ECR_REPOSITORY`

**命名規約:**
- 大文字スネークケース: `AWS_ACCESS_KEY_ID`
- プレフィックスで分類: `AWS_*`, `DOCKER_*`, `SLACK_*`

---

## 5. Environments（Settings > Environments）

### 5.1 環境の作成

```
Settings > Environments > New environment
```

典型的な環境構成:

| 環境名 | 用途 | 保護レベル |
|--------|------|-----------|
| development | 開発検証 | なし |
| staging | ステージング | 承認必須 |
| production | 本番 | 承認必須 + ブランチ制限 |

### 5.2 Environment protection rules

**staging / production 環境の推奨設定:**

| 設定 | 値 |
|------|-----|
| Required reviewers | 1 人以上 |
| Wait timer | 0〜30 分（本番は猶予を設ける） |
| Deployment branches | `main` のみ |

**ワークフローでの使用例:**

```yaml
jobs:
  deploy-staging:
    runs-on: ubuntu-latest
    environment: staging
    steps:
      - name: Deploy
        run: echo "Deploying to staging..."

  deploy-production:
    runs-on: ubuntu-latest
    needs: deploy-staging
    environment: production
    steps:
      - name: Deploy
        run: echo "Deploying to production..."
```

---

## 6. CODEOWNERS

### 6.1 概要

`CODEOWNERS` ファイルは、コードの各部分に責任者を割り当てる。
PR が作成されると、変更ファイルに応じて自動でレビュワーが割り当てられる。

### 6.2 ファイルの配置

以下のいずれかに配置（優先度順）:
1. `.github/CODEOWNERS`
2. `CODEOWNERS`（ルート）
3. `docs/CODEOWNERS`

### 6.3 書式

```
# デフォルトオーナー（全ファイル）
* @owner

# ディレクトリ単位
/apps/api/ @backend-team
/apps/web/ @frontend-team
/infra/ @infra-team
/docs/ @docs-team

# ファイルパターン
*.rs @rust-experts
*.elm @elm-experts

# 特定ファイル
/Cargo.toml @owner
/.github/workflows/ @devops-team
```

### 6.4 RingiFlow での設定例

```
# .github/CODEOWNERS

# デフォルト: オーナー
* @ka2kama

# バックエンド（Rust）
/apps/api/ @ka2kama
/packages/ @ka2kama
*.rs @ka2kama

# フロントエンド（Elm）
/apps/web/ @ka2kama
*.elm @ka2kama

# インフラ
/infra/ @ka2kama

# CI/CD
/.github/ @ka2kama
```

---

## 7. Issue / PR テンプレート

### 7.1 Issue テンプレート

```
.github/ISSUE_TEMPLATE/
├── bug_report.yml
├── feature_request.yml
└── config.yml
```

**バグ報告テンプレート（YAML 形式）:**

```yaml
# .github/ISSUE_TEMPLATE/bug_report.yml
name: バグ報告
description: バグや不具合を報告する
labels: ["bug"]
body:
  - type: markdown
    attributes:
      value: |
        バグ報告ありがとうございます。
        以下の情報を可能な限り記入してください。

  - type: textarea
    id: description
    attributes:
      label: 現象
      description: 何が起きたか
      placeholder: ログイン画面でエラーが表示される
    validations:
      required: true

  - type: textarea
    id: expected
    attributes:
      label: 期待する動作
      description: 本来どうなるべきか
      placeholder: 正常にログインできる

  - type: textarea
    id: steps
    attributes:
      label: 再現手順
      description: バグを再現する手順
      placeholder: |
        1. ログイン画面を開く
        2. メールアドレスを入力
        3. パスワードを入力
        4. ログインボタンをクリック
    validations:
      required: true

  - type: input
    id: environment
    attributes:
      label: 環境
      description: OS、ブラウザ、バージョン等
      placeholder: macOS 14.0, Chrome 120
```

**機能要望テンプレート:**

```yaml
# .github/ISSUE_TEMPLATE/feature_request.yml
name: 機能要望
description: 新機能や改善の提案
labels: ["enhancement"]
body:
  - type: textarea
    id: problem
    attributes:
      label: 解決したい課題
      description: どのような問題を解決したいか
    validations:
      required: true

  - type: textarea
    id: solution
    attributes:
      label: 提案する解決策
      description: どのように解決できると思うか

  - type: textarea
    id: alternatives
    attributes:
      label: 代替案
      description: 他に検討した方法があれば
```

**テンプレート選択画面の設定:**

```yaml
# .github/ISSUE_TEMPLATE/config.yml
blank_issues_enabled: false
contact_links:
  - name: 質問・相談
    url: https://github.com/owner/repo/discussions
    about: 質問や相談は Discussions へ
```

### 7.2 PR テンプレート

```markdown
<!-- .github/pull_request_template.md -->

## 概要

<!-- この PR で何を変更したか -->

## 関連 Issue

<!-- closes #123 のように記載すると自動でリンク＆クローズ -->

## 変更内容

<!-- 主な変更点を箇条書きで -->

-

## テスト

<!-- 動作確認した内容 -->

- [ ] ローカルでテスト実行
- [ ] 動作確認済み

## レビュー観点

<!-- レビュワーに特に見てほしい点 -->

```

---

## 8. その他の設定

### 8.1 Webhooks

```
Settings > Webhooks
```

外部サービスとの連携に使用:
- Slack 通知
- CI/CD（Jenkins, CircleCI 等）
- デプロイツール

**設定例（Slack 通知）:**

| 項目 | 値 |
|------|-----|
| Payload URL | Slack Incoming Webhook URL |
| Content type | application/json |
| Events | Let me select individual events |
| ├─ Pull requests | ✅ |
| ├─ Push | ✅ |
| └─ Workflow runs | ✅ |

### 8.2 GitHub Apps

```
Settings > GitHub Apps
```

よく使われる App:
- Dependabot: 依存関係更新（組み込み）
- Codecov: コードカバレッジ
- SonarCloud: 静的解析
- Renovate: 依存関係更新（Dependabot の代替）

### 8.3 Deploy keys

```
Settings > Deploy keys
```

CI/CD や外部サービスからリポジトリにアクセスするための SSH キー。
リポジトリ単位で設定でき、Personal Access Token より安全。

| 設定 | 用途 |
|------|------|
| Read-only | クローン・フェッチのみ |
| Read/write | プッシュも可能 |

### 8.4 Autolink references

```
Settings > Autolink references
```

Issue 番号や外部システムの ID を自動リンクする。

**例: Jira 連携**

| 項目 | 値 |
|------|-----|
| Reference prefix | PROJ- |
| Target URL | https://company.atlassian.net/browse/PROJ-<num> |
| Alphanumeric | ❌ |

これにより、コミットメッセージやコメント内の `PROJ-123` が自動でリンクになる。

---

## 9. 設定チェックリスト

### 9.1 必須設定

| カテゴリ | 項目 | 完了 |
|---------|------|------|
| General | Automatically delete head branches | ☐ |
| Branches | main ブランチ保護 | ☐ |
| Branches | Require status checks (CI Success) | ☐ |
| Security | Dependabot alerts | ☐ |
| Security | Secret scanning | ☐ |
| Security | Push protection | ☐ |
| Actions | Workflow permissions: Read only | ☐ |

### 9.2 推奨設定

| カテゴリ | 項目 | 完了 |
|---------|------|------|
| General | Description / Topics 設定 | ☐ |
| General | Allow squash merging (default) | ☐ |
| General | Disable rebase merging | ☐ |
| Security | Code scanning (CodeQL) | ☐ |
| Security | SECURITY.md | ☐ |
| Files | CODEOWNERS | ☐ |
| Files | Issue テンプレート | ☐ |
| Files | PR テンプレート | ☐ |

### 9.3 チーム開発時の追加設定

| カテゴリ | 項目 | 完了 |
|---------|------|------|
| Branches | Required approvals: 1 以上 | ☐ |
| Branches | Require Code Owner review | ☐ |
| Environments | staging / production 環境 | ☐ |
| Environments | Required reviewers | ☐ |

---

## トラブルシューティング

### PR がマージできない

**原因 1: Status checks が通っていない**

```
Settings > Branches > main > Require status checks
```

指定した `CI Success` ジョブが成功しているか確認。

**原因 2: ブランチが最新でない**

```
$ git fetch origin
$ git rebase origin/main
$ git push -f
```

または GitHub UI の「Update branch」ボタンを使用。

**原因 3: レビュー承認が足りない**

Required approvals の数だけ Approve が必要。

### Secret scanning で誤検知

**正当なテストデータの場合:**

```
Security > Secret scanning > Alerts > アラートを選択 > Close as > Not a secret
```

**パターンを除外する場合:**

`.github/secret_scanning.yml` でカスタムパターンを除外:

```yaml
paths-ignore:
  - "tests/fixtures/**"
  - "docs/examples/**"
```

### Dependabot PR が多すぎる

`dependabot.yml` で制限:

```yaml
open-pull-requests-limit: 3  # 5 → 3 に減らす
```

または、グループ化:

```yaml
groups:
  minor-and-patch:
    patterns:
      - "*"
    update-types:
      - "minor"
      - "patch"
```

---

## 参考リンク

- [GitHub Docs: Managing a branch protection rule](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/managing-a-branch-protection-rule)
- [GitHub Docs: About rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/about-rulesets)
- [GitHub Docs: Code security](https://docs.github.com/en/code-security)
- [GitHub Docs: CODEOWNERS](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners)

---

## 10. Claude Code Action 設定

Claude Code Action は PR オープン時に自動レビューを実行する GitHub Action。
CLAUDE.md に記載されたプロジェクト理念と品質基準に基づいてレビューが行われる。

→ 設計判断: [ADR-011](../../04_ADR/011_Claude_Code_Action導入.md)

### 10.1 前提条件

- リポジトリへの Admin 権限があること

### 10.2 GitHub App のインストール

**方法 A: Claude Code CLI から（推奨）**

```bash
claude
# Claude Code 起動後
/install-github-app
```

**方法 B: 直接インストール**

1. https://github.com/apps/claude にアクセス
2. 「Install」をクリック
3. 対象リポジトリを選択（`ka2kama/ringiflow`）
4. 「Install」で完了

### 10.3 認証の設定

```
Settings > Secrets and variables > Actions > New repository secret
```

**方法 A: OAuth トークン（サブスクリプション利用）**

| 項目 | 値 |
|------|-----|
| Name | `CLAUDE_CODE_OAUTH_TOKEN` |
| Secret | Claude Code CLI で `/install-github-app` 実行時に取得したトークン |

Claude Pro/Max サブスクリプションの利用枠を消費する。

**方法 B: API キー（API 課金）**

| 項目 | 値 |
|------|-----|
| Name | `ANTHROPIC_API_KEY` |
| Secret | https://console.anthropic.com/ から取得した API キー |

API 利用料が発生する。利用状況は https://console.anthropic.com/usage で確認。

### 10.4 Ruleset への Status Check 追加

```
Settings > Rules > Rulesets > main-protection（編集）
```

「Require status checks to pass」セクションで以下を追加:

| Status Check | 説明 |
|--------------|------|
| `CI Success` | CI ワークフローのジョブ |
| `Auto Review` | Claude Code Review ワークフローのジョブ |

### 10.5 動作確認

1. 新しいブランチを作成し、何らかの変更をコミット
2. PR を作成
3. GitHub Actions の「Claude Code Review」ワークフローが実行されることを確認
4. PR にレビューコメントが投稿されることを確認

### 10.6 使い方

| 機能 | 説明 |
|------|------|
| 自動レビュー | PR オープン/更新時に自動実行 |
| 対話的レビュー | PR コメントで `@claude` とメンションして質問 |

**対話的レビューの例:**

```
@claude このコードのセキュリティ上の問題点を教えてください
```

```
@claude テナント削除時のデータ削除は考慮されていますか？
```

### 10.7 利用制限

| 認証方式 | 課金 |
|---------|------|
| OAuth トークン | Claude Pro/Max サブスクリプションの利用枠を消費 |
| API キー | Anthropic API 利用料が発生 |

**制限のポイント:**

- ワークフローで `--max-turns` を設定済み（自動レビュー: 5、対話: 10）
- 不要なワークフロー実行を避ける（ドラフト PR ではスキップ等、必要に応じて設定追加）

### 10.8 トラブルシューティング

**ワークフローが実行されない**

1. GitHub App がインストールされているか確認
   - `Settings > GitHub Apps > Installed GitHub Apps`
2. 認証情報が設定されているか確認
   - `Settings > Secrets and variables > Actions`
   - `CLAUDE_CODE_OAUTH_TOKEN` または `ANTHROPIC_API_KEY`

**レビューコメントが投稿されない**

1. ワークフローのログを確認
   - `Actions > Claude Code Review > 該当の実行`
2. 認証情報が有効か確認
   - OAuth: Claude Code CLI で `/install-github-app` を再実行してトークンを再取得
   - API キー: https://console.anthropic.com/ でキーのステータスを確認

**認証エラーが発生する**

1. Secret が正しく設定されているか再確認（コピペミス等）
2. GitHub App がリポジトリにインストールされているか確認

---

## 変更履歴

| 日付 | 変更内容 | 担当 |
|------|---------|------|
| 2026-01-15 | Claude Code Action: OAuth トークン方式に変更、Ruleset 設定追加 | - |
| 2026-01-15 | Claude Code Action 設定手順を追加 | - |
| 2026-01-14 | 初版作成 | - |
