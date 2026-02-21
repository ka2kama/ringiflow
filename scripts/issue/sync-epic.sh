#!/usr/bin/env bash
# =============================================================================
# Story Issue の親 Epic タスクリストを自動更新するスクリプト
#
# Story Issue の body から親 Epic 番号を検出し、Epic のタスクリストで
# 該当 Story のチェックボックスを [x] に更新する。
#
# 使い方:
#   ./scripts/issue/sync-epic.sh ISSUE_NUMBER
#
# 引数:
#   ISSUE_NUMBER : GitHub Issue 番号
#
# 例:
#   ./scripts/issue/sync-epic.sh 749
#   → Issue #749 の body から「Epic: #747」を検出
#   → Epic #747 のタスクリストで「- [ ] ... #749」を「- [x] ... #749」に更新
# =============================================================================

set -euo pipefail

if [[ $# -lt 1 ]]; then
    echo "使い方: $0 ISSUE_NUMBER" >&2
    exit 1
fi

ISSUE_NUMBER="$1"

# Issue の body を取得
issue_body=$(gh issue view "$ISSUE_NUMBER" --json body --jq '.body')
if [[ -z "$issue_body" ]]; then
    echo "エラー: Issue #$ISSUE_NUMBER が見つからないか、body が空です" >&2
    exit 1
fi

# Epic 番号を抽出（「Epic: #NNN」パターン）
epic_number=$(echo "$issue_body" | grep -oP 'Epic:\s*#\K[0-9]+' | head -1 || true)
if [[ -z "$epic_number" ]]; then
    echo "ℹ️ Issue #$ISSUE_NUMBER に親 Epic が設定されていません（スキップ）"
    exit 0
fi

echo "Issue #$ISSUE_NUMBER → Epic #$epic_number"

# Epic の body を取得
epic_body=$(gh issue view "$epic_number" --json body --jq '.body')
if [[ -z "$epic_body" ]]; then
    echo "エラー: Epic #$epic_number が見つからないか、body が空です" >&2
    exit 1
fi

# Issue 番号のパターン（#NNN の後に数字が続かないことを確認し、部分一致を防ぐ）
# 例: #75 が #751 にマッチしないようにする
issue_pattern="#${ISSUE_NUMBER}([^0-9]|$)"

# 該当行が既に [x] かチェック（冪等性）
if echo "$epic_body" | grep -qP "^- \[x\] .*${issue_pattern}"; then
    echo "✓ Epic #$epic_number のタスクリストは既に更新済みです"
    exit 0
fi

# 該当行が存在するかチェック
if ! echo "$epic_body" | grep -qP "^- \[ \] .*${issue_pattern}"; then
    echo "⚠️ Epic #$epic_number のタスクリストに #$ISSUE_NUMBER が見つかりません" >&2
    exit 0
fi

# チェックボックスを更新（- [ ] → - [x]、該当行のみ）
updated_body=$(echo "$epic_body" | sed -E '/^- \[ \] .*#'"$ISSUE_NUMBER"'([^0-9]|$)/s/\[ \]/[x]/')

# Epic を更新
gh issue edit "$epic_number" --body "$updated_body"
echo "✓ Epic #$epic_number のタスクリストを更新しました（#$ISSUE_NUMBER → [x]）"
