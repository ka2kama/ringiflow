#!/usr/bin/env bash
# =============================================================================
# 永続 worktree スロット内のブランチを切り替えるスクリプト
#
# スロット内で git switch を実行し、DB マイグレーションと依存関係の
# 差分更新を自動で行う。
#
# 使い方:
#   ./scripts/worktree-switch.sh N BRANCH
#
# 引数:
#   N      : スロット番号（1-9）
#   BRANCH : 切り替え先のブランチ名
#
# 例:
#   ./scripts/worktree-switch.sh 1 feature/625-persistent-slots
#   → スロット 1 のブランチを feature/625-persistent-slots に切り替え
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# 引数チェック
if [[ $# -lt 2 ]]; then
    echo "使い方: $0 N BRANCH" >&2
    echo "例: $0 1 feature/625-persistent-slots" >&2
    exit 1
fi

N="$1"
BRANCH="$2"

# スロット番号の検証（1-9）
if ! [[ "$N" =~ ^[1-9]$ ]]; then
    echo "エラー: スロット番号は 1-9 の数字である必要があります（指定: $N）" >&2
    exit 1
fi

PARENT_DIR=$(dirname "$PROJECT_ROOT")
WORKTREE_PATH="${PARENT_DIR}/ringiflow-${N}"

# スロットの存在確認
if [[ ! -f "$WORKTREE_PATH/.worktree-slot" ]]; then
    echo "エラー: スロット $N が見つかりません: $WORKTREE_PATH" >&2
    echo "  スロットを作成するには: just worktree-create $N" >&2
    exit 1
fi

# 未コミット変更のチェック
changes=$(git -C "$WORKTREE_PATH" status --porcelain 2>/dev/null || true)
if [[ -n "$changes" ]]; then
    echo "エラー: スロット $N に未コミットの変更があります" >&2
    echo "  変更をコミットまたは stash してからブランチを切り替えてください" >&2
    echo "" >&2
    git -C "$WORKTREE_PATH" status --short >&2
    exit 1
fi

# 切り替え前の HEAD を記録（pnpm-lock.yaml の差分チェック用）
PREV_HEAD=$(git -C "$WORKTREE_PATH" rev-parse HEAD 2>/dev/null || echo "")

# リモートの最新情報を取得
git -C "$WORKTREE_PATH" fetch origin --quiet

echo "スロット $N のブランチを切り替え中: $BRANCH"

# ブランチ切り替え
if git -C "$WORKTREE_PATH" rev-parse --verify "$BRANCH" >/dev/null 2>&1; then
    # ローカルに存在するブランチ
    git -C "$WORKTREE_PATH" switch "$BRANCH"
elif git -C "$WORKTREE_PATH" ls-remote --exit-code origin "$BRANCH" >/dev/null 2>&1; then
    # リモートのみに存在するブランチ
    git -C "$WORKTREE_PATH" switch -c "$BRANCH" "origin/$BRANCH"
else
    # 新規ブランチ（origin/main から作成）
    git -C "$WORKTREE_PATH" switch -c "$BRANCH" origin/main
fi

# DB マイグレーション
echo ""
echo "DB マイグレーションを実行中..."
(
    cd "$WORKTREE_PATH"
    env -u POSTGRES_PORT -u REDIS_PORT -u DYNAMODB_PORT \
        -u API_TEST_POSTGRES_PORT -u API_TEST_REDIS_PORT -u API_TEST_DYNAMODB_PORT \
        -u BFF_PORT -u VITE_PORT \
        just db-migrate
)

# pnpm-lock.yaml の差分チェック
# diff が失敗する場合（初回切り替え等）は安全側に倒して pnpm install を実行
pnpm_changed=false
if [[ -n "$PREV_HEAD" ]]; then
    if ! git -C "$WORKTREE_PATH" diff --quiet "$PREV_HEAD"..HEAD -- \
        pnpm-lock.yaml frontend/pnpm-lock.yaml tests/e2e/pnpm-lock.yaml 2>/dev/null; then
        pnpm_changed=true
    fi
else
    # PREV_HEAD が取得できない場合は安全側に倒す
    pnpm_changed=true
fi

if [[ "$pnpm_changed" == true ]]; then
    echo ""
    echo "pnpm-lock.yaml に差分を検出。依存関係を更新中..."
    (cd "$WORKTREE_PATH" && pnpm install)
    (cd "$WORKTREE_PATH/frontend" && pnpm install)
    (cd "$WORKTREE_PATH/tests/e2e" && pnpm install)
fi

echo ""
echo "✓ スロット $N を $BRANCH に切り替えました"
echo "  cd $WORKTREE_PATH"
