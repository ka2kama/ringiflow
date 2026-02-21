#!/usr/bin/env bash
# =============================================================================
# Issue のチェックボックス状態を検証するスクリプト
#
# Issue body 内の全チェックボックスを検査し、未チェック項目があれば報告する。
# 品質ゲートで Issue との整合を確認する際に使用する。
#
# 使い方:
#   ./scripts/issue/check-issue-state.sh [ISSUE_NUMBER]
#
# 引数:
#   ISSUE_NUMBER : GitHub Issue 番号（省略時はブランチ名から自動検出）
#
# 終了コード:
#   0 : 全チェックボックスが [x]（またはチェックボックスなし）
#   1 : 未チェック項目あり
#
# 例:
#   ./scripts/issue/check-issue-state.sh 751
#   ./scripts/issue/check-issue-state.sh       # ブランチ名から自動検出
# =============================================================================

set -euo pipefail

# Issue 番号の取得
if [[ $# -ge 1 ]]; then
    ISSUE_NUMBER="$1"
else
    # ブランチ名から自動検出（feature/NNN-xxx or fix/NNN-xxx）
    branch=$(git branch --show-current 2>/dev/null || echo "")
    ISSUE_NUMBER=$(echo "$branch" | grep -oP '(?:feature|fix)/\K[0-9]+' || echo "")
    if [[ -z "$ISSUE_NUMBER" ]]; then
        echo "エラー: Issue 番号を特定できません。引数で指定するか、feature/NNN-xxx 形式のブランチで実行してください" >&2
        exit 1
    fi
fi

# Issue の body を取得
issue_body=$(gh issue view "$ISSUE_NUMBER" --json body --jq '.body')
if [[ -z "$issue_body" ]]; then
    echo "エラー: Issue #$ISSUE_NUMBER が見つからないか、body が空です" >&2
    exit 1
fi

# 未チェック項目を抽出
unchecked=$(echo "$issue_body" | grep -n '^- \[ \]' || true)

if [[ -z "$unchecked" ]]; then
    checked_count=$(echo "$issue_body" | grep -c '^- \[x\]' || true)
    echo "✓ Issue #$ISSUE_NUMBER: 全チェックボックスが完了済み（${checked_count} 件）"
    exit 0
fi

unchecked_count=$(echo "$unchecked" | wc -l)
checked_count=$(echo "$issue_body" | grep -c '^- \[x\]' || true)
total=$((checked_count + unchecked_count))

echo "⚠️ Issue #$ISSUE_NUMBER: 未チェック項目があります（${unchecked_count}/${total} 件未完了）"
echo ""
echo "未チェック項目:"
echo "$unchecked" | while IFS= read -r line; do
    # 行番号プレフィックス（NNN:）を除去して内容だけ表示
    content="${line#*:}"
    echo "  $content"
done
exit 1
