#!/usr/bin/env bash
# Auto Review の Validation パスで使用するコンテキストを取得する。
# PR 本文、関連 Issue、計画ファイルを GITHUB_OUTPUT に出力する。
#
# 必須環境変数:
#   PR_NUMBER  - PR 番号
#   GH_REPO    - リポジトリ名（owner/repo 形式）
#   GITHUB_OUTPUT - GitHub Actions の出力先ファイル
set -euo pipefail

: "${PR_NUMBER:?PR_NUMBER is required}"
: "${GH_REPO:?GH_REPO is required}"
: "${GITHUB_OUTPUT:?GITHUB_OUTPUT is required}"

# PR 本文を取得
PR_BODY=$(gh pr view "$PR_NUMBER" --repo "$GH_REPO" --json body --jq '.body // ""')

# GitHub closing keywords パターンから Issue 番号を抽出
ISSUE_NUMBERS=$(echo "$PR_BODY" | grep -oP '(?i)(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?)\s+#\K\d+' || true)

{
  echo "VALIDATION_CONTEXT<<EOF_VALIDATION"

  echo "### PR 本文"
  echo ""
  echo "$PR_BODY"
  echo ""

  if [ -n "$ISSUE_NUMBERS" ]; then
    for NUM in $ISSUE_NUMBERS; do
      echo "### Issue #${NUM}（完了基準の参照元）"
      echo ""
      gh issue view "$NUM" --repo "$GH_REPO" --json title,body \
        --jq '"タイトル: \(.title)\n\n\(.body // "(本文なし)")"' 2>/dev/null || echo "(取得失敗)"
      echo ""

      # 計画ファイルの検出
      for FILE in "prompts/plans/${NUM}_"*.md; do
        if [ -f "$FILE" ]; then
          echo "### 計画ファイル: $(basename "$FILE")"
          echo ""
          head -c 10000 "$FILE"
          FILESIZE=$(wc -c < "$FILE")
          if [ "$FILESIZE" -gt 10000 ]; then
            echo ""
            echo "(... ${FILESIZE} bytes 中 10000 bytes を表示。全文は cat prompts/plans/$(basename "$FILE") で確認可能 ...)"
          fi
          echo ""
        fi
      done
    done
  else
    echo "(Issue 参照なし — Validation チェックは PR 本文の品質確認セクションのみで実施)"
  fi

  echo "EOF_VALIDATION"
} >> "$GITHUB_OUTPUT"
