#!/usr/bin/env zsh
# =============================================================================
# cw - Claude Code worktree switcher
#
# worktree に移動して Claude Code を起動するシェル関数。
# cd は親シェルで実行する必要があるため、justfile ではなくシェル関数として提供する。
#
# セットアップ:
#   echo 'source ~/path/to/ringiflow/scripts/cw.zsh' >> ~/.zshrc
#
# 使い方:
#   cw              worktree 一覧を表示
#   cw NAME         worktree に移動して Claude Code を起動
#   cw NAME --resume  前回のセッションを再開
#
# 例:
#   cw 321          ringiflow-321 に移動して claude を起動
#   cw main         メインworktree に移動して claude を起動
#   cw 321 --resume 前回セッションを再開
# =============================================================================

cw() {
  local base_dir="${CW_RINGIFLOW_ROOT:-$(ghq root 2>/dev/null)/github.com/ka2kama/ringiflow}"
  local parent_dir="${base_dir:h}"

  if [[ ! -d "$base_dir" ]]; then
    echo "error: ringiflow not found at $base_dir" >&2
    echo "hint: set CW_RINGIFLOW_ROOT to the correct path" >&2
    return 1
  fi

  # 引数なし: worktree 一覧
  if [[ -z "${1:-}" ]]; then
    git -C "$base_dir" worktree list
    return
  fi

  local name="$1"
  shift

  local dir
  if [[ "$name" == "main" ]]; then
    dir="$base_dir"
  else
    dir="${parent_dir}/ringiflow-${name}"
  fi

  if [[ ! -d "$dir" ]]; then
    echo "error: worktree not found: $dir" >&2
    echo "hint: just worktree-add NAME BRANCH" >&2
    return 1
  fi

  cd "$dir" && claude "$@"
}

# zsh 補完: worktree 名を補完候補に表示
_cw() {
  local base_dir="${CW_RINGIFLOW_ROOT:-$(ghq root 2>/dev/null)/github.com/ka2kama/ringiflow}"
  local parent_dir="${base_dir:h}"

  if [[ ! -d "$base_dir" ]]; then
    return
  fi

  local -a worktrees
  worktrees=(main)
  for dir in "$parent_dir"/ringiflow-*(N/); do
    worktrees+=("${dir:t#ringiflow-}")
  done

  _describe 'worktree' worktrees
}
compdef _cw cw
