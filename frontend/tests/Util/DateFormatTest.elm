module Util.DateFormatTest exposing (suite)

{-| Util.DateFormat モジュールのテスト

日付フォーマット関数の正確性を検証する。
タイムゾーン変換が正しく行われることを確認する。

-}

import Expect
import Test exposing (..)
import Time
import Util.DateFormat as DateFormat



-- テスト用タイムゾーン


{-| JST (+09:00)
-}
jst : Time.Zone
jst =
    Time.customZone (9 * 60) []


{-| UTC
-}
utc : Time.Zone
utc =
    Time.utc



-- suite


suite : Test
suite =
    describe "Util.DateFormat"
        [ formatDateTests
        , formatMaybeDateTests
        , formatDateTimeTests
        , formatMaybeDateTimeTests
        ]



-- formatDate


formatDateTests : Test
formatDateTests =
    describe "formatDate"
        [ test "UTC の日時を JST の日付に変換する" <|
            \_ ->
                DateFormat.formatDate jst "2026-01-15T10:30:00Z"
                    |> Expect.equal "2026-01-15"
        , test "UTC の深夜をまたぐ時刻を JST で翌日の日付に変換する" <|
            \_ ->
                -- UTC 23:30 → JST 翌日 08:30
                DateFormat.formatDate jst "2026-01-15T23:30:00Z"
                    |> Expect.equal "2026-01-16"
        , test "UTC タイムゾーンでは日付がそのまま返る" <|
            \_ ->
                DateFormat.formatDate utc "2026-01-15T10:30:00Z"
                    |> Expect.equal "2026-01-15"
        , test "パース失敗時は元の文字列を返す" <|
            \_ ->
                DateFormat.formatDate jst "invalid"
                    |> Expect.equal "invalid"
        ]



-- formatMaybeDate


formatMaybeDateTests : Test
formatMaybeDateTests =
    describe "formatMaybeDate"
        [ test "Just の場合はタイムゾーン変換した日付を返す" <|
            \_ ->
                DateFormat.formatMaybeDate jst (Just "2026-01-15T10:30:00Z")
                    |> Expect.equal "2026-01-15"
        , test "Nothing の場合は \"-\" を返す" <|
            \_ ->
                DateFormat.formatMaybeDate jst Nothing
                    |> Expect.equal "-"
        ]



-- formatDateTime


formatDateTimeTests : Test
formatDateTimeTests =
    describe "formatDateTime"
        [ test "UTC の日時を JST の日時に変換する" <|
            \_ ->
                DateFormat.formatDateTime jst "2026-01-15T10:30:00Z"
                    |> Expect.equal "2026-01-15 19:30"
        , test "UTC の深夜をまたぐ時刻を JST で翌日に変換する" <|
            \_ ->
                -- UTC 23:30 → JST 翌日 08:30
                DateFormat.formatDateTime jst "2026-01-15T23:30:00Z"
                    |> Expect.equal "2026-01-16 08:30"
        , test "UTC タイムゾーンでは時刻がそのまま返る" <|
            \_ ->
                DateFormat.formatDateTime utc "2026-01-15T10:30:00Z"
                    |> Expect.equal "2026-01-15 10:30"
        , test "パース失敗時は元の文字列を返す" <|
            \_ ->
                DateFormat.formatDateTime jst "invalid"
                    |> Expect.equal "invalid"
        ]



-- formatMaybeDateTime


formatMaybeDateTimeTests : Test
formatMaybeDateTimeTests =
    describe "formatMaybeDateTime"
        [ test "Just の場合はタイムゾーン変換した日時を返す" <|
            \_ ->
                DateFormat.formatMaybeDateTime jst (Just "2026-01-15T10:30:00Z")
                    |> Expect.equal "2026-01-15 19:30"
        , test "Nothing の場合は \"-\" を返す" <|
            \_ ->
                DateFormat.formatMaybeDateTime jst Nothing
                    |> Expect.equal "-"
        ]
