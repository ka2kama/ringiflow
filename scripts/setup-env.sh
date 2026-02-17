#!/usr/bin/env bash
# =============================================================================
# 環境変数ファイルをセットアップする
#
# .env ファイルが存在しない場合に作成する。
# worktree の場合は空きポートオフセットを自動で割り当てる。
#
# 使い方:
#   ./scripts/setup-env.sh
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

echo "環境変数ファイルを確認中..."

# .env が既に存在する場合はスキップ
if [[ -f .env ]]; then
    echo "  確認: .env"
    echo "  確認: backend/.env"
    echo "  確認: backend/.env.api-test"
    echo "✓ 環境変数ファイル準備完了"
    exit 0
fi

# worktree かどうかを判定（.git がファイルなら worktree）
if [[ -f .git ]]; then
    # worktree: 空きオフセットを探して generate-env.sh を実行
    echo "  worktree を検出しました。空きポートを探しています..."

    # 使用中のオフセットを収集
    used_offsets=()
    while IFS= read -r wt_path; do
        env_file="$wt_path/.env"
        if [[ -f "$env_file" ]]; then
            port=$(grep -E '^POSTGRES_PORT=' "$env_file" 2>/dev/null | cut -d= -f2)
            if [[ -n "$port" ]]; then
                offset=$(( (port - 15432) / 100 ))
                used_offsets+=("$offset")
            fi
        fi
    done < <(git worktree list --porcelain | grep '^worktree ' | cut -d' ' -f2-)

    # 空きオフセットを探す（1-9）
    port_offset=""
    for i in {1..9}; do
        found=false
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
        echo "エラー: 空きポートオフセットがありません" >&2
        exit 1
    fi

    ./scripts/generate-env.sh "$port_offset"
else
    # メイン worktree: テンプレートからコピー
    cp .env.template .env
    echo "  作成: .env"
    cp backend/.env.template backend/.env
    echo "  作成: backend/.env"
    cp backend/.env.api-test.template backend/.env.api-test
    echo "  作成: backend/.env.api-test"
fi

echo "✓ 環境変数ファイル準備完了"
