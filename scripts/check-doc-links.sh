#!/usr/bin/env bash
#
# ドキュメント内の相対パスリンク切れをチェックする。
# Markdown の [text](path) 形式のリンクを抽出し、リンク先が存在するか検証する。
#
# 対象: docs/, prompts/, .claude/, CLAUDE.md
# スキップ: HTTP(S) リンク、アンカーのみ（#section）、テンプレート/プレースホルダー、コードブロック内
#
# Usage: ./scripts/check-doc-links.sh

set -euo pipefail

ERRORS=()

# コードブロック（``` で囲まれた範囲）を除外して Markdown リンクを抽出する。
# コードブロック内のリンクは、別ファイルへの差分や Rust doc comment 例示など
# 実際のナビゲーションに使われないため、チェック対象外とする。
extract_links_outside_codeblocks() {
    local file="$1"
    awk '
        /^[[:space:]]*```/ { in_code = !in_code; next }
        !in_code { print }
    ' "$file" | grep -oP '\]\(\K[^)]+(?=\))' 2>/dev/null
}

# 対象ファイルを収集
mapfile -t files < <(find docs prompts .claude CLAUDE.md -name '*.md' -type f 2>/dev/null | sort)

for file in "${files[@]}"; do
    dir=$(dirname "$file")

    # Markdown リンクを抽出: [text](path) 形式（コードブロック外のみ）
    while IFS= read -r link; do
        # アンカーを除去してパス部分のみ取得
        path="${link%%#*}"

        # スキップ条件
        [ -z "$path" ] && continue                          # アンカーのみ
        [[ "$path" =~ ^https?:// ]] && continue             # HTTP(S)
        [[ "$path" == "..." ]] && continue                  # 省略記号（テンプレート例）
        [[ "$path" == *"<"*">"* ]] && continue              # プレースホルダー（<NN> 等）
        [[ "$path" == *"{"*"}"* ]] && continue              # テンプレート変数（{url} 等）
        [[ "$path" == *"XXX"* ]] && continue                # テンプレートの仮ファイル名
        [[ "$path" == *"xxx"* ]] && continue                # 例示用パス（小文字）
        [[ "$path" == "relative-link" ]] && continue        # スキル定義の例示リンク
        [[ "$path" == "path" ]] && continue                 # 例示用パス

        # パス解決: 先頭が / ならリポジトリルートから、それ以外はファイルの相対パス
        if [[ "$path" == /* ]]; then
            resolved=".${path}"
        else
            resolved="${dir}/${path}"
        fi

        # ファイルまたはディレクトリが存在するか
        if [ ! -e "$resolved" ]; then
            ERRORS+=("$file: リンク切れ → $link")
        fi
    done < <(extract_links_outside_codeblocks "$file")
done

if [ ${#ERRORS[@]} -gt 0 ]; then
    echo "⚠️  リンク切れが見つかりました (${#ERRORS[@]} 件):"
    for error in "${ERRORS[@]}"; do
        echo "  - $error"
    done
    exit 1
fi

echo "✅ すべてのドキュメントリンクが有効です"
