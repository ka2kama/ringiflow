#!/usr/bin/env bash
# =============================================================================
# 不要なブランチとワークツリーを整理するスクリプト
#
# 以下を検出して削除する:
# - マージ済みブランチに紐づくワークツリー（Docker コンテナも停止）
# - リモートブランチが削除されたワークツリー（squash merge 後など）
# - ワークツリーに紐づかないマージ済みローカルブランチ
#
# 使い方:
#   ./scripts/cleanup.sh [--dry-run]
#
# オプション:
#   --dry-run : 削除対象を表示するだけで、実際の削除は行わない
# =============================================================================

set -euo pipefail

DRY_RUN=false
if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN=true
fi

# メインワークツリーに移動（ワークツリーからの実行に対応）
main_worktree=$(git worktree list --porcelain | sed -n '1s/^worktree //p')
original_dir=$(pwd)

if [[ "$(pwd)" != "$main_worktree" ]]; then
    echo "ワークツリーからの実行を検出。メインワークツリーに移動: $main_worktree"
    cd "$main_worktree"
fi

current_branch=$(git rev-parse --abbrev-ref HEAD)
if [[ "$current_branch" != "main" ]]; then
    echo "エラー: メインワークツリーが main ブランチではありません（現在: $current_branch）" >&2
    exit 1
fi

echo "リモートの最新情報を取得中..."
git fetch origin --prune

# =============================================================================
# ワークツリーの整理
# =============================================================================
echo ""
echo "=== ワークツリー ==="

stale_worktrees=()
wt_path=""
wt_branch=""

while IFS= read -r line; do
    if [[ "$line" == worktree\ * ]]; then
        wt_path="${line#worktree }"
        wt_branch=""
    elif [[ "$line" == branch\ refs/heads/* ]]; then
        wt_branch="${line#branch refs/heads/}"
    elif [[ -z "$line" && -n "$wt_path" && -n "$wt_branch" ]]; then
        # 空行でレコード区切り。main は除外
        if [[ "$wt_branch" != "main" ]]; then
            reason=""

            # マージ済みかチェック
            if git branch --merged origin/main | grep -qw "$wt_branch"; then
                reason="マージ済み"
            fi

            # リモートブランチが削除されているかチェック（squash merge 対応）
            if [[ -z "$reason" ]]; then
                tracking=$(git for-each-ref --format='%(upstream:track)' "refs/heads/$wt_branch" 2>/dev/null || true)
                if [[ "$tracking" == "[gone]" ]]; then
                    reason="リモートブランチ削除済み"
                fi
            fi

            if [[ -n "$reason" ]]; then
                stale_worktrees+=("${wt_path}|${wt_branch}|${reason}")
            fi
        fi
        wt_path=""
        wt_branch=""
    fi
done < <(git worktree list --porcelain; echo "")
# ↑ 末尾に空行を追加して最後のレコードも処理する

if [[ ${#stale_worktrees[@]} -eq 0 ]]; then
    echo "  整理対象のワークツリーはありません"
else
    for entry in "${stale_worktrees[@]}"; do
        IFS='|' read -r path branch reason <<< "$entry"
        name=$(basename "$path" | sed 's/^ringiflow-//')

        # 未コミットの変更があるか確認
        dirty=""
        if [[ -d "$path" ]]; then
            changes=$(git -C "$path" status --porcelain 2>/dev/null || true)
            if [[ -n "$changes" ]]; then
                dirty=" ⚠ 未コミットの変更あり"
            fi
        fi

        echo "  🗑  ${branch} (${path})${dirty}"
        echo "      理由: ${reason}"

        if [[ -n "$dirty" && "$DRY_RUN" == false ]]; then
            echo "      → 未コミットの変更があるためスキップします"
            continue
        fi

        if [[ "$DRY_RUN" == false ]]; then
            # Docker コンテナを停止・削除
            project_name="ringiflow-${name}"
            containers=$(docker compose -p "$project_name" -f infra/docker/docker-compose.yaml ps -q 2>/dev/null || true)
            if [[ -n "$containers" ]]; then
                echo "      Docker コンテナを停止中..."
                docker compose -p "$project_name" -f infra/docker/docker-compose.yaml down -v 2>/dev/null || true
            fi

            # ワークツリーを削除
            git worktree remove "$path" --force 2>/dev/null || true

            # ローカルブランチを削除
            git branch -D "$branch" 2>/dev/null || true

            echo "      ✓ 削除完了"
        fi
    done
fi

# =============================================================================
# ブランチの整理（ワークツリーに紐づかないもの）
# =============================================================================
echo ""
echo "=== ブランチ ==="

# ワークツリーに紐づくブランチを収集
worktree_branches=()
while IFS= read -r line; do
    if [[ "$line" == branch\ refs/heads/* ]]; then
        worktree_branches+=("${line#branch refs/heads/}")
    fi
done < <(git worktree list --porcelain)

stale_branches=()
while IFS= read -r branch; do
    branch=$(echo "$branch" | xargs)
    [[ -z "$branch" || "$branch" == "main" || "$branch" == *"*"* ]] && continue

    # ワークツリーに紐づくブランチはスキップ（上で処理済み）
    is_worktree=false
    for wt_branch in "${worktree_branches[@]+"${worktree_branches[@]}"}"; do
        if [[ "$wt_branch" == "$branch" ]]; then
            is_worktree=true
            break
        fi
    done
    [[ "$is_worktree" == true ]] && continue

    stale_branches+=("$branch")
done < <(git branch --merged origin/main)

# リモートブランチが gone のブランチも追加（squash merge 対応）
while IFS= read -r branch; do
    branch=$(echo "$branch" | xargs)
    [[ -z "$branch" || "$branch" == "main" || "$branch" == *"*"* ]] && continue

    # ワークツリーに紐づくブランチはスキップ
    is_worktree=false
    for wt_branch in "${worktree_branches[@]+"${worktree_branches[@]}"}"; do
        if [[ "$wt_branch" == "$branch" ]]; then
            is_worktree=true
            break
        fi
    done
    [[ "$is_worktree" == true ]] && continue

    # 既に stale_branches に含まれている場合はスキップ
    already_found=false
    for sb in "${stale_branches[@]+"${stale_branches[@]}"}"; do
        if [[ "$sb" == "$branch" ]]; then
            already_found=true
            break
        fi
    done
    [[ "$already_found" == true ]] && continue

    tracking=$(git for-each-ref --format='%(upstream:track)' "refs/heads/$branch" 2>/dev/null || true)
    if [[ "$tracking" == "[gone]" ]]; then
        stale_branches+=("$branch")
    fi
done < <(git branch --format='%(refname:short)')

if [[ ${#stale_branches[@]} -eq 0 ]]; then
    echo "  整理対象のブランチはありません"
else
    for branch in "${stale_branches[@]}"; do
        echo "  🗑  ${branch}"
        if [[ "$DRY_RUN" == false ]]; then
            git branch -D "$branch" 2>/dev/null || true
            echo "      ✓ 削除完了"
        fi
    done
fi

# ワークツリーの管理ファイルをクリーンアップ
git worktree prune 2>/dev/null || true

echo ""
if [[ "$DRY_RUN" == true ]]; then
    echo "（ドライラン: 実際の削除は行いません）"
    echo "削除するには: just cleanup"
else
    echo "✓ 整理完了"
fi

# ワークツリーから実行された場合、元のディレクトリが削除されていれば案内
if [[ "$original_dir" != "$main_worktree" && ! -d "$original_dir" ]]; then
    echo ""
    echo "⚠ 実行元のワークツリーが削除されました"
    echo "  → cd $main_worktree"
fi
