#!/usr/bin/env bash
#
# 計画ファイルの「確認事項」セクションに未チェックのチェックボックスがないか検証する。
#
# main ブランチとの差分で変更された計画ファイル（prompts/plans/*.md）のみ対象。
# 「#### 確認事項」セクション内の `- [ ]` を検出してエラーにする。
#
# Usage: ./scripts/check/plan-confirmations.sh

set -euo pipefail

# main ブランチが存在しない場合（初回クローン等）はスキップ
if ! git rev-parse --verify main >/dev/null 2>&1 &&
   ! git rev-parse --verify origin/main >/dev/null 2>&1; then
    echo "✅ main ブランチが見つかりません（スキップ）"
    exit 0
fi

# main ブランチ上にいる場合はスキップ
current_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "HEAD")
if [[ "$current_branch" == "main" ]]; then
    echo "✅ main ブランチのため計画ファイルチェックをスキップ"
    exit 0
fi

# base ref を決定（ローカル main → origin/main のフォールバック）
base_ref=$(git rev-parse --verify main 2>/dev/null || git rev-parse --verify origin/main)

# main との差分で変更された計画ファイル（README.md を除外）
changed_plans=$(git diff --name-only "$base_ref"...HEAD -- 'prompts/plans/*.md' 2>/dev/null | grep -v 'README.md' || true)

if [ -z "$changed_plans" ]; then
    echo "✅ 変更された計画ファイルなし"
    exit 0
fi

errors=()

for file in $changed_plans; do
    # ファイルが存在しない場合（削除された場合）はスキップ
    [ -f "$file" ] || continue

    in_section=false
    line_num=0

    while IFS= read -r line; do
        line_num=$((line_num + 1))

        # 「#### 確認事項」セクション開始
        if [[ "$line" =~ ^####[[:space:]]確認事項 ]]; then
            in_section=true
            continue
        fi

        # 次のセクションヘッダーで終了（#, ##, ###, ####）
        if $in_section && [[ "$line" =~ ^#{1,4}[[:space:]] ]]; then
            in_section=false
        fi

        # 確認事項セクション内の未チェックボックスを検出
        if $in_section && [[ "$line" =~ ^-[[:space:]]\[[[:space:]]\] ]]; then
            # 行の先頭の "- [ ] " を除去して内容を取得
            content="${line#- \[ \] }"
            errors+=("$file:$line_num: $content")
        fi
    done < "$file"
done

if [ ${#errors[@]} -gt 0 ]; then
    echo "❌ 計画ファイルに未チェックの確認事項があります（${#errors[@]} 件）:"
    echo ""
    for error in "${errors[@]}"; do
        echo "  - $error"
    done
    echo ""
    echo "対応方法:"
    echo "  1. 各 Phase の確認事項を Read/Grep で実施する"
    echo "  2. チェックボックスを [x] に更新する"
    echo "  3. 確認結果を1行で追記する（ファイルパス:行番号、具体的な内容）"
    echo ""
    echo "詳細: .claude/rules/pre-implementation.md"
    exit 1
fi

echo "✅ 計画ファイルの確認事項はすべてチェック済みです"
