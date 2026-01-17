# lefthook

Git フック管理ツール。コミット時やプッシュ時に自動でスクリプトを実行できる。

公式: https://github.com/evilmartians/lefthook

## 概要

lefthook は Evil Martians が開発する Git フック管理ツール。Go で書かれたシングルバイナリで、Node.js や Ruby などの依存なしに動作する。

採用実績: GitLab, Discourse, Logux など

## 類似ツールとの比較

| ツール | 言語依存 | 特徴 |
|--------|---------|------|
| lefthook | なし | シングルバイナリ、YAML 設定、高速 |
| husky | Node.js | npm エコシステムと統合、広く普及 |
| Overcommit | Ruby | Ruby プロジェクト向け |
| pre-commit | Python | Python プロジェクト向け、多数のフック提供 |

## インストール

```bash
# mise
mise use -g lefthook

# Homebrew
brew install lefthook

# Go
go install github.com/evilmartians/lefthook@latest

# npm
npm install -g lefthook
```

## 基本的な使い方

### 初期化

```bash
lefthook install
```

`.git/hooks/` にフックが作成される。

### 設定ファイル

`lefthook.yml` をプロジェクトルートに配置する。

```yaml
# コミット前に実行
pre-commit:
  parallel: true
  commands:
    lint:
      run: npm run lint
    test:
      run: npm test

# プッシュ前に実行
pre-push:
  commands:
    build:
      run: npm run build
```

### スクリプトを使う場合

```yaml
prepare-commit-msg:
  scripts:
    "add-issue-number.sh":
      runner: bash
```

スクリプトは `.lefthook/<フック名>/` に配置する。

```
.lefthook/
└── prepare-commit-msg/
    └── add-issue-number.sh
```

## RingiFlow での使用

### Issue 番号の自動付与

ブランチ名から Issue 番号を抽出してコミットメッセージの先頭に追加する。

```yaml
# lefthook.yml
prepare-commit-msg:
  scripts:
    "add-issue-number.sh":
      runner: bash
```

```bash
# .lefthook/prepare-commit-msg/add-issue-number.sh
#!/bin/bash

COMMIT_MSG_FILE=$1
BRANCH_NAME=$(git symbolic-ref --short HEAD 2>/dev/null)

# feature/34-xxx → 34
ISSUE_NUMBER=$(echo "$BRANCH_NAME" | sed -n 's|^[^/]*/\([0-9]\+\)-.*|\1|p')

if [ -z "$ISSUE_NUMBER" ]; then
    exit 0
fi

COMMIT_MSG=$(cat "$COMMIT_MSG_FILE")

# すでに #数字 で始まっている場合はスキップ
if echo "$COMMIT_MSG" | head -1 | grep -qE "^#[0-9]+ "; then
    exit 0
fi

echo "#$ISSUE_NUMBER $COMMIT_MSG" > "$COMMIT_MSG_FILE"
```

### 動作例

```bash
# ブランチ: feature/34-user-auth

git commit -m "UserRepository を実装"
# → コミットメッセージ: "#34 UserRepository を実装"
```

## よく使うフック

| フック | タイミング | 用途 |
|--------|-----------|------|
| `pre-commit` | コミット前 | リント、フォーマット |
| `commit-msg` | コミットメッセージ入力後 | メッセージ検証 |
| `prepare-commit-msg` | コミットメッセージ編集前 | メッセージ自動生成・修正 |
| `pre-push` | プッシュ前 | テスト、ビルド |

## Tips

### フックをスキップする

```bash
# 一時的にスキップ
git commit --no-verify -m "WIP"

# 環境変数でスキップ
LEFTHOOK=0 git commit -m "WIP"
```

### 並列実行

```yaml
pre-commit:
  parallel: true
  commands:
    lint:
      run: npm run lint
    format:
      run: npm run format
```

### ファイルフィルタリング

```yaml
pre-commit:
  commands:
    eslint:
      glob: "*.{js,ts}"
      run: eslint {staged_files}
```

## 参考資料

- [lefthook - GitHub](https://github.com/evilmartians/lefthook)
- [Lefthook: knock your team's code back into shape](https://evilmartians.com/chronicles/lefthook-knock-your-teams-code-back-into-shape)
