# Dependabot

## 概要

Dependabot は GitHub が提供する依存関係の自動更新サービス。
プロジェクトが使用しているライブラリやツールの新バージョンを検出し、更新用の Pull Request を自動作成する。

主な役割:
- セキュリティ脆弱性のある依存を検出・修正
- 依存ライブラリを最新に保つ
- 手動での依存更新作業を削減

## 主な機能

### 1. バージョン更新（Version Updates）

設定ファイル（`dependabot.yml`）に基づき、定期的に依存をスキャンして更新 PR を作成する。

### 2. セキュリティ更新（Security Updates）

GitHub Advisory Database で脆弱性が公開されると、影響を受ける依存の更新 PR を即座に作成する。
`dependabot.yml` の設定がなくても、リポジトリ設定で有効化できる。

### 3. アラート（Dependabot Alerts）

脆弱性のある依存を検出し、GitHub の Security タブに通知する。

## 設定ファイル

`.github/dependabot.yml` で設定する。

### 基本構造

```yaml
version: 2
updates:
  - package-ecosystem: "cargo"  # 対象のパッケージマネージャ
    directory: "/"              # package ファイルの場所
    schedule:
      interval: "weekly"        # チェック頻度
```

### 主要な設定項目

| 項目 | 説明 | 例 |
|------|------|-----|
| `package-ecosystem` | パッケージマネージャの種類 | `cargo`, `npm`, `github-actions` |
| `directory` | 設定ファイルのパス | `/`, `/apps/web` |
| `schedule.interval` | チェック頻度 | `daily`, `weekly`, `monthly` |
| `schedule.day` | 実行曜日（weekly 時） | `monday`, `tuesday`, ... |
| `open-pull-requests-limit` | 同時に開ける PR 数の上限 | `5` |
| `labels` | PR に付与するラベル | `["dependencies", "rust"]` |
| `reviewers` | 自動アサインするレビュアー | `["username"]` |
| `ignore` | 更新を無視する依存 | 下記参照 |

### 対応パッケージエコシステム

| ecosystem | 対象ファイル | 備考 |
|-----------|-------------|------|
| `cargo` | `Cargo.toml`, `Cargo.lock` | Rust |
| `npm` | `package.json`, `package-lock.json` | JavaScript/TypeScript |
| `github-actions` | `.github/workflows/*.yml` | GitHub Actions |
| `pip` | `requirements.txt`, `Pipfile` | Python |
| `gomod` | `go.mod` | Go |
| `docker` | `Dockerfile` | Docker |
| `terraform` | `*.tf` | Terraform |

### 特定の依存を無視する

```yaml
updates:
  - package-ecosystem: "npm"
    directory: "/"
    schedule:
      interval: "weekly"
    ignore:
      # 特定のパッケージを無視
      - dependency-name: "lodash"
      # バージョン範囲を指定して無視
      - dependency-name: "express"
        versions: [">=5.0.0"]
      # パターンマッチで複数パッケージを無視
      - dependency-name: "@types/*"
```

### グループ化

関連する依存をまとめて 1 つの PR にする:

```yaml
updates:
  - package-ecosystem: "npm"
    directory: "/"
    schedule:
      interval: "weekly"
    groups:
      # dev dependencies をまとめる
      development-dependencies:
        dependency-type: "development"
      # 特定パターンの依存をまとめる
      eslint:
        patterns:
          - "eslint*"
          - "@typescript-eslint/*"
```

## 運用のベストプラクティス

### スケジュール設計

| 頻度 | ユースケース |
|------|-------------|
| daily | 活発に開発中、即時の更新が必要 |
| weekly | 通常の開発、週単位でまとめて対応 |
| monthly | 安定運用フェーズ、変更を最小限に |

### PR 数の制限

`open-pull-requests-limit` は CI 負荷とレビューキャパシティを考慮して設定する。
多すぎると対応が追いつかず、古い PR が溜まる原因になる。

### ラベルの活用

