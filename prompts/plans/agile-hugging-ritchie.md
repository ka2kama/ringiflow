# scripts/ ディレクトリの整理

## Context

`scripts/` に 19 個のシェルスクリプトがフラットに配置されており、目的別に分類されていない。スクリプト数が増えるにつれ見通しが悪くなっている。機能別サブディレクトリに整理し、ファイル名からも冗長なプレフィックスを除去する。

## 対象

`scripts/` 配下の 19 個のシェルスクリプトすべて。

## 対象外

- スクリプトの機能変更（パス変更のみ）
- スクリプトの新規追加・削除

## ディレクトリ構成

```
scripts/
├── check/           # 7 個: コード・ドキュメント品質チェック
│   ├── parallel.sh               ← check-parallel.sh
│   ├── doc-links.sh              ← check-doc-links.sh
│   ├── file-size.sh              ← check-file-size.sh
│   ├── fn-size.sh                ← check-fn-size.sh
│   ├── impl-docs.sh              ← check-impl-docs.sh
│   ├── improvement-records.sh    ← check-improvement-records.sh
│   └── rule-files.sh             ← check-rule-files.sh
├── test/            # 3 個: テスト実行
│   ├── reset-db.sh               ← api-test-reset-db.sh
│   ├── run-api.sh                ← run-api-tests.sh
│   └── run-e2e.sh                ← run-e2e-tests.sh
├── worktree/        # 4 個: worktree・ブランチ管理
│   ├── create.sh                 ← worktree-create.sh
│   ├── switch.sh                 ← worktree-switch.sh
│   ├── issue.sh                  ← worktree-issue.sh
│   └── cleanup.sh                ← cleanup.sh
├── env/             # 2 個: 環境変数ファイル管理
│   ├── setup.sh                  ← setup-env.sh
│   └── generate.sh               ← generate-env.sh
└── tools/           # 3 個: 独立ツール
    ├── dump-schema.sh            ← dump-schema.sh
    ├── mcp-postgres.sh           ← mcp-postgres.sh
    └── deploy-lightsail.sh       ← deploy-lightsail.sh
```

### 配置判断の理由

| スクリプト | 配置先 | 理由 |
|-----------|--------|------|
| `cleanup.sh` | `worktree/` | `.worktree-slot` マーカーファイルとの密結合、worktree のリセットロジックを含む |
| `api-test-reset-db.sh` | `test/` | テスト実行ワークフローの前準備（justfile で `test-api` から呼ばれる） |
| `dump-schema.sh` | `tools/` | DB スキーマ操作だが db/ にすると 1 ファイル。justfile + CI から呼ばれる独立ツール |
| `mcp-postgres.sh` | `tools/` | `.mcp.json` から呼ばれる独立ツール |
| `deploy-lightsail.sh` | `tools/` | CI からのみ呼ばれるデプロイ用独立ツール |

### ファイル名変更方針

ディレクトリがプレフィックスの役割を担うため、冗長なプレフィックスを除去する:
- `check-xxx.sh` → `xxx.sh`（`check/` ディレクトリで自明）
- `worktree-xxx.sh` → `xxx.sh`（`worktree/` ディレクトリで自明）
- `run-xxx-tests.sh` → `run-xxx.sh`（`test/` ディレクトリで自明）
- `api-test-reset-db.sh` → `reset-db.sh`（`test/` ディレクトリで自明）
- `setup-env.sh` / `generate-env.sh` → `setup.sh` / `generate.sh`（`env/` ディレクトリで自明）

## パス変更一覧

### justfile（17 箇所）

