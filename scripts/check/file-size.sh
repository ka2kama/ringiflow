#!/usr/bin/env bash
#
# ソースファイルの行数が閾値を超えていないかチェックする。
# テストファイルは除外する（テストは行数が多くなりやすい性質がある）。
#
# ベースライン: .config/baselines.env の FILE_SIZE_MAX_COUNT
# 超過ファイル数がベースラインを超えたら exit 1（ラチェット方式）。
#
# Usage: ./scripts/check/file-size.sh
# 環境変数: WARN_THRESHOLD（デフォルト: 500）

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"

# shellcheck disable=SC1091
source "$PROJECT_ROOT/.config/baselines.env"

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
elif [ "$found" -gt "$FILE_SIZE_MAX_COUNT" ]; then
   echo ""
   echo "❌ ${WARN_THRESHOLD} 行超ファイル数がベースラインを超えました: ${found} 件（上限: ${FILE_SIZE_MAX_COUNT} 件）"
   exit 1
elif [ "$found" -lt "$FILE_SIZE_MAX_COUNT" ]; then
   echo ""
   echo "💡 ${WARN_THRESHOLD} 行超ファイル数が改善されました: ${found} 件（ベースライン: ${FILE_SIZE_MAX_COUNT} 件）"
   echo "   .config/baselines.env の FILE_SIZE_MAX_COUNT を ${found} に更新してください"
fi
