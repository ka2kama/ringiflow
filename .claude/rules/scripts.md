---
paths:
  - "justfile"
  - ".github/workflows/**/*.yaml"
  - "mprocs.yaml"
  - "lefthook.yaml"
---

# スクリプト分離ルール

justfile、CI ワークフロー、設定ファイル内の複雑な処理は、別ファイルに切り出す。

## 分離の判断基準

以下のいずれかに該当する場合、`scripts/` ディレクトリにシェルスクリプトとして切り出す:

| 基準 | 例 |
|------|-----|
| 5行を超える | 複数のコマンドを連結した処理 |
| 条件分岐を含む | `if` / `case` / `[ -f ... ]` など |
| ループを含む | `for` / `while` |
| 再利用の可能性がある | 複数の場所で同じ処理を行う |

## 良い例

```yaml
# .github/workflows/check-rule-files.yaml
- name: Check rule files
  run: ./scripts/check/rule-files.sh
```

```just
# justfile
check-rule-files:
    ./scripts/check/rule-files.sh
```

## 悪い例

```yaml
# .github/workflows/check-rule-files.yaml
- name: Check rule files
  run: |
    set -euo pipefail
    ERRORS=()
    for file in .claude/rules/*.md; do
      # ... 20行以上のスクリプト
    done
```

## スクリプトファイルの配置

| 用途 | 配置先 |
|------|--------|
| 品質チェック | `scripts/check/` |
| テスト実行 | `scripts/test/` |
| worktree 管理 | `scripts/worktree/` |
| 環境変数管理 | `scripts/env/` |
| 独立ツール | `scripts/tools/` |
| Git hooks | `scripts/hooks/` |

## 開発ツール追加時の必須対応

新しい開発ツールを追加する場合、以下を同時に更新:

1. `justfile` の `check-tools` タスク
2. [`docs/60_手順書/01_開発参画/01_開発環境構築.md`](../../docs/60_手順書/01_開発参画/01_開発環境構築.md)

## CI ワークフロー変更時の必須対応

GitHub Actions ワークフローに新しい Action を追加した場合、以下を実施する。

→ 詳細: [ナレッジベース: GitHub Actions](../../docs/80_ナレッジベース/devtools/GitHubActions.md#アクション許可設定)

1. [ナレッジベースの許可設定テーブル](../../docs/80_ナレッジベース/devtools/GitHubActions.md#プロジェクトでの許可設定)に Action のパターンを追記する
2. 間接依存（Action が内部で呼び出す別の Action）がないか確認し、あれば同様に追記する
3. GitHub Settings → Actions → General の許可パターンにも追加する（リポジトリ管理者が手動で実施）

**禁止:** 許可設定を更新せずに CI ワークフローに新しい Action を追加すること

