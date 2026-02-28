#!/usr/bin/env bash
set -euo pipefail

# Rules Check の各グループ結果と PR コメントのマーカーから
# 最終ステータスを決定し、GITHUB_OUTPUT に書き出す。
#
# 入力（環境変数）:
#   HAS_MATCHING_RULES: true | false
#   GROUP1_RESULT: rules-group-1 ジョブの result（success / failure / skipped）
#   GROUP2_RESULT: rules-group-2 ジョブの result
#   PR_NUMBER: PR 番号
#   GH_REPO: リポジトリ名（owner/repo 形式）
# 出力（GITHUB_OUTPUT）:
#   skip: true | false
#   state: success | failure（skip=false の場合のみ）
#   description: ステータスの説明（skip=false の場合のみ）

echo "has_matching_rules=$HAS_MATCHING_RULES"
echo "group1_result=$GROUP1_RESULT"
echo "group2_result=$GROUP2_RESULT"

# マッチルールなし → setup で既に skipped 報告済み
if [[ "$HAS_MATCHING_RULES" == "false" ]]; then
  echo "skip=true" >> "$GITHUB_OUTPUT"
  exit 0
fi

echo "skip=false" >> "$GITHUB_OUTPUT"

# アクションレベルの失敗
if [[ "$GROUP1_RESULT" == "failure" || "$GROUP2_RESULT" == "failure" ]]; then
  echo "state=failure" >> "$GITHUB_OUTPUT"
  echo "description=Rules check job failed" >> "$GITHUB_OUTPUT"
  exit 0
fi

# 成功したグループ数をカウント
GROUPS_RAN=0
[[ "$GROUP1_RESULT" == "success" ]] && GROUPS_RAN=$((GROUPS_RAN + 1))
[[ "$GROUP2_RESULT" == "success" ]] && GROUPS_RAN=$((GROUPS_RAN + 1))

if [[ "$GROUPS_RAN" == "0" ]]; then
  echo "state=success" >> "$GITHUB_OUTPUT"
  echo "description=Rules check completed" >> "$GITHUB_OUTPUT"
  exit 0
fi

# 最新 GROUPS_RAN 件の結果マーカーを確認
HAS_FAIL=$(gh api "repos/${GH_REPO}/issues/${PR_NUMBER}/comments" \
  --jq "[.[] | select(.user.login == \"claude[bot]\" and (.body | test(\"<!-- rules-check-result:(pass|fail) -->\")))] | .[-${GROUPS_RAN}:] | any(.[]; .body | contains(\"<!-- rules-check-result:fail -->\"))")

if [[ "$HAS_FAIL" == "true" ]]; then
  echo "state=failure" >> "$GITHUB_OUTPUT"
  echo "description=Rule violations found" >> "$GITHUB_OUTPUT"
else
  echo "state=success" >> "$GITHUB_OUTPUT"
  echo "description=Rules check completed" >> "$GITHUB_OUTPUT"
fi
