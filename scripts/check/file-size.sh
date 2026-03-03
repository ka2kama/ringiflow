#!/usr/bin/env bash
#
# ソースファイルの行数が閾値を超えていないかチェックする。
# テストファイルは除外する（テストは行数が多くなりやすい性質がある）。
#
# 例外リスト方式: .config/file-size-exceptions.txt に登録されたファイルは通過。
# 新たに閾値を超えたファイルのみ失敗にする。
#
# Usage:
#   ./scripts/check/file-size.sh                     # 全ファイルチェック（品質ゲート）
#   ./scripts/check/file-size.sh --pre-commit file... # 指定ファイルのみ（警告、exit 0）
#
# 環境変数: WARN_THRESHOLD（デフォルト: 500）

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"

WARN_THRESHOLD="${WARN_THRESHOLD:-500}"
EXCEPTION_FILE="$PROJECT_ROOT/.config/file-size-exceptions.txt"

# --- 例外リストの読み込みとバリデーション ---

if [ ! -f "$EXCEPTION_FILE" ]; then
    echo "❌ 例外リストが見つかりません: $EXCEPTION_FILE"
    exit 1
fi

# 例外リストを解析（パス → 理由のマッピング）
declare -A exceptions=()
validation_errors=()
line_number=0

while IFS= read -r line; do
    line_number=$((line_number + 1))

    # 空行とコメント行をスキップ
    [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue

    # 理由（# 以降）の存在を検証
    if [[ "$line" != *" # "* ]]; then
        validation_errors+=("  行 ${line_number}: 理由がありません: $line")
        continue
    fi

    # パスと理由を分離
    path="${line%% \# *}"
    path="${path%% }"  # 末尾空白を除去
    exceptions["$path"]=1
done < "$EXCEPTION_FILE"

if [ ${#validation_errors[@]} -gt 0 ]; then
    echo "❌ 例外リストにバリデーションエラーがあります（${#validation_errors[@]} 件）:"
    printf '%s\n' "${validation_errors[@]}"
    echo ""
    echo "形式: ファイルパス # 理由"
    exit 1
fi

# --- モード判定 ---

pre_commit=false
files_to_check=()

if [ "${1:-}" = "--pre-commit" ]; then
    pre_commit=true
    shift
    # 引数で渡されたファイルをチェック対象にする
    for file in "$@"; do
        # テストファイルを除外
        case "$file" in
            */tests/*) continue ;;
            *_test.rs) continue ;;
        esac
        files_to_check+=("$file")
    done
else
    # 全ファイルスキャン
    while IFS= read -r file; do
        # テストファイルを除外
        case "$file" in
            */tests/*) continue ;;
            *_test.rs) continue ;;
        esac
        files_to_check+=("$file")
    done < <(git ls-files --cached --others --exclude-standard "backend/**/*.rs" "frontend/src/*.elm" "frontend/src/**/*.elm")
fi

# --- ファイルサイズチェック ---

new_violations=()
known_exceptions=()
stale_exceptions=()

for file in "${files_to_check[@]}"; do
    [ -f "$file" ] || continue

    lines=$(wc -l < "$file")
    if [ "$lines" -gt "$WARN_THRESHOLD" ]; then
        if [[ -v "exceptions[$file]" ]]; then
            known_exceptions+=("$(printf "  %5d 行: %s" "$lines" "$file")")
        else
            new_violations+=("$(printf "  %5d 行: %s" "$lines" "$file")")
        fi
    fi
done

# 陳腐化した例外の検出（全ファイルスキャン時のみ）
if ! $pre_commit; then
    for path in "${!exceptions[@]}"; do
        if [ -f "$path" ]; then
            lines=$(wc -l < "$path")
            if [ "$lines" -le "$WARN_THRESHOLD" ]; then
                stale_exceptions+=("$(printf "  %5d 行: %s（例外リストから削除できます）" "$lines" "$path")")
            fi
        else
            stale_exceptions+=("  ファイルなし: ${path}（例外リストから削除できます）")
        fi
    done
fi

# --- 結果出力 ---

if $pre_commit; then
    # pre-commit モード: 警告のみ（exit 0）
    if [ ${#new_violations[@]} -gt 0 ]; then
        echo "⚠ ${WARN_THRESHOLD} 行を超える新しいファイルがあります（例外リストに未登録）:"
        printf '%s\n' "${new_violations[@]}"
        echo ""
        echo "対応: 分割するか、.config/file-size-exceptions.txt に理由付きで追加してください"
    fi
    exit 0
fi

# 通常モード: 結果を出力
if [ ${#known_exceptions[@]} -gt 0 ]; then
    echo "ℹ ${WARN_THRESHOLD} 行を超えるファイル（例外リスト登録済み: ${#known_exceptions[@]} 件）:"
    printf '%s\n' "${known_exceptions[@]}"
fi

if [ ${#stale_exceptions[@]} -gt 0 ]; then
    echo ""
    echo "💡 例外リストの整理が可能です（${#stale_exceptions[@]} 件）:"
    printf '%s\n' "${stale_exceptions[@]}"
fi

if [ ${#new_violations[@]} -gt 0 ]; then
    echo ""
    echo "❌ ${WARN_THRESHOLD} 行を超える新しいファイルがあります（${#new_violations[@]} 件）:"
    printf '%s\n' "${new_violations[@]}"
    echo ""
    echo "対応: 分割するか、.config/file-size-exceptions.txt に理由付きで追加してください"
    exit 1
fi

if [ ${#known_exceptions[@]} -eq 0 ]; then
    echo "✓ ${WARN_THRESHOLD} 行を超えるファイルはありません"
fi
