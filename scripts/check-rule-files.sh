#!/usr/bin/env bash
#
# .claude/rules/ 内のルールファイルが適切に参照されているかチェックする。
# 各ファイルは以下のいずれかを満たす必要がある:
# - paths: フロントマターを持つ
# - CLAUDE.md から参照されている
#
# Usage: ./scripts/check-rule-files.sh

set -euo pipefail

ERRORS=()

for file in .claude/rules/*.md; do
  filename=$(basename "$file")

  # paths: フロントマターがあるかチェック
  has_paths=false
  if head -10 "$file" | grep -q "^paths:"; then
    has_paths=true
  fi

  # CLAUDE.md から参照されているかチェック
  is_referenced=false
  if grep -q "$filename" CLAUDE.md; then
    is_referenced=true
  fi

  if [ "$has_paths" = false ] && [ "$is_referenced" = false ]; then
    ERRORS+=("$file: paths: フロントマターがなく、CLAUDE.md からも参照されていません")
  fi
done

if [ ${#ERRORS[@]} -gt 0 ]; then
  echo "❌ 以下のルールファイルが適切に参照されていません:"
  for error in "${ERRORS[@]}"; do
    echo "  - $error"
  done
  echo ""
  echo "各ルールファイルは以下のいずれかを満たす必要があります:"
  echo "  1. paths: フロントマターを持つ（特定のファイルパターンに対してルールを適用）"
  echo "  2. CLAUDE.md から参照されている（常に読み込まれるルール）"
  exit 1
fi

echo "✅ すべてのルールファイルが適切に参照されています"
