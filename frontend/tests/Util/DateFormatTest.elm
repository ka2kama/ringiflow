module Util.DateFormatTest exposing (suite)

{-| Util.DateFormat モジュールのテスト

日付フォーマット関数の正確性を検証する。

-}

import Expect
import Test exposing (..)
import Util.DateFormat as DateFormat


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
        [ test "ISO 8601 日時文字列から日付部分を抽出する" <|
            \_ ->
                DateFormat.formatDate "2026-01-15T10:30:00Z"
                    |> Expect.equal "2026-01-15"
        , test "タイムゾーンオフセット付きでも日付部分を抽出する" <|
            \_ ->
                DateFormat.formatDate "2026-01-15T10:30:00+09:00"
                    |> Expect.equal "2026-01-15"
        , test "日付のみの文字列はそのまま返す" <|
            \_ ->
                DateFormat.formatDate "2026-01-15"
                    |> Expect.equal "2026-01-15"
        , test "10文字未満の文字列はそのまま返す" <|
            \_ ->
                DateFormat.formatDate "short"
                    |> Expect.equal "short"
        ]



-- formatMaybeDate


formatMaybeDateTests : Test
formatMaybeDateTests =
    describe "formatMaybeDate"
        [ test "Just の場合は日付部分を抽出する" <|
            \_ ->
                DateFormat.formatMaybeDate (Just "2026-01-15T10:30:00Z")
                    |> Expect.equal "2026-01-15"
        , test "Nothing の場合は \"-\" を返す" <|
            \_ ->
                DateFormat.formatMaybeDate Nothing
                    |> Expect.equal "-"
        ]



-- formatDateTime


formatDateTimeTests : Test
formatDateTimeTests =
    describe "formatDateTime"
        [ test "ISO 8601 日時文字列から日付と時刻を抽出する" <|
            \_ ->
                DateFormat.formatDateTime "2026-01-15T10:30:00Z"
                    |> Expect.equal "2026-01-15 10:30"
        , test "T を空白に置換する" <|
            \_ ->
                DateFormat.formatDateTime "2026-12-31T23:59:00Z"
                    |> Expect.equal "2026-12-31 23:59"
        , test "16文字未満の文字列は T を空白に置換して返す" <|
            \_ ->
                DateFormat.formatDateTime "2026-01-15"
                    |> Expect.equal "2026-01-15"
        ]



-- formatMaybeDateTime


formatMaybeDateTimeTests : Test
formatMaybeDateTimeTests =
    describe "formatMaybeDateTime"
        [ test "Just の場合は日付と時刻を抽出する" <|
            \_ ->
                DateFormat.formatMaybeDateTime (Just "2026-01-15T10:30:00Z")
                    |> Expect.equal "2026-01-15 10:30"
        , test "Nothing の場合は \"-\" を返す" <|
            \_ ->
                DateFormat.formatMaybeDateTime Nothing
                    |> Expect.equal "-"
        ]
