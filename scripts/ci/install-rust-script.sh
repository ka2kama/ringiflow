#!/usr/bin/env bash
set -euo pipefail

# rust-script がインストール済みでなければインストールする。
# CI で sccache キャッシュと併用するため、呼び出し側で
# SCCACHE_GHA_ENABLED / RUSTC_WRAPPER を設定すること。

if ! command -v rust-script &> /dev/null; then
  cargo install rust-script
fi
