# GitHub Actions

## 概要

GitHub Actions は GitHub が提供する CI/CD プラットフォーム。
リポジトリ内のイベント（push、pull_request など）をトリガーにワークフローを実行する。

## アクション許可設定

### 許可モード

Settings → Actions → General → Actions permissions で設定。

| モード | 説明 |
|--------|------|
| Allow all actions | すべてのアクションを許可 |
| Allow local actions only | 同一リポジトリ内のアクションのみ |
| Allow select actions | 許可リストに基づいて制限 |

### 許可パターンの設定

「Allow select actions」を選択した場合、許可するアクションをパターンで指定する。

```
actions/*                    # GitHub 公式アクション
owner/repo@*                 # 特定リポジトリの全バージョン
owner/repo@v1                # 特定バージョンのみ
```

### 間接的な依存に注意

一部のアクションは内部で別のアクションを呼び出す。
その場合、間接的に使用されるアクションも許可リストに追加する必要がある。

**例: `extractions/setup-just@v3`**

`setup-just@v3` は内部で `extractions/setup-crate@v1` を使用する。

```
# エラーメッセージ
The action extractions/setup-crate@v1 is not allowed in owner/repo because all actions must be from a repository owned by ...
```

**対処法:**

許可パターンに両方を追加:

```
extractions/setup-just@*
extractions/setup-crate@*   # ← 間接依存も追加
```

### 許可設定のデバッグ

CI が「action not allowed」エラーで失敗した場合:

1. エラーメッセージから不許可のアクション名を特定
2. Settings → Actions → General で許可パターンを追加
3. CI を再実行

## プロジェクトでの許可設定

| パターン | 用途 |
|----------|------|
| `actions/*` | GitHub 公式アクション |
| `dorny/paths-filter@*` | 変更検出 |
| `dtolnay/rust-toolchain@*` | Rust ツールチェーン |
| `extractions/setup-just@*` | just コマンドランナー |
| `extractions/setup-crate@*` | setup-just の間接依存 |
| `pnpm/action-setup@*` | pnpm パッケージマネージャ |
| `EmbarkStudios/cargo-deny-action@*` | 依存関係セキュリティスキャン |
| `mozilla-actions/sccache-action@*` | Rust コンパイルキャッシュ |
| `docker/setup-buildx-action@*` | Docker Buildx セットアップ（デモデプロイ） |
| `docker/login-action@*` | Docker レジストリ認証（デモデプロイ） |
| `docker/build-push-action@*` | Docker イメージのビルド・プッシュ（デモデプロイ） |

---

## dorny/paths-filter

Monorepo で変更されたファイルを検出し、必要なジョブのみを実行するためのアクション。

### on.push.paths との違い

| 観点 | `on.push.paths` | `dorny/paths-filter` |
|------|-----------------|---------------------|
| 制御単位 | ワークフロー全体 | ジョブ単位 |
| 複数パターン | 別ワークフローが必要 | 単一ワークフローで管理 |
| ブランチ保護 | 複数ステータスの管理が煩雑 | 単一ステータスで管理可能 |

### 動作原理

| イベント | 比較対象 |
|---------|---------|
| pull_request | ベースブランチ（通常 main）との差分 |
| push | `github.event.before`（前回のコミット）との差分 |

### 基本的な使い方

```yaml
jobs:
  changes:
    outputs:
      rust: ${{ steps.filter.outputs.rust }}
    steps:
      - uses: dorny/paths-filter@v3
        id: filter
        with:
          filters: |
            rust:
              - 'apps/**'
              - 'packages/**'

  rust:
    needs: changes
    if: needs.changes.outputs.rust == 'true'
```

### filters 構文

```yaml
filters: |
  フィルタ名:
    - 'glob パターン'
```

**glob パターン:**

| パターン | 意味 |
|---------|------|
| `*` | 任意の文字列（ディレクトリ区切りを除く） |
| `**` | 任意のディレクトリ階層 |
| `apps/**` | apps 配下の全ファイル |
| `*.rs` | 拡張子 .rs のファイル |

複数パターンは OR 条件で評価される。いずれかにマッチすれば true。

### outputs の型に注意

outputs は **文字列** `'true'` / `'false'` で返される。
後続ジョブの条件では文字列として比較する:

