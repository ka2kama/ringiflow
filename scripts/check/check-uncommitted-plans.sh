#!/usr/bin/env bash
#
# 計画ファイル（prompts/plans/）の未コミット変更を検出する。
#
# PR ブランチで unstaged/untracked の計画ファイルがある場合に警告を出す。
# 警告のみ（exit 0）で、コミットはブロックしない。
#
# Usage:
#   ./scripts/check/check-uncommitted-plans.sh

set -euo pipefail

# --- ブランチ判定 ---
BRANCH=$(git symbolic-ref --short HEAD 2>/dev/null || echo "")

# main または detached HEAD ではスキップ
if [ -z "$BRANCH" ] || [ "$BRANCH" = "main" ]; then
    exit 0
fi

# --- 未コミット変更の検出 ---
# git status --porcelain で prompts/plans/ 配下の変更を検出
# ?? = untracked, M = modified, A = added(unstaged), etc.
uncommitted=$(git status --porcelain -- "prompts/plans/" 2>/dev/null || true)

if [ -z "$uncommitted" ]; then
    exit 0
fi

# --- 警告出力 ---
echo ""
echo "⚠️  計画ファイルに未コミットの変更があります:"
echo ""
echo "$uncommitted" | while IFS= read -r line; do
    echo "  $line"
done
echo ""
echo "PR マージ前にコミットを忘れないようにしてください。"
echo ""
