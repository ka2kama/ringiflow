module Data.WorkflowDefinitionStatusTest exposing (suite)

{-| Data.WorkflowDefinition のステータス関連ヘルパーのテスト
-}

import Data.WorkflowDefinition as WorkflowDefinition
    exposing
        ( WorkflowDefinitionStatus(..)
        )
import Expect
import Json.Decode as Decode
import Json.Encode as Encode
import Test exposing (..)


suite : Test
suite =
    describe "WorkflowDefinitionStatus"
        [ statusFromStringTests
        , statusToJapaneseTests
        , statusToBadgeTests
        , definitionStatusTests
        , encodeCreateRequestTests
        , encodeVersionRequestTests
        ]



-- statusFromString


statusFromStringTests : Test
statusFromStringTests =
    describe "statusFromString"
        [ test "\"draft\" → Draft" <|
            \_ ->
                WorkflowDefinition.statusFromString "draft"
                    |> Expect.equal Draft
        , test "\"published\" → Published" <|
            \_ ->
                WorkflowDefinition.statusFromString "published"
                    |> Expect.equal Published
        , test "\"archived\" → Archived" <|
            \_ ->
                WorkflowDefinition.statusFromString "archived"
                    |> Expect.equal Archived
        , test "不明な値は Draft にフォールバック" <|
            \_ ->
                WorkflowDefinition.statusFromString "unknown"
                    |> Expect.equal Draft
        ]



-- statusToJapanese


statusToJapaneseTests : Test
statusToJapaneseTests =
    describe "statusToJapanese"
        [ test "Draft → 下書き" <|
            \_ ->
                WorkflowDefinition.statusToJapanese Draft
                    |> Expect.equal "下書き"
        , test "Published → 公開済み" <|
            \_ ->
                WorkflowDefinition.statusToJapanese Published
                    |> Expect.equal "公開済み"
        , test "Archived → アーカイブ済み" <|
            \_ ->
                WorkflowDefinition.statusToJapanese Archived
                    |> Expect.equal "アーカイブ済み"
        ]



-- statusToBadge


statusToBadgeTests : Test
statusToBadgeTests =
    describe "statusToBadge"
        [ test "Draft は secondary カラーで「下書き」ラベル" <|
            \_ ->
                let
                    badge =
                        WorkflowDefinition.statusToBadge Draft
                in
                { colorClass = badge.colorClass, label = badge.label }
                    |> Expect.equal
                        { colorClass = "bg-secondary-100 text-secondary-600 border-secondary-200"
                        , label = "下書き"
                        }
        , test "Published は success カラーで「公開済み」ラベル" <|
            \_ ->
                let
                    badge =
                        WorkflowDefinition.statusToBadge Published
                in
                { colorClass = badge.colorClass, label = badge.label }
                    |> Expect.equal
                        { colorClass = "bg-success-50 text-success-600 border-success-200"
                        , label = "公開済み"
                        }
        , test "Archived は secondary カラーで「アーカイブ済み」ラベル" <|
            \_ ->
                let
                    badge =
                        WorkflowDefinition.statusToBadge Archived
                in
                { colorClass = badge.colorClass, label = badge.label }
                    |> Expect.equal
                        { colorClass = "bg-secondary-100 text-secondary-500 border-secondary-200"
                        , label = "アーカイブ済み"
                        }
        ]



-- definitionStatus


definitionStatusTests : Test
definitionStatusTests =
    describe "definitionStatus"
        [ test "status が \"draft\" の定義 → Draft" <|
            \_ ->
                makeDefinition "draft"
                    |> WorkflowDefinition.definitionStatus
                    |> Expect.equal Draft
        , test "status が \"published\" の定義 → Published" <|
            \_ ->
                makeDefinition "published"
                    |> WorkflowDefinition.definitionStatus
                    |> Expect.equal Published
        , test "status が \"archived\" の定義 → Archived" <|
            \_ ->
                makeDefinition "archived"
                    |> WorkflowDefinition.definitionStatus
                    |> Expect.equal Archived
        ]



-- encodeCreateRequest


encodeCreateRequestTests : Test
encodeCreateRequestTests =
    describe "encodeCreateRequest"
        [ test "name, description, definition を含む JSON を生成" <|
            \_ ->
                let
                    encoded =
                        WorkflowDefinition.encodeCreateRequest
                            { name = "経費精算申請"
                            , description = "経費の精算を申請します"
                            }

                    decodeName =
                        Decode.decodeValue (Decode.field "name" Decode.string) encoded

                    decodeDescription =
                        Decode.decodeValue (Decode.field "description" Decode.string) encoded

                    decodeDefinition =
                        Decode.decodeValue (Decode.field "definition" Decode.value) encoded
                in
                { name = decodeName
                , description = decodeDescription
                , hasDefinition = Result.toMaybe decodeDefinition /= Nothing
                }
                    |> Expect.equal
                        { name = Ok "経費精算申請"
                        , description = Ok "経費の精算を申請します"
                        , hasDefinition = True
                        }
        , test "definition に steps 配列と start ステップが含まれる" <|
            \_ ->
                let
                    encoded =
                        WorkflowDefinition.encodeCreateRequest
                            { name = "テスト"
                            , description = ""
                            }

                    stepsDecoder =
                        Decode.field "definition"
                            (Decode.field "steps" (Decode.list (Decode.field "type" Decode.string)))
                in
                Decode.decodeValue stepsDecoder encoded
                    |> Expect.equal (Ok [ "start" ])
        ]



-- encodeVersionRequest


encodeVersionRequestTests : Test
encodeVersionRequestTests =
    describe "encodeVersionRequest"
        [ test "version フィールドを含む JSON を生成" <|
            \_ ->
                let
                    encoded =
                        WorkflowDefinition.encodeVersionRequest { version = 3 }

                    decodeVersion =
                        Decode.decodeValue (Decode.field "version" Decode.int) encoded
                in
                decodeVersion
                    |> Expect.equal (Ok 3)
        ]



-- Helpers


{-| テスト用の WorkflowDefinition を構築するヘルパー
-}
makeDefinition : String -> WorkflowDefinition.WorkflowDefinition
makeDefinition status =
    { id = "def-001"
    , name = "テスト定義"
    , description = Nothing
    , version = 1
    , definition = Encode.object []
    , status = status
    , createdBy = "user-001"
    , createdAt = "2026-01-01T00:00:00Z"
    , updatedAt = "2026-01-01T00:00:00Z"
    }
