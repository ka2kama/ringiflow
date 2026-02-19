#!/usr/bin/env bash
#
# ソースファイルの行数が閾値を超えていないかチェックする。
# テストファイルは除外する（テストは行数が多くなりやすい性質がある）。
#
# 警告のみ（exit 0）: CI をブロックしない。肥大化の可視化が目的。
#
# Usage: ./scripts/check/file-size.sh
# 環境変数: WARN_THRESHOLD（デフォルト: 500）

set -euo pipefail

WARN_THRESHOLD="${WARN_THRESHOLD:-500}"
found=0

# git ls-files でトラッキング対象のファイルのみを取得
# テストファイルを除外: tests/ 配下、*_test.rs
while IFS= read -r file; do
   # テストファイルを除外
   case "$file" in
      */tests/*) continue ;;
      *_test.rs) continue ;;
   esac

   lines=$(wc -l < "$file")
   if [ "$lines" -gt "$WARN_THRESHOLD" ]; then
      if [ "$found" -eq 0 ]; then
         echo "⚠ ${WARN_THRESHOLD} 行を超えるファイル（分割を検討してください）:"
      fi
      printf "  %5d 行: %s\n" "$lines" "$file"
      found=$((found + 1))
   fi
done < <(git ls-files --cached --others --exclude-standard "backend/**/*.rs" "frontend/src/**/*.elm")

if [ "$found" -eq 0 ]; then
   echo "✓ ${WARN_THRESHOLD} 行を超えるファイルはありません"
fi
