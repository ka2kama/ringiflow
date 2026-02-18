#!/usr/bin/env bash
# =============================================================================
# 永続 worktree スロットを作成するスクリプト
#
# 固定の作業ディレクトリを作成する。スロットは削除せずに再利用する。
# ブランチの切り替えは worktree-switch.sh で行う。
#
# 使い方:
#   ./scripts/worktree-create.sh N
#
# 引数:
#   N : スロット番号（1-9）。ポートオフセットとしても使用される
#
# 例:
#   ./scripts/worktree-create.sh 1
#   → ringiflow-1/ ディレクトリを作成（detached HEAD、ポートオフセット 1）
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# 引数チェック
if [[ $# -lt 1 ]]; then
    echo "使い方: $0 N" >&2
    echo "例: $0 1" >&2
    exit 1
fi

N="$1"

# スロット番号の検証（1-9）
if ! [[ "$N" =~ ^[1-9]$ ]]; then
    echo "エラー: スロット番号は 1-9 の数字である必要があります（指定: $N）" >&2
    exit 1
fi

PARENT_DIR=$(dirname "$PROJECT_ROOT")
WORKTREE_PATH="${PARENT_DIR}/ringiflow-${N}"

# 既存チェック
if [[ -d "$WORKTREE_PATH" ]]; then
    echo "エラー: スロット $N は既に存在します: $WORKTREE_PATH" >&2
    exit 1
fi

echo "永続スロットを作成中: $N"
echo "  パス: $WORKTREE_PATH"
echo "  ポートオフセット: $N"

# リモートの最新 main を取得
git fetch origin main --quiet

# detached HEAD で worktree を作成
git worktree add --detach "$WORKTREE_PATH" origin/main

# マーカーファイルを作成（cleanup.sh が永続スロットを識別するために使用）
echo "$N" > "$WORKTREE_PATH/.worktree-slot"

# .env を生成（offset = スロット番号）
cd "$WORKTREE_PATH"
./scripts/generate-env.sh "$N"

# セットアップ実行
echo ""
echo "セットアップを実行中..."
# 親プロセス（メインリポジトリの just）から継承されたポート環境変数をクリアする。
# just の dotenv-load は既存の環境変数を上書きしないため、
# クリアしないと worktree の .env ではなくメインの .env の値が使われてしまう。
env -u POSTGRES_PORT -u REDIS_PORT -u DYNAMODB_PORT \
    -u API_TEST_POSTGRES_PORT -u API_TEST_REDIS_PORT -u API_TEST_DYNAMODB_PORT \
    -u BFF_PORT -u VITE_PORT \
    just setup-worktree

echo ""
echo "✓ 永続スロット $N を作成しました"
echo "  cd $WORKTREE_PATH"
echo ""
echo "ブランチを切り替えるには:"
echo "  just worktree-switch $N <ブランチ名>"
