#!/bin/bash
# git commit 前にステージされたファイルに応じて lint/test を実行

set -e

staged_files=$(git diff --cached --name-only)
has_rust=false
has_elm=false

for file in $staged_files; do
    [[ "$file" =~ \.rs$ || "$file" =~ Cargo\.toml$ ]] && has_rust=true
    [[ "$file" =~ \.elm$ ]] && has_elm=true
done

if [ "$has_rust" = true ]; then
    echo "🦀 Rust: lint-rust test-rust"
    just lint-rust && just test-rust
fi

if [ "$has_elm" = true ]; then
    echo "🌳 Elm: lint-elm test-elm"
    just lint-elm && just test-elm
fi
