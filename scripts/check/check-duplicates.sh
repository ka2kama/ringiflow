#!/usr/bin/env bash
#
# コード重複率がベースラインを超えていないかチェックする。
# jscpd を使用し、baselines.env の閾値で判定する。
#
# 超過時は exit 1 で CI を失敗させる（ラチェット方式）。
#
# Usage: ./scripts/check/check-duplicates.sh
# 参照: [ADR-042](../../docs/05_ADR/042_コピペ検出ツールの選定.md)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"

# shellcheck disable=SC1091
source "$PROJECT_ROOT/.config/baselines.env"

echo "=== Rust コード重複チェック (threshold: ${JSCPD_RUST_THRESHOLD}%) ==="
pnpm exec jscpd --min-lines 10 --min-tokens 50 --format "rust" --gitignore \
    --threshold "$JSCPD_RUST_THRESHOLD" backend/

echo ""
echo "=== Elm コード重複チェック (threshold: ${JSCPD_ELM_THRESHOLD}%) ==="
pnpm exec jscpd --min-lines 10 --min-tokens 50 --format "haskell" --formats-exts "haskell:elm" \
    --gitignore --threshold "$JSCPD_ELM_THRESHOLD" frontend/src/
