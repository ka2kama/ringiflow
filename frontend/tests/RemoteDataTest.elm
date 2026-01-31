module RemoteDataTest exposing (suite)

{-| RemoteData モジュールのテスト

各ユーティリティ関数の振る舞いを検証する。

-}

import Expect
import RemoteData exposing (RemoteData(..))
import Test exposing (..)


suite : Test
suite =
    describe "RemoteData"
        [ mapTests
        , withDefaultTests
        , toMaybeTests
        , fromResultTests
        , isLoadingTests
        ]



-- map


mapTests : Test
mapTests =
    describe "map"
        [ test "NotAsked はそのまま" <|
            \_ ->
                RemoteData.map String.length NotAsked
                    |> Expect.equal NotAsked
        , test "Loading はそのまま" <|
            \_ ->
                RemoteData.map String.length Loading
                    |> Expect.equal Loading
        , test "Failure はそのまま" <|
            \_ ->
                RemoteData.map String.length (Failure "error")
                    |> Expect.equal (Failure "error")
        , test "Success は変換される" <|
            \_ ->
                RemoteData.map String.length (Success "hello")
                    |> Expect.equal (Success 5)
        ]



-- withDefault


withDefaultTests : Test
withDefaultTests =
    describe "withDefault"
        [ test "NotAsked はデフォルト値" <|
            \_ ->
                RemoteData.withDefault 0 NotAsked
                    |> Expect.equal 0
        , test "Loading はデフォルト値" <|
            \_ ->
                RemoteData.withDefault 0 Loading
                    |> Expect.equal 0
        , test "Failure はデフォルト値" <|
            \_ ->
                RemoteData.withDefault 0 (Failure "error")
                    |> Expect.equal 0
        , test "Success は中身を返す" <|
            \_ ->
                RemoteData.withDefault 0 (Success 42)
                    |> Expect.equal 42
        ]



-- toMaybe


toMaybeTests : Test
toMaybeTests =
    describe "toMaybe"
        [ test "NotAsked は Nothing" <|
            \_ ->
                RemoteData.toMaybe NotAsked
                    |> Expect.equal Nothing
        , test "Loading は Nothing" <|
            \_ ->
                RemoteData.toMaybe Loading
                    |> Expect.equal Nothing
        , test "Failure は Nothing" <|
            \_ ->
                RemoteData.toMaybe (Failure "error")
                    |> Expect.equal Nothing
        , test "Success は Just" <|
            \_ ->
                RemoteData.toMaybe (Success "value")
                    |> Expect.equal (Just "value")
        ]



-- fromResult


fromResultTests : Test
fromResultTests =
    describe "fromResult"
        [ test "Ok は Success" <|
            \_ ->
                RemoteData.fromResult (Ok 42)
                    |> Expect.equal (Success 42)
        , test "Err は Failure" <|
            \_ ->
                RemoteData.fromResult (Err "error")
                    |> Expect.equal (Failure "error")
        ]



-- isLoading


isLoadingTests : Test
isLoadingTests =
    describe "isLoading"
        [ test "NotAsked は False" <|
            \_ ->
                RemoteData.isLoading NotAsked
                    |> Expect.equal False
        , test "Loading は True" <|
            \_ ->
                RemoteData.isLoading Loading
                    |> Expect.equal True
        , test "Failure は False" <|
            \_ ->
                RemoteData.isLoading (Failure "error")
                    |> Expect.equal False
        , test "Success は False" <|
            \_ ->
                RemoteData.isLoading (Success 42)
                    |> Expect.equal False
        ]
