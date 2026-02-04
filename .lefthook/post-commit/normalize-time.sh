#!/bin/bash
# コミット時刻を夜の時間帯（21:00-23:59）に正規化する
#
# 目的: Public リポジトリでの生活パターン推測を抑制する
# 方法: コミットの日付はそのまま、時刻を 21:00-23:59 の範囲に正規化
#       同日の連続コミットは前のコミットからの単調増加を保証する

# 現在のコミットの日付とタイムゾーンを取得
DATE_PART=$(git log -1 --format='%ad' --date=format:'%Y-%m-%d')
TIMEZONE=$(git log -1 --format='%ad' --date=format:'%z')

# 前のコミット（HEAD~1）の情報を取得
PREV_DATE=$(git log -1 --skip=1 --format='%ad' --date=format:'%Y-%m-%d' 2>/dev/null)
PREV_TIME=$(git log -1 --skip=1 --format='%ad' --date=format:'%H:%M:%S' 2>/dev/null)

# 前のコミットが同日かつ 21:00 以降なら、そこから単調増加
if [ "$PREV_DATE" = "$DATE_PART" ] && [ -n "$PREV_TIME" ]; then
    PREV_H=$(echo "$PREV_TIME" | cut -d: -f1)
    PREV_M=$(echo "$PREV_TIME" | cut -d: -f2)
    PREV_S=$(echo "$PREV_TIME" | cut -d: -f3)
    # 10# で基数を明示（08, 09 等の先頭ゼロを8進数として解釈させない）
    # :-0 は防御的デフォルト（外側の -n ガードで空文字列は到達しないが念のため）
    PREV_SECONDS=$((10#${PREV_H:-0} * 3600 + 10#${PREV_M:-0} * 60 + 10#${PREV_S:-0}))

    if [ $PREV_SECONDS -ge 75600 ]; then  # 75600 = 21:00:00
        # 前のコミットから 1〜15 分後（60-900秒）
        INCREMENT=$((60 + RANDOM % 841))
        NEW_SECONDS=$((PREV_SECONDS + INCREMENT))

        # 23:59:59（86399秒）を超えたら丸める
        if [ $NEW_SECONDS -gt 86399 ]; then
            NEW_SECONDS=86399
        fi

        HOUR=$((NEW_SECONDS / 3600))
        MINUTE=$(((NEW_SECONDS % 3600) / 60))
        SECOND=$((NEW_SECONDS % 60))
    else
        # 前のコミットが 21:00 より前（hook 導入前等）→ 21:00-21:29 で新規開始
        # 後続コミットの単調増加に余地を残すため、範囲を前半に限定する
        HOUR=21
        MINUTE=$((RANDOM % 30))
        SECOND=$((RANDOM % 60))
    fi
else
    # 前のコミットが別の日 or 存在しない → 21:00-21:29 で新規開始
    # 後続コミットの単調増加に余地を残すため、範囲を前半に限定する
    HOUR=21
    MINUTE=$((RANDOM % 30))
    SECOND=$((RANDOM % 60))
fi

NEW_DATE="${DATE_PART} $(printf '%02d:%02d:%02d' $HOUR $MINUTE $SECOND) ${TIMEZONE}"

# LEFTHOOK=0 でフックの再実行を防止して amend
LEFTHOOK=0 GIT_COMMITTER_DATE="$NEW_DATE" git commit --amend --allow-empty --no-edit --date="$NEW_DATE"
