#!/usr/bin/env bash
set -euo pipefail

# match-rules.rs --groups 2 の出力をグループ別に分割し、GITHUB_OUTPUT に書き出す。
#
# 入力:
#   $1: ルール出力ファイルパス（match-rules.rs の標準出力をリダイレクトしたもの）
# 出力（GITHUB_OUTPUT）:
#   has_matching_rules: true | false
#   RULES_GROUP_1: グループ1のルール内容
#   RULES_GROUP_2: グループ2のルール内容

RULES_OUTPUT="${1:?使い方: split-rule-groups.sh <rules-output-file>}"

# マッチルールの有無を判定（グループマーカーなしの no-matching-rules は全体が空）
if head -1 "$RULES_OUTPUT" | grep -q '^<!-- no-matching-rules -->$'; then
  echo "has_matching_rules=false" >> "$GITHUB_OUTPUT"
  {
    echo "RULES_GROUP_1<<EOF_GROUP_1"
    echo "<!-- no-matching-rules -->"
    echo "EOF_GROUP_1"
    echo "RULES_GROUP_2<<EOF_GROUP_2"
    echo "<!-- no-matching-rules -->"
    echo "EOF_GROUP_2"
  } >> "$GITHUB_OUTPUT"
else
  echo "has_matching_rules=true" >> "$GITHUB_OUTPUT"
  # グループマーカーで分割
  {
    echo "RULES_GROUP_1<<EOF_GROUP_1"
    awk '/^<!-- group:1 -->$/{p=1;next}/^<!-- group:2 -->$/{p=0}p' "$RULES_OUTPUT"
    echo "EOF_GROUP_1"
    echo "RULES_GROUP_2<<EOF_GROUP_2"
    awk '/^<!-- group:2 -->$/{p=1;next}p' "$RULES_OUTPUT"
    echo "EOF_GROUP_2"
  } >> "$GITHUB_OUTPUT"
fi
