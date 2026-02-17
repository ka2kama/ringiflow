#!/usr/bin/env bash
# =============================================================================
# Git worktree 追加スクリプト
#
# 並行開発用の独立した作業ディレクトリを作成する。
# ポートオフセットは使用中の worktree を検出して自動割り当てする。
#
# 使い方:
#   ./scripts/worktree-add.sh [--no-setup] NAME BRANCH
#
# オプション:
#   --no-setup : セットアップ（Docker 起動、DB マイグレーション、依存関係インストール）をスキップ
#
# 引数:
#   NAME   : worktree 名（ディレクトリ名に使用）
#   BRANCH : ブランチ名（存在しなければ新規作成）
#
# 例:
#   ./scripts/worktree-add.sh auth feature/auth
#   → ringiflow-auth/ ディレクトリを作成し、feature/auth ブランチをチェックアウト
#   ./scripts/worktree-add.sh --no-setup auth feature/auth
#   → セットアップをスキップ（.env 生成のみ）
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# オプション解析
NO_SETUP=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-setup)
            NO_SETUP=true
            shift
            ;;
        -*)
            echo "不明なオプション: $1" >&2
            exit 1
            ;;
        *)
            break
            ;;
    esac
done

# 引数チェック
if [[ $# -lt 2 ]]; then
    echo "使い方: $0 [--no-setup] NAME BRANCH" >&2
    echo "例: $0 auth feature/auth" >&2
    exit 1
fi

NAME="$1"
BRANCH="$2"

PARENT_DIR=$(dirname "$PROJECT_ROOT")
WORKTREE_PATH="${PARENT_DIR}/ringiflow-${NAME}"

# 使用中のオフセットを収集（各 worktree の .env から POSTGRES_PORT を読み取り）
used_offsets=()
while IFS= read -r wt_path; do
    env_file="$wt_path/.env"
    if [[ -f "$env_file" ]]; then
        port=$(grep -E '^POSTGRES_PORT=' "$env_file" 2>/dev/null | cut -d= -f2)
        if [[ -n "$port" ]]; then
            # ベースポート 15432 からのオフセットを計算（100単位）
            offset=$(( (port - 15432) / 100 ))
            used_offsets+=("$offset")
        fi
    fi
done < <(git worktree list --porcelain | grep '^worktree ' | cut -d' ' -f2-)

# 空きオフセットを探す（1-9、0 はメイン用）
port_offset=""
for i in {1..9}; do
    found=false
    # 配列が空でない場合のみチェック
    if [[ ${#used_offsets[@]} -gt 0 ]]; then
        for used in "${used_offsets[@]}"; do
            if [[ "$used" == "$i" ]]; then
                found=true
                break
            fi
        done
    fi
    if [[ "$found" == false ]]; then
        port_offset="$i"
        break
    fi
done

if [[ -z "$port_offset" ]]; then
    echo "エラー: 空きポートオフセットがありません（最大9個まで）" >&2
    exit 1
fi

echo "worktree を作成中: ${NAME}"
echo "  パス: $WORKTREE_PATH"
echo "  ブランチ: ${BRANCH}"
echo "  ポートオフセット: $port_offset（自動割り当て）"

# リモートの最新 main を取得
git fetch origin main --quiet

# worktree を追加（ブランチがなければ origin/main から作成）
if git rev-parse --verify "${BRANCH}" >/dev/null 2>&1; then
    git worktree add "$WORKTREE_PATH" "${BRANCH}"
else
    git worktree add -b "${BRANCH}" "$WORKTREE_PATH" origin/main
fi

# .env を生成
cd "$WORKTREE_PATH"
./scripts/generate-env.sh "$port_offset"

# セットアップ実行（--no-setup でスキップ）
if [[ "$NO_SETUP" == false ]]; then
    echo ""
    echo "セットアップを実行中..."
    # 親プロセス（メインリポジトリの just）から継承されたポート環境変数をクリアする。
    # just の dotenv-load は既存の環境変数を上書きしないため、
    # クリアしないと worktree の .env ではなくメインの .env の値が使われてしまう。
    env -u POSTGRES_PORT -u REDIS_PORT -u DYNAMODB_PORT -u BFF_PORT -u VITE_PORT just setup-worktree
else
    echo ""
    echo "（--no-setup: セットアップをスキップしました）"
fi

echo ""
echo "✓ worktree を作成しました"
echo "  cd $WORKTREE_PATH"
