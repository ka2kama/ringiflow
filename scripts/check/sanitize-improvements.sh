#!/usr/bin/env bash
#
# 改善記録のサニタイズ違反を検出する。
#
# ユーザー発言の直接引用（カギ括弧 「」『』 による引用）を検出してエラーにする。
# コードブロック（```）内の行は除外する。
#
# Usage:
#   ./scripts/check/sanitize-improvements.sh              # 全ファイル
#   ./scripts/check/sanitize-improvements.sh file1 file2  # 指定ファイルのみ（lefthook pre-commit 用）

set -euo pipefail

# --- 検出パターン（ERE） ---
# Pattern 1: ユーザー帰属 + カギ括弧引用
PATTERN_1='ユーザー[がのはから].*[「『][^」』]+[」』]'
# Pattern 2: カギ括弧引用 + 発話動詞
PATTERN_2='[「『][^」』]+[」』]と(言っ|述べ|指摘し|発言し|要求し|依頼し|質問し|確認し|聞い|尋ね|答え|返し|主張し)'
# Pattern 3: カギ括弧引用 + 帰属名詞
PATTERN_3='[「『][^」』]+[」』]という(指摘|発言|要求|依頼|質問|意見|フィードバック|コメント)'

COMBINED_PATTERN="($PATTERN_1)|($PATTERN_2)|($PATTERN_3)"

# --- ファイル一覧取得 ---
files=()
if [ $# -gt 0 ]; then
    files=("$@")
else
    while IFS= read -r f; do
        files+=("$f")
    done < <(git -c core.quotepath=false ls-files --cached "process/improvements/????-??/*.md")
fi

# --- チェック ---
errors=()

for file in "${files[@]}"; do
    # 存在チェック
    [ -f "$file" ] || continue
    # process/improvements/ 配下のみ対象
    [[ "$file" == process/improvements/* ]] || continue
    # Markdown のみ
    [[ "$file" == *.md ]] || continue
    # README.md はスキップ
    [[ "$(basename "$file")" == "README.md" ]] && continue

    # コードブロック外の行のみ抽出（行番号付き）
    outside=$(awk '/^```/{skip=!skip; next} !skip{print NR": "$0}' "$file")
    [ -z "$outside" ] && continue

    # パターンマッチ
    while IFS= read -r match; do
        [ -z "$match" ] && continue
        errors+=("$file:$match")
    done < <(printf '%s\n' "$outside" | grep -E "$COMBINED_PATTERN" || true)
done

# --- 結果出力 ---
if [ ${#errors[@]} -gt 0 ]; then
    echo "❌ 改善記録にサニタイズ違反の兆候があります（${#errors[@]} 件）:"
    echo ""
    for error in "${errors[@]}"; do
        echo "  $error"
    done
    echo ""
    echo "改善記録ではユーザーの発言を直接引用（「」『』）せず、技術的内容に要約してください。"
    echo "  例: ユーザーから「設定確認した？」と指摘 → ユーザーから設定確認の不足を指摘された"
    echo ""
    echo "詳細: process/improvements/README.md > サニタイズルール"
    exit 1
fi

echo "✅ 改善記録のサニタイズチェック完了"
