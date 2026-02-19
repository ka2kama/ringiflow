#!/usr/bin/env bash
#
# ハンドラとリポジトリ実装に #[tracing::instrument] が付与されているかチェックする。
#
# チェック対象:
# - backend/apps/*/src/handler/**/*.rs の pub async fn（health_check を除く）
# - backend/crates/infra/src/repository/**/*.rs の async fn（impl メソッドのみ、trait 署名は除く）
#
# Usage: ./scripts/check/instrumentation.sh

set -euo pipefail

ERRORS=()

# 除外する関数名
EXCLUDE_FUNCTIONS=("health_check")

# 関数名が除外リストに含まれるか判定
is_excluded() {
    local fn_name="$1"
    for exclude in "${EXCLUDE_FUNCTIONS[@]}"; do
        if [[ "$fn_name" == "$exclude" ]]; then
            return 0
        fi
    done
    return 1
}

# async fn 行から前方スキャンし、impl メソッド（{ で終了）か trait 署名（; で終了）かを判定
# 戻り値: 0 = impl メソッド、1 = trait 署名またはその他
is_impl_method() {
    local file="$1"
    local start_line="$2"
    local total_lines
    total_lines=$(wc -l < "$file")
    local max_line=$((start_line + 20))
    if (( max_line > total_lines )); then
        max_line=$total_lines
    fi

    local i
    for (( i = start_line; i <= max_line; i++ )); do
        local line
        line=$(sed -n "${i}p" "$file")
        # 関数本体の開始（impl メソッド）
        if [[ "$line" =~ \{[[:space:]]*$ ]]; then
            return 0
        fi
        # trait 署名の終了
        if [[ "$line" =~ \;[[:space:]]*$ ]]; then
            return 1
        fi
    done
    return 1
}

# 指定行の上方 N 行以内に #[tracing::instrument が存在するか判定
has_instrument() {
    local file="$1"
    local line_num="$2"
    local lookback=10
    local start=$((line_num - lookback))
    if (( start < 1 )); then
        start=1
    fi

    sed -n "${start},${line_num}p" "$file" | grep -q 'tracing::instrument'
}

# ハンドラチェック: pub async fn に #[tracing::instrument] があるか
check_handlers() {
    local files
    files=$(git ls-files 'backend/apps/*/src/handler/**/*.rs' | grep -v 'tests\.rs$' || true)
    if [[ -z "$files" ]]; then
        return
    fi

    local file
    while IFS= read -r file; do
        while IFS=: read -r line_num _; do
            local fn_name
            fn_name=$(sed -n "${line_num}p" "$file" | sed 's/.*pub async fn \([a-z_][a-z0-9_]*\).*/\1/')

            if is_excluded "$fn_name"; then
                continue
            fi

            if ! has_instrument "$file" "$line_num"; then
                ERRORS+=("${file}:${line_num}: ハンドラ ${fn_name} に #[tracing::instrument] がありません")
            fi
        done < <(grep -n 'pub async fn ' "$file" || true)
    done <<< "$files"
}

# リポジトリ impl チェック: async fn（impl メソッドのみ）に #[tracing::instrument] があるか
check_repository_impls() {
    local files
    files=$(git ls-files 'backend/crates/infra/src/repository/**/*.rs' || true)
    if [[ -z "$files" ]]; then
        return
    fi

    local file
    while IFS= read -r file; do
        while IFS=: read -r line_num _; do
            # trait 署名はスキップ
            if ! is_impl_method "$file" "$line_num"; then
                continue
            fi

            local fn_name
            fn_name=$(sed -n "${line_num}p" "$file" | sed 's/.*async fn \([a-z_][a-z0-9_]*\).*/\1/')

            if is_excluded "$fn_name"; then
                continue
            fi

            if ! has_instrument "$file" "$line_num"; then
                ERRORS+=("${file}:${line_num}: リポジトリ impl ${fn_name} に #[tracing::instrument] がありません")
            fi
        done < <(grep -n 'async fn ' "$file" || true)
    done <<< "$files"
}

check_handlers
check_repository_impls

if [ ${#ERRORS[@]} -gt 0 ]; then
    echo "❌ 計装漏れが見つかりました (${#ERRORS[@]} 件):"
    for error in "${ERRORS[@]}"; do
        echo "  - $error"
    done
    exit 1
fi

echo "✅ すべてのハンドラ・リポジトリに計装が設定されています"
