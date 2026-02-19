#!/usr/bin/env bash
#
# 改善記録が標準フォーマットに準拠しているかバリデーションする。
#
# チェック内容:
# - 「## 分類」セクションが存在する（エラー）
# - 「- カテゴリ: <有効値>」が存在する（エラー）
# - 「- 失敗タイプ: <有効値>」が存在する（エラー）
# - 「- 問題の性質: <有効値>」が存在する（警告のみ — 2026-02-15 導入のため遡及は別 Issue）
#
# 有効値の定義: process/improvements/README.md
#
# Usage: ./scripts/check/improvement-records.sh

set -euo pipefail

# 定義済みカテゴリ（README.md L152-161）
VALID_CATEGORIES=(
    "参照漏れ"
    "単一パス検証"
    "即座の対策"
    "視点不足"
    "コンテキスト引きずり"
    "知識-実行乖離"
)

# 定義済み失敗タイプ（README.md L167-171）
VALID_FAILURE_TYPES=(
    "知識ギャップ"
    "実行ギャップ"
    "プロセスギャップ"
)

# 定義済み問題の性質（README.md L105-109）
VALID_NATURES=(
    "技術的"
    "プロセス的"
    "思考的"
)

ERRORS=()
WARNINGS=()

# 値が有効リストに含まれるか判定する
is_valid_value() {
    local value="$1"
    shift
    local valid_values=("$@")
    for valid in "${valid_values[@]}"; do
        if [[ "$value" == "$valid" ]]; then
            return 0
        fi
    done
    return 1
}

# 値を抽出する（括弧付き補足を除去）
# 例: "知識-実行乖離（検証の仕組みは...）" → "知識-実行乖離"
extract_value() {
    local line="$1"
    local prefix="$2"
    # プレフィックスを除去し、全角・半角括弧以降を除去、末尾空白を除去
    echo "$line" | sed "s/^${prefix}//" | sed 's/[（(].*//' | sed 's/[[:space:]]*$//'
}

for file in process/improvements/????-??/*.md; do
    # 「## 分類」セクションの存在チェック
    if ! grep -q "^## 分類" "$file"; then
        ERRORS+=("$file: '## 分類' セクションがありません")
        continue
    fi

    # カテゴリのチェック
    category_line=$(grep -m 1 "^- カテゴリ: " "$file" || true)
    if [[ -z "$category_line" ]]; then
        ERRORS+=("$file: '- カテゴリ: ' が標準フォーマットで記載されていません")
    else
        value=$(extract_value "$category_line" "- カテゴリ: ")
        if ! is_valid_value "$value" "${VALID_CATEGORIES[@]}"; then
            ERRORS+=("$file: カテゴリ '${value}' は定義済みカテゴリに含まれません")
        fi
    fi

    # 失敗タイプのチェック
    failure_line=$(grep -m 1 "^- 失敗タイプ: " "$file" || true)
    if [[ -z "$failure_line" ]]; then
        ERRORS+=("$file: '- 失敗タイプ: ' が標準フォーマットで記載されていません")
    else
        value=$(extract_value "$failure_line" "- 失敗タイプ: ")
        if ! is_valid_value "$value" "${VALID_FAILURE_TYPES[@]}"; then
            ERRORS+=("$file: 失敗タイプ '${value}' は定義済み失敗タイプに含まれません")
        fi
    fi

    # 問題の性質のチェック（警告のみ）
    nature_line=$(grep -m 1 "^- 問題の性質: " "$file" || true)
    if [[ -z "$nature_line" ]]; then
        WARNINGS+=("$file: '- 問題の性質: ' が未記載です")
    else
        value=$(extract_value "$nature_line" "- 問題の性質: ")
        if ! is_valid_value "$value" "${VALID_NATURES[@]}"; then
            ERRORS+=("$file: 問題の性質 '${value}' は定義済み値に含まれません")
        fi
    fi
done

# 警告の表示
if [[ ${#WARNINGS[@]} -gt 0 ]]; then
    echo "⚠ 以下の改善記録に '- 問題の性質: ' が未記載です（${#WARNINGS[@]} 件）:"
    for warning in "${WARNINGS[@]}"; do
        echo "  - $warning"
    done
fi

# エラーの表示
if [[ ${#ERRORS[@]} -gt 0 ]]; then
    echo "❌ 以下の改善記録が標準フォーマットに準拠していません:"
    for error in "${ERRORS[@]}"; do
        echo "  - $error"
    done
    echo ""
    echo "標準フォーマット:"
    echo "  ## 分類"
    echo "  - カテゴリ: <参照漏れ|単一パス検証|即座の対策|視点不足|コンテキスト引きずり|知識-実行乖離>"
    echo "  - 失敗タイプ: <知識ギャップ|実行ギャップ|プロセスギャップ>"
    echo "  - 問題の性質: <技術的|プロセス的|思考的>"
    echo ""
    echo "詳細: process/improvements/README.md"
    exit 1
fi

echo "✅ すべての改善記録が標準フォーマットに準拠しています"
