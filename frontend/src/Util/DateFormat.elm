module Util.DateFormat exposing
    ( formatDate
    , formatDateTime
    , formatMaybeDate
    , formatMaybeDateTime
    )

{-| 日付フォーマットユーティリティ

ISO 8601 日時文字列をタイムゾーン変換し、UI 表示用にフォーマットする。
バックエンドから UTC（RFC 3339）で返される日時を、ブラウザのローカルタイムゾーンに変換して表示する。


## 使用例

    import Time
    import Util.DateFormat as DateFormat

    -- 日付のみ（JST で表示）
    DateFormat.formatDate jst "2026-01-15T10:30:00Z"
    --> "2026-01-15"

    -- 日付と時刻（JST で表示、+9時間）
    DateFormat.formatDateTime jst "2026-01-15T10:30:00Z"
    --> "2026-01-15 19:30"

    -- Maybe 対応
    DateFormat.formatMaybeDate jst Nothing
    --> "-"

-}

import Iso8601
import Maybe.Extra
import Time


{-| ISO 8601 日時文字列からタイムゾーン変換した日付を取得

    formatDate jst "2026-01-15T10:30:00Z" --> "2026-01-15"

-}
formatDate : Time.Zone -> String -> String
formatDate zone isoString =
    case Iso8601.toTime isoString of
        Ok posix ->
            formatPosixDate zone posix

        Err _ ->
            isoString


{-| Maybe な日時文字列からタイムゾーン変換した日付を取得

Nothing の場合は "-" を返す。

-}
formatMaybeDate : Time.Zone -> Maybe String -> String
formatMaybeDate zone maybeDate =
    Maybe.Extra.unwrap "-" (formatDate zone) maybeDate


{-| ISO 8601 日時文字列からタイムゾーン変換した日時を取得

    formatDateTime jst "2026-01-15T10:30:00Z" --> "2026-01-15 19:30"

-}
formatDateTime : Time.Zone -> String -> String
formatDateTime zone isoString =
    case Iso8601.toTime isoString of
        Ok posix ->
            formatPosixDate zone posix
                ++ " "
                ++ formatPosixTime zone posix

        Err _ ->
            isoString


{-| Maybe な日時文字列からタイムゾーン変換した日時を取得

Nothing の場合は "-" を返す。

-}
formatMaybeDateTime : Time.Zone -> Maybe String -> String
formatMaybeDateTime zone maybeDateTime =
    Maybe.Extra.unwrap "-" (formatDateTime zone) maybeDateTime



-- 内部関数


{-| Time.Posix を日付文字列にフォーマット（YYYY-MM-DD）
-}
formatPosixDate : Time.Zone -> Time.Posix -> String
formatPosixDate zone posix =
    let
        year =
            Time.toYear zone posix

        month =
            Time.toMonth zone posix |> monthToNumber

        day =
            Time.toDay zone posix
    in
    String.fromInt year
        ++ "-"
        ++ padZero month
        ++ "-"
        ++ padZero day


{-| Time.Posix を時刻文字列にフォーマット（HH:MM）
-}
formatPosixTime : Time.Zone -> Time.Posix -> String
formatPosixTime zone posix =
    let
        hour =
            Time.toHour zone posix

        minute =
            Time.toMinute zone posix
    in
    padZero hour ++ ":" ++ padZero minute


{-| Time.Month を数値に変換
-}
monthToNumber : Time.Month -> Int
monthToNumber month =
    case month of
        Time.Jan ->
            1

        Time.Feb ->
            2

        Time.Mar ->
            3

        Time.Apr ->
            4

        Time.May ->
            5

        Time.Jun ->
            6

        Time.Jul ->
            7

        Time.Aug ->
            8

        Time.Sep ->
            9

        Time.Oct ->
            10

        Time.Nov ->
            11

        Time.Dec ->
            12


{-| 数値を2桁のゼロパディング文字列に変換
-}
padZero : Int -> String
padZero n =
    if n < 10 then
        "0" ++ String.fromInt n

    else
        String.fromInt n
