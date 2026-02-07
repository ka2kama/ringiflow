#!/bin/bash
# Co-Authored-By 行にバージョンとモデル名を注入する
#
# - CLI バージョン: `claude --version` から取得
# - モデル: 固定値
#
# "Co-Authored-By: Claude Code <...>" を
# "Co-Authored-By: Claude Code (v2.1.34, Opus 4.6) <...>" に変換する
#
# すでに括弧付き情報が含まれている場合は何もしない

# モデル ID（固定値）
MODEL_ID="Opus 4.6"

COMMIT_MSG_FILE=$1

# Co-Authored-By 行が存在しない場合は何もしない
if ! grep -q "Co-Authored-By: Claude Code" "$COMMIT_MSG_FILE"; then
    exit 0
fi

# すでにバージョン/モデル情報が含まれている場合はスキップ
if grep -q "Co-Authored-By: Claude Code (.*)" "$COMMIT_MSG_FILE"; then
    exit 0
fi

# CLI バージョンを取得（例: "2.1.34 (Claude Code)" → "2.1.34"）
CLI_VERSION=""
if command -v claude &> /dev/null; then
    CLI_VERSION=$(claude --version 2>/dev/null | sed -n 's/^\([0-9][0-9.]*\).*/\1/p')
fi

# 注入する情報を組み立て
INFO_PARTS=()
[ -n "$CLI_VERSION" ] && INFO_PARTS+=("v${CLI_VERSION}")
[ -n "$MODEL_ID" ] && INFO_PARTS+=("$MODEL_ID")

# 情報がなければ何もしない
if [ ${#INFO_PARTS[@]} -eq 0 ]; then
    exit 0
fi

# カンマ区切りで結合（IFS は最初の1文字しか使わないため printf で結合）
INFO=$(printf '%s, ' "${INFO_PARTS[@]}" | sed 's/, $//')

# Co-Authored-By 行に情報を注入
sed -i "s/Co-Authored-By: Claude Code <\(.*\)>/Co-Authored-By: Claude Code ($INFO) <\1>/" "$COMMIT_MSG_FILE"
