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
# git status --porcelain の出力形式: XY filename
#   X = ステージング領域の状態、Y = ワーキングツリーの状態
# ステージ済み（X のみ変更、Y が空白）はコミットに含まれるので除外する。
# 検出対象:
#   ?? = untracked（未追跡）
#   Y が非空白 = ワーキングツリーに変更あり（例: " M", "AM"）
all_status=$(git status --porcelain -- "prompts/plans/" 2>/dev/null || true)

if [ -z "$all_status" ]; then
    exit 0
fi

# ステージ済みのみ（Y が空白）の行を除外
# XY の Y（2文字目）が空白でない行、または ?? の行を抽出
uncommitted=$(echo "$all_status" | grep -E '^(\?\?|.[^ ])' || true)

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
