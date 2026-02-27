#!/usr/bin/env bash
#
# 計画ファイルの命名規則違反を検出する。
#
# ファイル名が数字または README で始まらないファイルをエラーにする。
# 命名規則: {Issue番号}_{トピック}.md または YYYYMMDD_{トピック}.md
#
# Usage:
#   ./scripts/check/check-plan-filenames.sh              # 全ファイル
#   ./scripts/check/check-plan-filenames.sh file1 file2  # 指定ファイルのみ（lefthook pre-commit 用）

set -euo pipefail

# --- ファイル一覧取得 ---
files=()
if [ $# -gt 0 ]; then
    files=("$@")
else
    while IFS= read -r f; do
        files+=("$f")
    done < <(git -c core.quotepath=false ls-files --cached "prompts/plans/*.md")
fi

# --- チェック ---
errors=()

for file in "${files[@]}"; do
    # 存在チェック
    [ -f "$file" ] || continue
    # prompts/plans/ 配下のみ対象
    [[ "$file" == prompts/plans/* ]] || continue
    # Markdown のみ
    [[ "$file" == *.md ]] || continue

    filename=$(basename "$file")

    # README.md はスキップ
    [[ "$filename" == README* ]] && continue

    # 数字で始まるファイルは OK
    [[ "$filename" =~ ^[0-9] ]] && continue

    # それ以外は命名規則違反
    errors+=("$file")
done

# --- 結果出力 ---
if [ ${#errors[@]} -gt 0 ]; then
    echo "❌ 計画ファイルが命名規則に従っていません（${#errors[@]} 件）:"
    echo ""
    for error in "${errors[@]}"; do
        echo "  $error"
    done
    echo ""
    echo "計画ファイルは以下の命名規則に従ってください:"
    echo "  {Issue番号}_{トピック}.md  例: 288_dev-auth-feature-flag.md"
    echo "  YYYYMMDD_{トピック}.md    例: 20260207_plan-files-git-management.md"
    echo ""
    echo "詳細: prompts/plans/README.md"
    exit 1
fi

echo "✅ 計画ファイルの命名規則チェック完了"
