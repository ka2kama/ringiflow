#!/usr/bin/env bash
# shellcheck disable=SC2094
#
# 計画ファイルのテスト層網羅確認
#
# 計画ファイルの「テストリスト」セクションで 4 つのテスト層が
# 全て明記されているか検証する。
#   ユニットテスト / ハンドラテスト / API テスト / E2E テスト
# 該当しない層は「該当なし」と記載する。
#
# 免除条件:
#   - ヘッダー行に「該当なし」を含む場合（Phase 全体がテスト不要）
#   - セクション内に「テストリスト: 該当なし」がある場合
#
# main ブランチとの差分で変更された計画ファイルのみ対象。
#
# SC2094 抑制理由: check_section 関数は $file を読み取り専用で使用
# （エラーメッセージにファイルパスを含めるため）
#
# Usage: ./scripts/check/plan-test-layers.sh

set -euo pipefail

# main ブランチが存在しない場合はスキップ
if ! git rev-parse --verify main >/dev/null 2>&1 &&
   ! git rev-parse --verify origin/main >/dev/null 2>&1; then
    echo "✅ main ブランチが見つかりません（スキップ）"
    exit 0
fi

# main ブランチ上にいる場合はスキップ
current_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "HEAD")
if [[ "$current_branch" == "main" ]]; then
    echo "✅ main ブランチのためテスト層チェックをスキップ"
    exit 0
fi

# base ref を決定
base_ref=$(git rev-parse --verify main 2>/dev/null || git rev-parse --verify origin/main)

# main との差分で変更された計画ファイル
changed_plans=$(git diff --name-only "$base_ref"...HEAD -- 'prompts/plans/*.md' 2>/dev/null | grep -v 'README.md' || true)

if [ -z "$changed_plans" ]; then
    echo "✅ 変更された計画ファイルなし"
    exit 0
fi

REQUIRED_LAYERS=("ユニットテスト" "ハンドラテスト" "API テスト" "E2E テスト")
errors=()

# セクション内容を検査する関数
check_section() {
    local file="$1"
    local section_line="$2"
    local content="$3"

    # セクションレベルの免除: 「テストリスト: 該当なし」等
    if echo "$content" | grep -q "テストリスト.*該当なし"; then
        return
    fi

    for layer in "${REQUIRED_LAYERS[@]}"; do
        if ! echo "$content" | grep -q "$layer"; then
            errors+=("$file:$section_line: テスト層「$layer」が未記載")
        fi
    done
}

for file in $changed_plans; do
    [ -f "$file" ] || continue

    in_section=false
    section_line=0
    section_content=""

    line_num=0
    while IFS= read -r line; do
        line_num=$((line_num + 1))

        # 「#### テストリスト」セクション開始
        if [[ "$line" =~ ^####[[:space:]]テストリスト ]]; then
            # 前のセクションがあれば処理
            if $in_section; then
                check_section "$file" "$section_line" "$section_content"
            fi

            # ヘッダー行に「該当なし」があれば免除
            if [[ "$line" =~ 該当なし ]]; then
                in_section=false
                continue
            fi

            in_section=true
            section_line=$line_num
            section_content=""
            continue
        fi

        # 次のセクションヘッダーで現セクション終了
        if $in_section && [[ "$line" =~ ^#{1,4}[[:space:]] ]]; then
            check_section "$file" "$section_line" "$section_content"
            in_section=false
        fi

        # セクション内の内容を蓄積
        if $in_section; then
            section_content+="$line"$'\n'
        fi
    done < "$file"

    # ファイル末尾でセクションが閉じられなかった場合
    if $in_section; then
        check_section "$file" "$section_line" "$section_content"
    fi
done

if [ ${#errors[@]} -gt 0 ]; then
    echo "❌ 計画ファイルのテストリストにテスト層の記載漏れがあります（${#errors[@]} 件）:"
    echo ""
    for error in "${errors[@]}"; do
        echo "  - $error"
    done
    echo ""
    echo "対応方法:"
    echo "  テストリストに以下の 4 層を全て明記してください:"
    echo "    ユニットテスト / ハンドラテスト / API テスト / E2E テスト"
    echo "  該当しない層は「該当なし」と記載してください。"
    echo "  Phase 全体がテスト不要の場合は「テストリスト: 該当なし（理由）」と記載してください。"
    echo ""
    echo "詳細: .claude/rules/zoom-rhythm.md > テストリスト"
    exit 1
fi

echo "✅ 計画ファイルのテストリストは全テスト層が記載されています"
