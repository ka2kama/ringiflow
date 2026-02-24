#!/usr/bin/env bash
#
# クローズ済み Issue を参照する TODO/FIXME アノテーションを検出する。
#
# TODO(#NNN) / FIXME(#NNN) パターンを検索し、
# 参照先 Issue がクローズ済みの場合にエラーを報告する。
#
# 前提: gh CLI が利用可能であること（CI: GITHUB_TOKEN、ローカル: gh auth login）
#
# Usage: ./scripts/check/stale-annotations.sh

set -euo pipefail

# gh CLI の利用可否チェック
if ! command -v gh &>/dev/null; then
    echo "⚠ gh CLI が見つかりません（stale annotation チェックをスキップ）"
    exit 0
fi

# gh の認証状態チェック
if ! gh auth status &>/dev/null; then
    echo "⚠ gh CLI が未認証です（stale annotation チェックをスキップ）"
    exit 0
fi

# 対象ファイルからアノテーションを抽出
# git ls-files で VCS 管理下のファイルのみ
annotations=()
while IFS= read -r file; do
    while IFS= read -r match; do
        annotations+=("$file:$match")
    done < <(grep -nE '(TODO|FIXME)\(#[0-9]+\)' "$file" 2>/dev/null || true)
done < <(git ls-files --cached "backend/**/*.rs" "frontend/**/*.elm" "scripts/**/*.sh")

# アノテーションがなければ正常終了
if [ ${#annotations[@]} -eq 0 ]; then
    echo "✅ Issue 番号付き TODO/FIXME はありません"
    exit 0
fi

# 一意な Issue 番号を抽出
declare -A issue_numbers
for ann in "${annotations[@]}"; do
    num=$(echo "$ann" | grep -oP '\(#\K[0-9]+' | head -1)
    issue_numbers[$num]=1
done

# 各 Issue の状態を取得
declare -A issue_states
for num in "${!issue_numbers[@]}"; do
    state=$(gh issue view "$num" --json state --jq '.state' 2>/dev/null || echo "NOT_FOUND")
    issue_states[$num]="$state"
done

# クローズ済み/不存在の Issue を参照するアノテーションを報告
errors=()
for ann in "${annotations[@]}"; do
    num=$(echo "$ann" | grep -oP '\(#\K[0-9]+' | head -1)
    state="${issue_states[$num]}"
    if [ "$state" != "OPEN" ]; then
        errors+=("$ann (Issue #$num: $state)")
    fi
done

if [ ${#errors[@]} -gt 0 ]; then
    echo "❌ クローズ済み/不存在の Issue を参照する TODO/FIXME が見つかりました (${#errors[@]} 件):"
    for error in "${errors[@]}"; do
        echo "  - $error"
    done
    exit 1
fi

echo "✅ すべての TODO/FIXME 参照先 Issue が OPEN です (${#annotations[@]} 件確認)"
