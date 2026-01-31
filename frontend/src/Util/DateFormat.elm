module Util.DateFormat exposing
    ( formatDate
    , formatDateTime
    , formatMaybeDate
    , formatMaybeDateTime
    )

{-| 日付フォーマットユーティリティ

ISO 8601 日時文字列を UI 表示用にフォーマットする純粋関数を提供する。


## 使用例

    import Util.DateFormat as DateFormat

    -- 日付のみ
    DateFormat.formatDate "2026-01-15T10:30:00Z"
    --> "2026-01-15"

    -- 日付と時刻
    DateFormat.formatDateTime "2026-01-15T10:30:00Z"
    --> "2026-01-15 10:30"

    -- Maybe 対応
    DateFormat.formatMaybeDate Nothing
    --> "-"

-}


{-| ISO 8601 日時文字列から日付部分を抽出

    formatDate "2026-01-15T10:30:00Z" --> "2026-01-15"

-}
formatDate : String -> String
formatDate isoString =
    String.left 10 isoString


{-| Maybe な日時文字列から日付部分を抽出

Nothing の場合は "-" を返す。

-}
formatMaybeDate : Maybe String -> String
formatMaybeDate maybeDate =
    case maybeDate of
        Just isoString ->
            formatDate isoString

        Nothing ->
            "-"


{-| ISO 8601 日時文字列から日付と時刻（分まで）を抽出

    formatDateTime "2026-01-15T10:30:00Z" --> "2026-01-15 10:30"

-}
formatDateTime : String -> String
formatDateTime isoString =
    String.left 16 isoString
        |> String.replace "T" " "


{-| Maybe な日時文字列から日付と時刻を抽出

Nothing の場合は "-" を返す。

-}
formatMaybeDateTime : Maybe String -> String
formatMaybeDateTime maybeDateTime =
    case maybeDateTime of
        Just dateTime ->
            formatDateTime dateTime

        Nothing ->
            "-"
