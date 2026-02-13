module Data.WorkflowDefinitionTest exposing (suite)

{-| Data.WorkflowDefinition モジュールのテスト
-}

import Data.WorkflowDefinition as WorkflowDefinition
import Expect
import Json.Decode as Decode
import Json.Encode as Encode
import Test exposing (..)


suite : Test
suite =
    describe "Data.WorkflowDefinition"
        [ decoderTests
        , listDecoderTests
        , approvalStepInfosTests
        ]



-- decoder


decoderTests : Test
decoderTests =
    describe "decoder"
        [ test "全フィールドをデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "def-001",
                            "name": "経費精算申請",
                            "description": "経費の精算を申請します",
                            "version": 1,
                            "definition": {"form": {"fields": []}},
                            "status": "active",
                            "created_by": "user-001",
                            "created_at": "2026-01-01T00:00:00Z",
                            "updated_at": "2026-01-01T00:00:00Z"
                        }
                        """
                in
                Decode.decodeString WorkflowDefinition.decoder json
                    |> Result.map
                        (\d ->
                            { id = d.id
                            , name = d.name
                            , description = d.description
                            , version = d.version
                            }
                        )
                    |> Expect.equal
                        (Ok
                            { id = "def-001"
                            , name = "経費精算申請"
                            , description = Just "経費の精算を申請します"
                            , version = 1
                            }
                        )
        , test "description が null の場合は Nothing" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "def-001",
                            "name": "休暇申請",
                            "description": null,
                            "version": 1,
                            "definition": {},
                            "status": "active",
                            "created_by": "user-001",
                            "created_at": "2026-01-01T00:00:00Z",
                            "updated_at": "2026-01-01T00:00:00Z"
                        }
                        """
                in
                Decode.decodeString WorkflowDefinition.decoder json
                    |> Result.map .description
                    |> Expect.equal (Ok Nothing)
        , test "description がない場合も Nothing" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "def-001",
                            "name": "休暇申請",
                            "version": 1,
                            "definition": {},
                            "status": "active",
                            "created_by": "user-001",
                            "created_at": "2026-01-01T00:00:00Z",
                            "updated_at": "2026-01-01T00:00:00Z"
                        }
                        """
                in
                Decode.decodeString WorkflowDefinition.decoder json
                    |> Result.map .description
                    |> Expect.equal (Ok Nothing)
        , test "必須フィールドがない場合はエラー" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "def-001"
                        }
                        """
                in
                Decode.decodeString WorkflowDefinition.decoder json
                    |> Expect.err
        ]



-- listDecoder


listDecoderTests : Test
listDecoderTests =
    describe "listDecoder"
        [ test "data フィールドから一覧をデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": [
                                {
                                    "id": "def-001",
                                    "name": "経費精算",
                                    "version": 1,
                                    "definition": {},
                                    "status": "active",
                                    "created_by": "user-001",
                                    "created_at": "2026-01-01T00:00:00Z",
                                    "updated_at": "2026-01-01T00:00:00Z"
                                },
                                {
                                    "id": "def-002",
                                    "name": "休暇申請",
                                    "version": 1,
                                    "definition": {},
                                    "status": "active",
                                    "created_by": "user-001",
                                    "created_at": "2026-01-01T00:00:00Z",
                                    "updated_at": "2026-01-01T00:00:00Z"
                                }
                            ]
                        }
                        """
                in
                Decode.decodeString WorkflowDefinition.listDecoder json
                    |> Result.map List.length
                    |> Expect.equal (Ok 2)
        , test "空の一覧をデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": []
                        }
                        """
                in
                Decode.decodeString WorkflowDefinition.listDecoder json
                    |> Expect.equal (Ok [])
        , test "data フィールドがない場合はエラー" <|
            \_ ->
                let
                    json =
                        """
                        []
                        """
                in
                Decode.decodeString WorkflowDefinition.listDecoder json
                    |> Expect.err
        ]



-- approvalStepInfos


approvalStepInfosTests : Test
approvalStepInfosTests =
    describe "approvalStepInfos"
        [ test "複数の承認ステップから id と name を抽出" <|
            \_ ->
                let
                    definition =
                        makeDefinition
                            (Encode.object
                                [ ( "steps"
                                  , Encode.list identity
                                        [ Encode.object
                                            [ ( "id", Encode.string "step-1" )
                                            , ( "name", Encode.string "部長承認" )
                                            , ( "type", Encode.string "approval" )
                                            ]
                                        , Encode.object
                                            [ ( "id", Encode.string "step-2" )
                                            , ( "name", Encode.string "経理承認" )
                                            , ( "type", Encode.string "approval" )
                                            ]
                                        ]
                                  )
                                ]
                            )
                in
                WorkflowDefinition.approvalStepInfos definition
                    |> List.map (\info -> ( info.id, info.name ))
                    |> Expect.equal
                        [ ( "step-1", "部長承認" )
                        , ( "step-2", "経理承認" )
                        ]
        , test "承認ステップが空の場合は空リスト" <|
            \_ ->
                let
                    definition =
                        makeDefinition
                            (Encode.object
                                [ ( "steps", Encode.list identity [] ) ]
                            )
                in
                WorkflowDefinition.approvalStepInfos definition
                    |> Expect.equal []
        , test "approval 以外のステップは除外" <|
            \_ ->
                let
                    definition =
                        makeDefinition
                            (Encode.object
                                [ ( "steps"
                                  , Encode.list identity
                                        [ Encode.object
                                            [ ( "id", Encode.string "step-1" )
                                            , ( "name", Encode.string "部長承認" )
                                            , ( "type", Encode.string "approval" )
                                            ]
                                        , Encode.object
                                            [ ( "id", Encode.string "step-notify" )
                                            , ( "name", Encode.string "通知" )
                                            , ( "type", Encode.string "notification" )
                                            ]
                                        ]
                                  )
                                ]
                            )
                in
                WorkflowDefinition.approvalStepInfos definition
                    |> List.map .id
                    |> Expect.equal [ "step-1" ]
        ]



-- Helpers


{-| テスト用の WorkflowDefinition を構築するヘルパー
-}
makeDefinition : Encode.Value -> WorkflowDefinition.WorkflowDefinition
makeDefinition def =
    { id = "def-001"
    , name = "テスト定義"
    , description = Nothing
    , version = 1
    , definition = def
    , status = "active"
    , createdBy = "user-001"
    , createdAt = "2026-01-01T00:00:00Z"
    , updatedAt = "2026-01-01T00:00:00Z"
    }
