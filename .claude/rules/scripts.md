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
  run: ./scripts/check-rule-files.sh
```

```just
# justfile
check-rule-files:
    ./scripts/check-rule-files.sh
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
| 汎用スクリプト | `scripts/` |
| Git hooks | `scripts/hooks/` |
| CI 専用スクリプト | `scripts/` または `.github/scripts/`（プロジェクトの規約に従う） |

