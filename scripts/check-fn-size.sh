#!/usr/bin/env bash
#
# 関数の行数が閾値を超えていないかチェックする。
# clippy::too_many_lines を使用。閾値は backend/clippy.toml で設定。
# テストファイル（tests/ 配下）は除外する。
#
# 警告のみ（exit 0）: CI をブロックしない。肥大化の可視化が目的。
#
# Usage: ./scripts/check-fn-size.sh [clippy options...]
# 例: ./scripts/check-fn-size.sh --quiet

set -euo pipefail

# clippy 出力から "warning + -->" のペアを抽出し、テストファイルを除外
output=$(cd backend && cargo clippy "$@" --all-targets --all-features -- -W clippy::too_many_lines 2>&1 \
    | grep -E "(-->|this function has too many lines)" \
    | paste - - \
    | grep -v "/tests/" \
    | sed 's/\t/\n/') || true

if [ -n "$output" ]; then
    echo "⚠ 50 行を超える関数（分割を検討してください）:"
    echo "$output"
else
    echo "✓ 50 行を超える関数はありません"
fi