```yaml
labels:
  - "dependencies"  # 全依存更新の共通ラベル
  - "rust"          # エコシステム別ラベル
```

ラベルを付けることで:
- 担当者の振り分けが容易になる
- Issue/PR 一覧でのフィルタリングができる
- 自動化ワークフロー（auto-merge 等）のトリガーに使える

## セキュリティ更新の自動マージ

GitHub Actions と組み合わせて、セキュリティ更新を自動マージできる:

```yaml
# .github/workflows/dependabot-auto-merge.yml
name: Dependabot auto-merge
on: pull_request

permissions:
  contents: write
  pull-requests: write

jobs:
  dependabot:
    runs-on: ubuntu-latest
    if: github.actor == 'dependabot[bot]'
    steps:
      - name: Dependabot metadata
        id: metadata
        uses: dependabot/fetch-metadata@v2
        with:
          github-token: "${{ secrets.GITHUB_TOKEN }}"

      - name: Enable auto-merge for patch updates
        if: steps.metadata.outputs.update-type == 'version-update:semver-patch'
        run: gh pr merge --auto --squash "$PR_URL"
        env:
          PR_URL: ${{ github.event.pull_request.html_url }}
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

## プロジェクトでの使用

### 設定ファイル

`.github/dependabot.yml` で以下のエコシステムを管理:

| エコシステム | ディレクトリ | 対象 |
|-------------|-------------|------|
| cargo | `/` | Rust ワークスペース全体 |
| npm | `/apps/web` | Elm/Vite ビルドツール |
| github-actions | `/` | CI ワークフローのアクション |

### 設定方針

- スケジュール: 週次（月曜日）
- PR 上限: 5 件/エコシステム
- ラベル: `dependencies` + エコシステム別

### 制限事項

Elm パッケージ（`elm.json` の dependencies）は Dependabot 非対応。
手動または `elm-json` ツールで更新する必要がある。

## トラブルシューティング

### PR が作成されない

1. `dependabot.yml` の構文エラーを確認
2. `directory` パスが正しいか確認（パッケージファイルがある場所）
3. GitHub の Settings → Code security and analysis で Dependabot が有効か確認

### 不要な PR が多い

- `ignore` で特定の依存を除外
- `groups` で関連パッケージをまとめる
- `open-pull-requests-limit` で上限を下げる

### CI が失敗する

Dependabot の PR で CI が失敗する場合:
1. Dependabot は secrets にアクセスできない（セキュリティ上の制限）
2. `pull_request_target` イベントを使うか、必要な secrets を `dependabot/secrets` に設定

### 複数 PR がコンフリクトする

同じファイルを変更する複数の Dependabot PR は、1 つマージすると他がコンフリクト状態になる。
例: GitHub Actions の複数アクションを更新する PR が同時に作成された場合（すべて `ci.yml` を変更）

**対処法:**

1. **リベースを依頼**: PR のコメントに `@dependabot rebase` と投稿すると、Dependabot が自動でリベースする
2. **auto-merge を活用**: ブランチ保護で CI 必須にしていれば、リベース後に auto-merge が動作する
3. **一括更新**: 複数 PR を無視し、新規ブランチで手動で一括更新する

**一括更新の手順:**

```bash
# 新規ブランチで依存を更新
git checkout -b chore/update-dependencies
# 各ファイルを編集して依存バージョンを更新
# ロックファイルを更新
pnpm install        # npm 系
cargo update        # Rust
# コミット・PR 作成後、古い Dependabot PR をクローズ
gh pr close <PR番号> --comment "PR #XX で一括更新のためクローズ"
```

## 関連リソース

- [Dependabot 公式ドキュメント](https://docs.github.com/en/code-security/dependabot)
- [dependabot.yml 設定リファレンス](https://docs.github.com/en/code-security/dependabot/dependabot-version-updates/configuration-options-for-the-dependabot.yml-file)
- [GitHub Advisory Database](https://github.com/advisories)

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-01-15 | 初版作成 |
| 2026-01-15 | 複数PRコンフリクト対応を追加 |
