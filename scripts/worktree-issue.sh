#!/usr/bin/env bash
# =============================================================================
# Issue 番号からブランチを作成してスロットに切り替えるスクリプト
#
# Issue タイトルからブランチ名を自動生成し、worktree-switch.sh に委譲する。
#
# 使い方:
#   ./scripts/worktree-issue.sh NUMBER [SLOT]
#
# 引数:
#   NUMBER : GitHub Issue 番号
#   SLOT   : スロット番号（省略時は現在のスロットを自動検出）
#
# 例:
#   ./scripts/worktree-issue.sh 321 1
#   → feature/321-add-hurl-api-tests-for-uncovered を生成
#   → worktree-switch.sh 1 feature/321-add-hurl-api-tests-for-uncovered を実行
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ $# -lt 1 ]]; then
    echo "使い方: $0 NUMBER [SLOT]" >&2
    echo "例: $0 321 1" >&2
    exit 1
fi

NUMBER="$1"
SLOT="${2:-}"

# スロット番号の解決
if [[ -z "$SLOT" ]]; then
    # 現在のディレクトリがスロット内かチェック
    if [[ -f "$(pwd)/.worktree-slot" ]]; then
        SLOT=$(cat "$(pwd)/.worktree-slot")
    else
        # スロットが特定できない場合、利用可能なスロットを表示してエラー
        echo "エラー: スロット番号を指定してください" >&2
        echo "" >&2
        echo "利用可能なスロット:" >&2
        found_slot=false
        while IFS= read -r wt_path; do
            slot_file="$wt_path/.worktree-slot"
            if [[ -f "$slot_file" ]]; then
                slot_num=$(cat "$slot_file")
                wt_branch=$(git -C "$wt_path" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "detached")
                if [[ "$wt_branch" == "HEAD" ]]; then
                    wt_branch="(detached HEAD - 空き)"
                fi
                echo "  スロット $slot_num: $wt_branch" >&2
                found_slot=true
            fi
        done < <(git worktree list --porcelain | grep '^worktree ' | cut -d' ' -f2-)
        if [[ "$found_slot" == false ]]; then
            echo "  （スロットがありません。just worktree-create N で作成してください）" >&2
        fi
        echo "" >&2
        echo "使い方: $0 $NUMBER <スロット番号>" >&2
        exit 1
    fi
fi

TITLE=$(gh issue view "$NUMBER" --json title -q .title)
if [[ -z "$TITLE" ]]; then
    echo "エラー: Issue #$NUMBER が見つかりません" >&2
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
echo "Slot: ${SLOT}"
echo ""

"$SCRIPT_DIR/worktree-switch.sh" "$SLOT" "$BRANCH"