```yaml
# 正しい
if: needs.changes.outputs.rust == 'true'

# 間違い（boolean として評価されない）
if: needs.changes.outputs.rust
```

### fetch-depth: 0 が必要

paths-filter が正しく差分を比較するには、比較対象のコミット履歴が必要。
`actions/checkout` のデフォルト（shallow clone）だと、全ファイルが変更扱いになる場合がある。

```yaml
- uses: actions/checkout@v6
  with:
    fetch-depth: 0  # 全履歴を取得
```

---

## workflow_run イベント

別のワークフローの完了をトリガーにして実行するイベント。
CI 完了後に自動レビューを実行する、といったユースケースに使う。

### 基本構文

```yaml
on:
  workflow_run:
    workflows: ["CI"]  # トリガー元のワークフロー名
    types:
      - completed      # 完了時（成功・失敗問わず）
```

### コンテキスト

`github.event.workflow_run` でトリガー元の情報にアクセスできる：

```yaml
github.event.workflow_run:
  id: 12345678              # ワークフロー実行 ID
  name: "CI"                # ワークフロー名
  head_sha: "abc123..."     # コミット SHA
  conclusion: "success"     # 結果（success, failure, cancelled）
  event: "pull_request"     # 元のトリガーイベント
  pull_requests:            # 関連する PR の配列
    - number: 73
      head:
        sha: "abc123..."
        ref: "feature-branch"
      base:
        sha: "def456..."
        ref: "main"
```

### pull_requests 配列

1 つのコミットが複数の PR に関連付けられる可能性があるため、配列で提供される。
通常は `[0]` で最初の要素を取得する。

```yaml
# PR 番号の取得
PR_NUMBER=${{ github.event.workflow_run.pull_requests[0].number }}

# PR が存在するかチェック
if: github.event.workflow_run.pull_requests[0] != null
```

**含まれる情報:**
- `number`: PR 番号
- `head.sha`, `head.ref`: ソースブランチ情報
- `base.sha`, `base.ref`: ターゲットブランチ情報

**含まれない情報:**
- `isDraft`: Draft PR かどうか
- `title`, `body`: タイトル、本文
- `labels`, `assignees`: ラベル、担当者

これらの情報が必要な場合は `gh pr view` で取得する：

```yaml
IS_DRAFT=$(gh pr view "$PR_NUMBER" --repo "${{ github.repository }}" --json isDraft --jq '.isDraft')
```

### 重要な制約

#### 1. デフォルトブランチのコンテキストで実行される

`workflow_run` はデフォルトブランチ（main）のコンテキストで実行される。
そのため、PR のコミットにステータスが自動で紐付かない。

**解決策:** GitHub Status API で明示的にステータスを報告する。

```yaml
gh api "repos/${{ github.repository }}/statuses/${{ github.event.workflow_run.head_sha }}" \
  -f state=success \
  -f context="My Check" \
  -f description="Check completed"
```

→ 詳細: [ADR-011 補足](../../05_ADR/011_Claude_Code_Action導入.md#補足-workflow_run-イベントでのステータス報告)

#### 2. 同時実行制御が必要

同じ PR で複数回 CI が実行されると、複数の `workflow_run` がトリガーされる。
`concurrency` で重複実行を防止する：

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.event.workflow_run.pull_requests[0].number || github.run_id }}
  cancel-in-progress: true
```

### ユースケース

| ユースケース | 説明 |
|-------------|------|
| CI 完了後の自動レビュー | テストが通った PR のみレビューを実行 |
| デプロイ前のセキュリティスキャン | ビルド成功後にスキャンを実行 |
| 通知 | CI 結果を Slack などに通知 |

---

## バイナリキャッシュ戦略

### 背景

`e2e-test` / `api-test` ジョブは `dorny/paths-filter` でフロントエンドや API テストファイルの変更でもトリガーされる。バックエンドソースに変更がない場合でも `cargo build --release` が実行され、sccache があってもキャッシュダウンロード + リンクで数分のコストが発生する。

### アプローチ

`actions/cache` でリリースバイナリを直接キャッシュし、バックエンドソースのハッシュをキーにする。キャッシュヒット時は `cargo build` を完全にスキップする。

```yaml
- name: Cache backend binaries
  id: backend-cache
  uses: actions/cache@v5
  with:
    path: |
      backend/target/release/ringiflow-bff
      backend/target/release/ringiflow-core-service
      backend/target/release/ringiflow-auth-service
    key: ${{ runner.os }}-backend-release-${{ hashFiles('backend/**/*.rs', 'backend/**/Cargo.toml', 'backend/Cargo.lock') }}

