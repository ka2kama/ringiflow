#!/usr/bin/env bash
# =============================================================================
# Issue 番号から worktree を作成するスクリプト
#
# Issue タイトルからブランチ名を自動生成し、worktree-add.sh に委譲する。
#
# 使い方:
#   ./scripts/worktree-issue.sh NUMBER
#
# 例:
#   ./scripts/worktree-issue.sh 321
#   → feature/321-add-hurl-api-tests-for-uncovered を生成
#   → worktree-add.sh 321 feature/321-add-hurl-api-tests-for-uncovered を実行
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ $# -lt 1 ]]; then
    echo "使い方: $0 NUMBER" >&2
    echo "例: $0 321" >&2
    exit 1
fi

NUMBER="$1"

TITLE=$(gh issue view "$NUMBER" --json title -q .title)
if [[ -z "$TITLE" ]]; then
    echo "error: Issue #$NUMBER が見つかりません" >&2
    exit 1
fi

# タイトルからスラッグを生成（英数字とハイフンのみ）
# 切り詰めで不完全になった末尾の単語を削除
SLUG=$(echo "$TITLE" | \
    sed 's/[（(][^)）]*[)）]//g' | \
    tr '[:upper:]' '[:lower:]' | \
    sed 's/[^a-z0-9]/-/g' | \
    sed 's/-\{2,\}/-/g' | \
    sed 's/^-//' | \
    sed 's/-$//' | \
    cut -c1-50 | \
    sed 's/-[^-]*$//')

# 日本語タイトル等でスラッグが空になった場合のフォールバック
if [[ -z "$SLUG" ]]; then
    SLUG="issue"
fi

BRANCH="feature/${NUMBER}-${SLUG}"

echo "Issue #${NUMBER}: ${TITLE}"
echo "Branch: ${BRANCH}"
echo ""

"$SCRIPT_DIR/worktree-add.sh" "$NUMBER" "$BRANCH"

echo ""
echo "次のステップ:"
echo "  1. Claude Code を終了"
echo "  2. cd ../ringiflow-$NUMBER && claude"
