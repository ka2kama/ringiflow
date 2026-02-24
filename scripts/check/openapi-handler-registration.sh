#!/usr/bin/env bash
#
# OpenAPI ハンドラ登録照合
#
# #[utoipa::path] アノテーションが付いたハンドラ関数が
# openapi.rs の paths() マクロに登録されているか検証する。
#
# 検出: アノテーション付きだが未登録の関数
# 対応: openapi.rs の paths() に登録を追加するか、
#       意図的に除外する場合は #[utoipa::path] を削除する
#
# Usage: ./scripts/check/openapi-handler-registration.sh

set -euo pipefail

HANDLER_DIR="backend/apps/bff/src/handler"
OPENAPI_FILE="backend/apps/bff/src/openapi.rs"

# --- openapi.rs の paths() セクションから登録済みエントリを抽出 ---
registered=()
in_paths=false
while IFS= read -r line; do
    # paths( で開始
    if [[ "$line" =~ paths\( ]]; then
        in_paths=true
        continue
    fi

    if $in_paths; then
        # 閉じ括弧またはセクション切り替えで終了
        if [[ "$line" =~ ^[[:space:]]*\) ]] || [[ "$line" =~ components\( ]]; then
            in_paths=false
            continue
        fi
        # コメント行をスキップ
        if [[ "$line" =~ ^[[:space:]]*//.* ]]; then
            continue
        fi
        # "   module::function," → "module::function"
        trimmed=$(echo "$line" | sed 's/^[[:space:]]*//' | sed 's/,[[:space:]]*$//' | sed 's/[[:space:]]*$//')
        if [ -n "$trimmed" ]; then
            registered+=("$trimmed")
        fi
    fi
done < "$OPENAPI_FILE"

# --- ハンドラファイルから #[utoipa::path] 付き関数を抽出 ---
expected=()
while IFS= read -r file; do
    # モジュール名を決定
    relative="${file#"$HANDLER_DIR"/}"
    if [[ "$relative" == */* ]]; then
        # サブディレクトリ: "auth/login.rs" → "auth"
        module="${relative%%/*}"
    else
        # 単一ファイル: "user.rs" → "user"
        module="${relative%.rs}"
    fi

    # #[utoipa::path] の後に来る pub async fn <name> を抽出
    found_utoipa=false
    while IFS= read -r line; do
        if [[ "$line" =~ \#\[utoipa::path ]]; then
            found_utoipa=true
        fi
        if $found_utoipa && [[ "$line" =~ pub[[:space:]]+async[[:space:]]+fn[[:space:]]+([a-z_][a-z0-9_]*) ]]; then
            fn_name="${BASH_REMATCH[1]}"
            expected+=("${module}::${fn_name}")
            found_utoipa=false
        fi
    done < "$file"
done < <(find "$HANDLER_DIR" -name "*.rs" -not -name "mod.rs" | sort)

# --- 照合: expected にあって registered にないものを検出 ---
errors=()
for entry in "${expected[@]}"; do
    found=false
    for reg in "${registered[@]}"; do
        if [[ "$reg" == "$entry" ]]; then
            found=true
            break
        fi
    done
    if ! $found; then
        errors+=("$entry")
    fi
done

# --- 結果出力 ---
if [ ${#errors[@]} -gt 0 ]; then
    echo "❌ 以下のハンドラ関数が openapi.rs の paths() に未登録です（${#errors[@]} 件）:"
    echo ""
    for error in "${errors[@]}"; do
        echo "  - $error"
    done
    echo ""
    echo "対応方法:"
    echo "  1. $OPENAPI_FILE の paths() に登録を追加する"
    echo "  2. 意図的に除外する場合は、関数から #[utoipa::path] を削除する"
    echo ""
    echo "詳細: #[utoipa::path] が付いた関数はすべて OpenAPI 仕様に含まれる必要があります"
    exit 1
fi

echo "✅ すべてのハンドラ関数が openapi.rs に登録されています（${#expected[@]} 件）"
