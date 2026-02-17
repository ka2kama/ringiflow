#!/usr/bin/env bash
#
# ドキュメント内の相対パスリンク切れをチェックする。
# Markdown の [text](path) 形式のリンクを抽出し、リンク先が存在するか検証する。
#
# 対象: docs/, prompts/, .claude/, CLAUDE.md
# スキップ: HTTP(S) リンク、アンカーのみ（#section）、テンプレート/プレースホルダー
#
# Usage: ./scripts/check-doc-links.sh

set -euo pipefail

ERRORS=()

# 対象ファイルを収集
mapfile -t files < <(find docs prompts .claude CLAUDE.md -name '*.md' -type f 2>/dev/null | sort)

for file in "${files[@]}"; do
    dir=$(dirname "$file")

    # Markdown リンクを抽出: [text](path) 形式
    while IFS= read -r link; do
        # アンカーを除去してパス部分のみ取得
        path="${link%%#*}"

        # スキップ条件
        [ -z "$path" ] && continue                          # アンカーのみ
        [[ "$path" =~ ^https?:// ]] && continue             # HTTP(S)
        [[ "$path" == "..." ]] && continue                  # 省略記号（テンプレート例）
        [[ "$path" == *"<"*">"* ]] && continue              # プレースホルダー（<NN> 等）
        [[ "$path" == *"{"*"}"* ]] && continue              # テンプレート変数（{url} 等）

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
    done < <(grep -oP '\]\(\K[^)]+(?=\))' "$file" 2>/dev/null)
done

if [ ${#ERRORS[@]} -gt 0 ]; then
    echo "⚠️  リンク切れが見つかりました (${#ERRORS[@]} 件):"
    for error in "${ERRORS[@]}"; do
        echo "  - $error"
    done
    # 警告のみ（exit 0）: 既存のリンク切れが多数あるため、ブロックしない
    # リンク修正後に exit 1 に切り替える
    exit 0
fi

echo "✅ すべてのドキュメントリンクが有効です"
