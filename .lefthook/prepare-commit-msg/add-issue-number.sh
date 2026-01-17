#!/bin/bash
# ブランチ名から Issue 番号を抽出してコミットメッセージの先頭に自動付与する
#
# 対象ブランチ名の例:
#   - feature/34-user-auth → #34
#   - fix/123-bug-fix → #123

COMMIT_MSG_FILE=$1
COMMIT_SOURCE=$2

# マージ、スカッシュ、amend の場合は何もしない
if [ "$COMMIT_SOURCE" = "merge" ] || [ "$COMMIT_SOURCE" = "squash" ] || [ "$COMMIT_SOURCE" = "commit" ]; then
    exit 0
fi

# 現在のブランチ名を取得
BRANCH_NAME=$(git symbolic-ref --short HEAD 2>/dev/null)

# ブランチ名が取得できない場合（detached HEAD など）は何もしない
if [ -z "$BRANCH_NAME" ]; then
    exit 0
fi

# ブランチ名から Issue 番号を抽出（例: feature/34-user-auth → 34）
# パターン: prefix/数字-description
ISSUE_NUMBER=$(echo "$BRANCH_NAME" | sed -n 's|^[^/]*/\([0-9]\+\)-.*|\1|p')

# Issue 番号が抽出できない場合は何もしない（main, develop など）
if [ -z "$ISSUE_NUMBER" ]; then
    exit 0
fi

# 現在のコミットメッセージを読み込み
COMMIT_MSG=$(cat "$COMMIT_MSG_FILE")

# すでに Issue 番号で始まっている場合はスキップ
if echo "$COMMIT_MSG" | head -1 | grep -qE "^#[0-9]+ "; then
    exit 0
fi

# Issue 番号をプレフィックスとして追加
echo "#$ISSUE_NUMBER $COMMIT_MSG" > "$COMMIT_MSG_FILE"
