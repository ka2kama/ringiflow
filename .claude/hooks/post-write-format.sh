#!/bin/bash
# PostToolUse hook: Write/Edit 後にフォーマッタを実行
set -euo pipefail

LOG_FILE="/tmp/claude/post-write-format.log"
mkdir -p /tmp/claude

# stdin から file_path を抽出
file_path=$(cat | jq -r '.tool_input.file_path // empty' 2>/dev/null || echo "")
[[ -z "$file_path" ]] && exit 0

case "$file_path" in
    *.rs)
        just fmt-rust "$file_path" >> "$LOG_FILE" 2>&1 \
            && echo "[$(date '+%H:%M:%S')] fmt-rust: $file_path" >> "$LOG_FILE"
        ;;
    *.elm)
        just fmt-elm "$file_path" >> "$LOG_FILE" 2>&1 \
            && echo "[$(date '+%H:%M:%S')] fmt-elm: $file_path" >> "$LOG_FILE"
        ;;
esac
exit 0
