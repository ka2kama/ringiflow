#!/usr/bin/env bash
# 実装中の軽量チェック（リント、テスト、統合テスト、ビルド、SQLx キャッシュ同期、OpenAPI 同期、構造品質）
# Rust レーンと Non-Rust レーンを並列実行して高速化
#
# Usage:
#   ./scripts/check-parallel.sh           # 全チェック（just check）
#   ./scripts/check-parallel.sh --skip-db  # DB 不要のチェックのみ（just check-pre-push）
set -uo pipefail

skip_db=false
if [ "${1:-}" = "--skip-db" ]; then
    skip_db=true
fi

non_rust_log=$(mktemp)
trap 'rm -f "$non_rust_log"' EXIT

# Non-Rust レーン（バックグラウンド）
(
    set -e
    just lint-elm
    just test-elm
    just build-elm
    just lint-shell
    just lint-ci
    just lint-openapi
    just lint-improvements
    just lint-rules
    just check-doc-links
    just check-impl-docs
    just check-unused-deps
    just check-file-size
    just check-duplicates
) > "$non_rust_log" 2>&1 &
non_rust_pid=$!

# Rust レーン（フォアグラウンド）
rust_ok=true
if $skip_db; then
    # DB 不要のチェックのみ（pre-push 用）
    just lint-rust && \
    just test-rust && \
    just openapi-check || rust_ok=false
else
    # 全チェック（just check 用）
    just lint-rust && \
    just test-rust && \
    just test-rust-integration && \
    just sqlx-check && \
    just schema-check && \
    just openapi-check || rust_ok=false
fi

# Non-Rust レーンの完了待ち
non_rust_ok=true
wait $non_rust_pid || non_rust_ok=false

echo ""
echo "=== Non-Rust チェック ==="
cat "$non_rust_log"

# 結果判定
if ! $rust_ok || ! $non_rust_ok; then
    echo ""
    ! $rust_ok && echo "✗ Rust レーン: 失敗"
    ! $non_rust_ok && echo "✗ Non-Rust レーン: 失敗"
    exit 1
fi
echo ""
echo "✓ 全チェック完了"