| 現在のパス | 新しいパス |
|---|---|
| `./scripts/setup-env.sh` | `./scripts/env/setup.sh` |
| `./scripts/dump-schema.sh` (×2: L217, L483) | `./scripts/tools/dump-schema.sh` |
| `./scripts/api-test-reset-db.sh` | `./scripts/test/reset-db.sh` |
| `./scripts/run-api-tests.sh` | `./scripts/test/run-api.sh` |
| `./scripts/run-e2e-tests.sh` | `./scripts/test/run-e2e.sh` |
| `./scripts/check-file-size.sh` | `./scripts/check/file-size.sh` |
| `./scripts/check-fn-size.sh` | `./scripts/check/fn-size.sh` |
| `./scripts/check-improvement-records.sh` | `./scripts/check/improvement-records.sh` |
| `./scripts/check-rule-files.sh` | `./scripts/check/rule-files.sh` |
| `./scripts/check-doc-links.sh` | `./scripts/check/doc-links.sh` |
| `./scripts/check-impl-docs.sh` | `./scripts/check/impl-docs.sh` |
| `./scripts/check-parallel.sh` (×2: L444, L449) | `./scripts/check/parallel.sh` |
| `./scripts/cleanup.sh` | `./scripts/worktree/cleanup.sh` |
| `./scripts/worktree-create.sh` | `./scripts/worktree/create.sh` |
| `./scripts/worktree-switch.sh` | `./scripts/worktree/switch.sh` |
| `./scripts/worktree-issue.sh` | `./scripts/worktree/issue.sh` |

### CI ワークフロー（3 箇所）

| ファイル | 現在のパス | 新しいパス |
|---|---|---|
| `.github/workflows/deploy-demo.yaml` L79 | `./scripts/deploy-lightsail.sh` | `./scripts/tools/deploy-lightsail.sh` |
| `.github/workflows/check-rule-files.yaml` L30 | `./scripts/check-rule-files.sh` | `./scripts/check/rule-files.sh` |
| `.github/workflows/ci.yaml` L280 | `./scripts/dump-schema.sh` | `./scripts/tools/dump-schema.sh` |

### .mcp.json（1 箇所）

| 現在のパス | 新しいパス |
|---|---|
| `scripts/mcp-postgres.sh` | `scripts/tools/mcp-postgres.sh` |

### スクリプト内部の相互参照（4 箇所）

| ファイル | 行 | 変更前 | 変更後 | 補足 |
|---|---|---|---|---|
| `env/setup.sh` | L41 | `./scripts/generate-env.sh "$offset"` | `"$SCRIPT_DIR/generate.sh" "$offset"` | SCRIPT_DIR は L14 で定義済み |
| `env/setup.sh` | L88 | `./scripts/generate-env.sh "$port_offset"` | `"$SCRIPT_DIR/generate.sh" "$port_offset"` | 同上 |
| `worktree/create.sh` | L63 | `./scripts/generate-env.sh "$N"` | `./scripts/env/generate.sh "$N"` | L62 で `cd "$WORKTREE_PATH"` 後の相対パス |
| `worktree/issue.sh` | L95 | `"$SCRIPT_DIR/worktree-switch.sh"` | `"$SCRIPT_DIR/switch.sh"` | 同じディレクトリ内の参照 |

### mcp-postgres.sh の PROJECT_ROOT 修正（1 箇所）

`tools/` に移動すると `SCRIPT_DIR` が `scripts/tools/` を指すため、`PROJECT_ROOT` の計算を修正:

```bash
# 変更前（L13）
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# 変更後
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"
```

### スクリプト内コメントの Usage 行（影響なし、一貫性のため更新）

各スクリプトの冒頭コメントにある `# 使い方: ./scripts/xxx.sh` を新パスに更新する。

## Phase 分割

### Phase 1: `git mv` でファイル移動

サブディレクトリを作成し、`git mv` で全スクリプトを移動 + リネーム。

確認事項: なし（既知のパターンのみ）

### Phase 2: スクリプト内部のパス修正

スクリプト間の相互参照パスと `PROJECT_ROOT` 計算を修正。各スクリプトの冒頭コメント（Usage 行）も更新。

確認事項: なし（Phase 3 の前提調査で特定済み）

### Phase 3: justfile のパス更新

justfile 内の 17 箇所のスクリプトパスを新パスに一括更新。

確認事項: なし（Grep で全箇所特定済み）

### Phase 4: CI・設定ファイルのパス更新

CI ワークフロー 3 ファイルと `.mcp.json` のパスを更新。

確認事項: なし（Grep で全箇所特定済み）

### Phase 5: 動作検証

`just check` で品質チェックが通ることを確認。

## 検証方法

```bash
just check       # check-parallel.sh 経由で check 系スクリプト全体を検証
just setup-env   # env/setup.sh → env/generate.sh の呼び出しチェーンを検証
```

justfile 経由の呼び出し、スクリプト間の内部呼び出し、CI パスの整合性がすべてカバーされる。
