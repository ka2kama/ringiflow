#!/usr/bin/env bash
set -euo pipefail

# Auto Review の verification / validation 結果と claude[bot] のレビュー状態から
# 最終ステータスを決定し、GITHUB_OUTPUT に書き出す。
#
# 入力（環境変数）:
#   VERIFICATION_RESULT: verification ジョブの result（success / failure / skipped）
#   VALIDATION_RESULT: validation ジョブの result
#   REVIEW_STATE: claude[bot] の最新レビュー状態（APPROVED / CHANGES_REQUESTED / NONE）
# 出力（GITHUB_OUTPUT）:
#   state: success | failure
#   description: ステータスの説明

echo "Verification result: $VERIFICATION_RESULT"
echo "Validation result: $VALIDATION_RESULT"
echo "Review state: $REVIEW_STATE"

if [[ "$VERIFICATION_RESULT" == "failure" || "$VALIDATION_RESULT" == "failure" ]]; then
  echo "state=failure" >> "$GITHUB_OUTPUT"
  echo "description=Review job failed" >> "$GITHUB_OUTPUT"
elif [[ "$REVIEW_STATE" == "CHANGES_REQUESTED" ]]; then
  echo "state=failure" >> "$GITHUB_OUTPUT"
  echo "description=Changes requested" >> "$GITHUB_OUTPUT"
else
  echo "state=success" >> "$GITHUB_OUTPUT"
  echo "description=Review completed" >> "$GITHUB_OUTPUT"
fi