- name: Build backend services
  if: steps.backend-cache.outputs.cache-hit != 'true'
  run: cargo build --release --no-default-features
  working-directory: backend
```

### CI キャッシュの3層構造

| 層 | ツール | 対象 | キー | restore-keys |
|----|--------|------|------|-------------|
| 依存ソース | `actions/cache` | `~/.cargo/registry/`, `~/.cargo/git/` | `Cargo.lock` | あり（部分ヒット有用） |
| 中間生成物 | sccache | `.rlib`, `.rmeta` 等 | sccache 内部管理 | — |
| 最終バイナリ | `actions/cache` | `target/release/` のバイナリ | Rust ソース全体のハッシュ | なし（完全一致必須） |

### キャッシュキーの設計

| 要素 | 含む理由 |
|------|---------|
| `runner.os` | OS ごとのバイナリ互換性 |
| `backend/**/*.rs` | Rust ソースコード。`target/` は `.gitignore` で除外されチェックアウト後には存在しない |
| `backend/**/Cargo.toml` | 依存関係、フィーチャー定義、release プロファイル設定 |
| `backend/Cargo.lock` | ロックされた依存バージョン |

Rust toolchain バージョンは含めない。ci.yaml にピン留め（1.93.0）されており、変更時は通常 Cargo.toml/Cargo.lock も変わるため。

### restore-keys を使わない理由

Cargo registry キャッシュは古いバージョンでも再利用可能なため `restore-keys` で部分ヒットが有用。一方、バイナリキャッシュは異なるソースから生成されたバイナリを実行すると不整合が起きるため、完全一致のみ許容する。

### キャッシュの共有

`actions/cache` はデフォルトブランチ（main）のキャッシュを他ブランチから参照可能。main への push で CI が走るため、フロントエンド専用ブランチでも main のキャッシュにヒットする。キャッシュ保持期間は最終アクセスから 7 日間（LRU、リポジトリ上限 10 GB）。

参照: #592、[ADR-004](../../05_ADR/004_CI並列化と変更検出.md)

---

## PR レビューシステムの制約

### 同一著者のレビューは最新のみ有効

GitHub の PR レビューシステムでは、同一著者からの複数レビューのうち **最新のもの** だけがカウントされる。

| 仕様 | 詳細 |
|------|------|
| レビュー枠 | 1著者 = 1枠 |
| 有効なレビュー | 各著者の最新レビューのみ |
| `required_approving_review_count` | 有効な APPROVED レビューの数で判定 |

### Bot アカウントでの注意

Bot アカウント（例: `claude[bot]`）で複数のワークフローからレビューを投稿する場合、最後に投稿されたレビューが他のレビューを上書きする。

```
# 問題のあるパターン
ワークフロー A: gh pr review --approve    → APPROVED
ワークフロー B: gh pr review --comment    → COMMENTED（A の APPROVED を上書き）
→ 最終状態: COMMENTED（承認条件を満たさない）
```

### 解決策

レビュー承認が不要なワークフローは `gh pr review` の代わりに `gh pr comment` を使用する。`gh pr comment` は通常のコメントであり、レビューシステムに影響しない。

| 方法 | レビューシステムへの影響 |
|------|----------------------|
| `gh pr review --approve` | APPROVED レビューを投稿（枠を使用） |
| `gh pr review --comment` | COMMENTED レビューを投稿（枠を使用） |
| `gh pr review --request-changes` | CHANGES_REQUESTED レビューを投稿（枠を使用） |
| `gh pr comment` | 通常コメント（レビューシステムに影響なし） |

参照: #390

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-02-17 | バイナリキャッシュ戦略セクションを追加（#592） |
| 2026-02-13 | sccache-action を許可設定テーブルに追加（#447） |
| 2026-02-10 | PR レビューシステムの制約セクションを追加（#390） |
| 2026-01-18 | workflow_run イベントセクションを追加 |
| 2026-01-15 | 初版作成（アクション許可設定） |
| 2026-01-15 | paths-filter セクションを追加 |
