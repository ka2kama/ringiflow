#!/usr/bin/env bash
#
# 実装解説ドキュメントのファイル命名規則をチェックする。
#
# 命名規則（docs/07_実装解説/README.md）:
# - ディレクトリ名: PR<番号>_<機能名>/ (diff モード) または <機能名>/ (feature モード)
# - ファイル名: NN_<トピック>_機能解説.md / NN_<トピック>_コード解説.md
# - 機能解説とコード解説は必ずペアで作成する
#
# Usage: ./scripts/check/impl-docs.sh

set -euo pipefail

BASE_DIR="docs/07_実装解説"
ERRORS=()

# サブディレクトリを走査
for dir in "$BASE_DIR"/*/; do
    dirname=$(basename "$dir")

    # ディレクトリ名チェック: PR<番号>_<機能名> または <機能名> パターン
    # 旧形式（連番プレフィックス）を拒否
    if [[ "$dirname" =~ ^[0-9]+_ ]]; then
        ERRORS+=("旧形式の連番ディレクトリ名です。PR<番号>_<機能名> に変更してください: $dir")
        continue
    fi
    # PR プレフィックスがある場合は PR<数字>_ 形式を検証
    if [[ "$dirname" =~ ^PR ]] && ! [[ "$dirname" =~ ^PR[0-9]+_ ]]; then
        ERRORS+=("ディレクトリ名が PR<番号>_<機能名> パターンに合致しません: $dir")
        continue
    fi

    # ファイル名チェック
    for file in "$dir"*.md; do
        [ -f "$file" ] || continue
        filename=$(basename "$file")

        # NN_<トピック>_{機能解説,コード解説}.md パターン
        if ! [[ "$filename" =~ ^[0-9]{2}_.+_(機能解説|コード解説)\.md$ ]]; then
            ERRORS+=("ファイル名が NN_<トピック>_{機能解説,コード解説}.md パターンに合致しません: $file")
        fi
    done

    # ペアチェック: トピック単位で機能解説とコード解説がペアで存在するか
    # NN_<トピック>_機能解説.md からトピック部分を抽出してペアを確認
    declare -A topics_feature topics_code
    for file in "$dir"*.md; do
        [ -f "$file" ] || continue
        filename=$(basename "$file")

        if [[ "$filename" =~ ^[0-9]{2}_(.+)_機能解説\.md$ ]]; then
            topics_feature["${BASH_REMATCH[1]}"]=1
        elif [[ "$filename" =~ ^[0-9]{2}_(.+)_コード解説\.md$ ]]; then
            topics_code["${BASH_REMATCH[1]}"]=1
        fi
    done

    # 機能解説があるがコード解説がないトピック
    for topic in "${!topics_feature[@]}"; do
        if [ -z "${topics_code[$topic]+_}" ]; then
            ERRORS+=("コード解説が欠如しています: ${dir} トピック「${topic}」に機能解説はあるがコード解説がない")
        fi
    done

    # コード解説があるが機能解説がないトピック
    for topic in "${!topics_code[@]}"; do
        if [ -z "${topics_feature[$topic]+_}" ]; then
            ERRORS+=("機能解説が欠如しています: ${dir} トピック「${topic}」にコード解説はあるが機能解説がない")
        fi
    done

    unset topics_feature topics_code
done

if [ ${#ERRORS[@]} -gt 0 ]; then
    echo "⚠️  実装解説の命名規則違反が見つかりました (${#ERRORS[@]} 件):"
    for error in "${ERRORS[@]}"; do
        echo "  - $error"
    done
    exit 1
fi

echo "✅ 実装解説のファイル命名規則に準拠しています"
